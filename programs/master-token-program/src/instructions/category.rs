use crate::{states::category::*, SBT_DECIMALS, VESTING_MONTH};
use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::{get_associated_token_address_with_program_id, AssociatedToken},
    token_2022::{self, Token2022, TransferChecked},
    token_interface::{Mint, TokenAccount},
};

fn update_vesting_for_investor_category<'info>(
    category: &mut Account<'info, InvestorCategoryData>,
    category_ata: AccountInfo<'info>,
    master_pda: &AccountInfo<'info>,
    master_ata: &InterfaceAccount<'info, TokenAccount>,
    mint: &InterfaceAccount<'info, Mint>,
    token_program: &Program<'info, Token2022>,
) -> Result<()> {
    require_keys_eq!(
        category_ata.key(),
        get_associated_token_address_with_program_id(
            &category.to_account_info().key(),
            &mint.key(),
            &token_2022::ID,
        )
    );

    let cliff_pre = category.cliff_months_remaining;
    let vesting_pre = category.vesting_months_remaining;

    let now = Clock::get()?.unix_timestamp as u64;
    require!(
        category.cliff_started_at != 0,
        crate::error::ErrorCode::TgeNotHappened
    );

    let since_tge = now.saturating_sub(category.cliff_started_at);
    let months_elapsed = (since_tge / VESTING_MONTH) as u8;
    let total_months = months_elapsed.saturating_sub(category.months_claimed);

    if total_months == 0 {
        msg!("No claim available.");
        return Ok(());
    }

    let mut total_tokens = 0;
    for _ in 0..total_months {
        if category.vesting_months_remaining == 0 {
            break;
        }
        if category.cliff_months_remaining > 0 {
            category.cliff_months_remaining -= 1;
            continue;
        }
        if category.cliff_months_remaining == 0 && category.vesting_months_remaining > 0 {
            total_tokens += category.monthly_allocation;
            category.vesting_months_remaining -= 1;
            continue;
        }
    }
    if total_tokens > 0 {
        let cpi_accounts = TransferChecked {
            from: master_ata.to_account_info(),
            to: category_ata,
            authority: master_pda.to_account_info(),
            mint: mint.to_account_info(),
        };
        let cpi_ctx = CpiContext::new(token_program.to_account_info(), cpi_accounts);
        let transfer = token_2022::transfer_checked(cpi_ctx, total_tokens, SBT_DECIMALS as u8);
        if transfer.is_err() {
            msg!("Unable to transfer tokens from to category. This really shouldn't happen.");
            category.cliff_months_remaining = cliff_pre;
            category.vesting_months_remaining = vesting_pre;
            transfer?;
        }
    }
    category.months_claimed += total_months;

    Ok(())
}

fn update_vesting_for_functional_category<'info>(
    category: &mut Account<'info, FunctionalCategoryData>,
    category_ata: AccountInfo<'info>,
    master_pda: &AccountInfo<'info>,
    master_ata: &InterfaceAccount<'info, TokenAccount>,
    mint: &InterfaceAccount<'info, Mint>,
    token_program: &Program<'info, Token2022>,
) -> Result<()> {
    require_keys_eq!(
        category_ata.key(),
        get_associated_token_address_with_program_id(
            &category.to_account_info().key(),
            &mint.key(),
            &token_2022::ID,
        )
    );

    let cliff_pre = category.cliff_months_remaining;
    let vesting_pre = category.vesting_months_remaining;

    let now = Clock::get()?.unix_timestamp as u64;
    require!(
        category.cliff_started_at != 0,
        crate::error::ErrorCode::TgeNotHappened
    );

    let since_tge = now.saturating_sub(category.cliff_started_at);
    let months_elapsed = (since_tge / VESTING_MONTH) as u8;
    let total_months = months_elapsed.saturating_sub(category.months_claimed);

    if total_months == 0 {
        msg!("No claim available.");
        return Ok(());
    }

    let mut total_tokens = 0;
    for _ in 0..total_months {
        if category.vesting_months_remaining == 0 {
            break;
        }
        if category.cliff_months_remaining > 0 {
            category.cliff_months_remaining -= 1;
            continue;
        }
        if category.cliff_months_remaining == 0 && category.vesting_months_remaining > 0 {
            total_tokens += category.monthly_allocation;
            category.vesting_months_remaining -= 1;
        }
    }

    if total_tokens > 0 {
        let cpi_accounts = TransferChecked {
            from: master_ata.to_account_info(),
            to: category_ata,
            authority: master_pda.to_account_info(),
            mint: mint.to_account_info(),
        };
        let cpi_ctx = CpiContext::new(token_program.to_account_info(), cpi_accounts);
        let transfer = token_2022::transfer_checked(cpi_ctx, total_tokens, SBT_DECIMALS as u8);
        if transfer.is_err() {
            msg!("Unable to transfer tokens to the category. This really shouldn't happen.");
            category.cliff_months_remaining = cliff_pre;
            category.vesting_months_remaining = vesting_pre;
            transfer?;
        }
    }
    category.months_claimed += total_months;

    Ok(())
}

pub fn transfer_category_vestings<'info>(
    ctx: Context<'_, '_, '_, 'info, TransferCategoryVestings<'info>>,
) -> Result<()> {
    let master_ata = &ctx.accounts.master_ata;
    let master_pda = &ctx.accounts.master_pda;
    let mint = &ctx.accounts.mint;
    let token_program = &ctx.accounts.token_program;

    if update_vesting_for_investor_category(
        &mut ctx.accounts.pre_seed_cat,
        ctx.accounts.pre_seed_ata.to_account_info(),
        master_pda,
        master_ata,
        mint,
        token_program,
    )
    .is_err()
    {
        msg!("Failed to transfer tokens from master to pre-seed!");
    }
    if update_vesting_for_investor_category(
        &mut ctx.accounts.seed_cat,
        ctx.accounts.seed_ata.to_account_info(),
        master_pda,
        master_ata,
        mint,
        token_program,
    )
    .is_err()
    {
        msg!("Failed to transfer tokens from master to seed!");
    }
    if update_vesting_for_investor_category(
        &mut ctx.accounts.institutional_cat,
        ctx.accounts.institutional_ata.to_account_info(),
        master_pda,
        master_ata,
        mint,
        token_program,
    )
    .is_err()
    {
        msg!("Failed to transfer tokens from master to institutional!");
    }
    if update_vesting_for_investor_category(
        &mut ctx.accounts.vgp_cat,
        ctx.accounts.vgp_ata.to_account_info(),
        master_pda,
        master_ata,
        mint,
        token_program,
    )
    .is_err()
    {
        msg!("Failed to transfer tokens from master to VGP!");
    }
    if update_vesting_for_investor_category(
        &mut ctx.accounts.founders_cat,
        ctx.accounts.founders_ata.to_account_info(),
        master_pda,
        master_ata,
        mint,
        token_program,
    )
    .is_err()
    {
        msg!("Failed to transfer tokens from master to founders!");
    }
    if update_vesting_for_functional_category(
        &mut ctx.accounts.marketing_cat,
        ctx.accounts.marketing_ata.to_account_info(),
        master_pda,
        master_ata,
        mint,
        token_program,
    )
    .is_err()
    {
        msg!("Failed to transfer tokens from master to marketing!");
    }
    if update_vesting_for_functional_category(
        &mut ctx.accounts.reserve_cat,
        ctx.accounts.reserve_ata.to_account_info(),
        master_pda,
        master_ata,
        mint,
        token_program,
    )
    .is_err()
    {
        msg!("Failed to transfer tokens from master to reserve!");
    }
    if update_vesting_for_functional_category(
        &mut ctx.accounts.liquidity_cat,
        ctx.accounts.liquidity_ata.to_account_info(),
        master_pda,
        master_ata,
        mint,
        token_program,
    )
    .is_err()
    {
        msg!("Failed to transfer tokens from master to liquidity!");
    }
    Ok(())
}

#[derive(Accounts)]
pub struct TransferCategoryVestings<'info> {
    #[account(
        mut,
        seeds = [b"master", mint.key().as_ref()],
        bump
    )]
    /// CHECK: pda authority
    pub master_pda: AccountInfo<'info>,

    #[account(
        mut,
        associated_token::mint = mint,
        associated_token::authority = master_pda,
        associated_token::token_program = token_program
    )]
    /// CHECK: created by Initialize
    pub master_ata: InterfaceAccount<'info, TokenAccount>,

    #[account(
        mut,
        seeds = [PRE_SEED_CATEGORY.0, mint.key().as_ref()],
        bump
    )]
    pub pre_seed_cat: Account<'info, InvestorCategoryData>,
    /// CHECK: created by Initialize
    #[account(mut)]
    pub pre_seed_ata: UncheckedAccount<'info>,

    #[account(
        mut,
        seeds = [SEED_CATEGORY.0, mint.key().as_ref()],
        bump
    )]
    pub seed_cat: Account<'info, InvestorCategoryData>,
    /// CHECK: created by Initialize
    #[account(mut)]
    pub seed_ata: UncheckedAccount<'info>,

    #[account(
        mut,
        seeds = [INSTITUTIONAL_CATEGORY.0, mint.key().as_ref()],
        bump
    )]
    pub institutional_cat: Account<'info, InvestorCategoryData>,
    /// CHECK: created by Initialize
    #[account(mut)]
    pub institutional_ata: UncheckedAccount<'info>,

    #[account(
        mut,
        seeds = [VGP_CATEGORY.0, mint.key().as_ref()],
        bump
    )]
    pub vgp_cat: Account<'info, InvestorCategoryData>,
    /// CHECK: created by Initialize
    #[account(mut)]
    pub vgp_ata: UncheckedAccount<'info>,

    #[account(
        mut,
        seeds = [FOUNDERS_CATEGORY.0, mint.key().as_ref()],
        bump
    )]
    pub founders_cat: Account<'info, InvestorCategoryData>,
    /// CHECK: created by Initialize
    #[account(mut)]
    pub founders_ata: UncheckedAccount<'info>,

    #[account(
        mut,
        seeds = [MARKETING_CATEGORY.0, mint.key().as_ref()],
        bump
    )]
    pub marketing_cat: Account<'info, FunctionalCategoryData>,
    /// CHECK: created by Initialize
    #[account(mut)]
    pub marketing_ata: UncheckedAccount<'info>,

    #[account(
        mut,
        seeds = [RESERVE_CATEGORY.0, mint.key().as_ref()],
        bump
    )]
    pub reserve_cat: Account<'info, FunctionalCategoryData>,
    /// CHECK: created by Initialize
    #[account(mut)]
    pub reserve_ata: UncheckedAccount<'info>,

    #[account(
        mut,
        seeds = [LIQUIDITY_CATEGORY.0, mint.key().as_ref()],
        bump
    )]
    pub liquidity_cat: Account<'info, FunctionalCategoryData>,
    /// CHECK: created by Initialize
    #[account(mut)]
    pub liquidity_ata: UncheckedAccount<'info>,

    #[account(mut)]
    pub mint: Box<InterfaceAccount<'info, Mint>>,
    pub token_program: Program<'info, Token2022>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
}
