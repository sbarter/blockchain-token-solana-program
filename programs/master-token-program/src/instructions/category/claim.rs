use crate::{states::category::*, SBT_DECIMALS, VESTING_MONTH};
use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::{get_associated_token_address_with_program_id, AssociatedToken},
    token_2022::{self, Token2022, TransferChecked},
    token_interface::{Mint, TokenAccount},
};

#[allow(clippy::too_many_arguments)]
fn update_vesting_for_investor_category<'info>(
    now: u64,
    category: &mut Account<'info, InvestorCategoryData>,
    category_ata: AccountInfo<'info>,
    master_pda: &AccountInfo<'info>,
    master_ata: &InterfaceAccount<'info, TokenAccount>,
    pda_seeds: &[&[&[u8]]],
    mint: &InterfaceAccount<'info, Mint>,
    token_program: &Program<'info, Token2022>,
) -> Result<()> {
    require_keys_eq!(
        category_ata.key(),
        get_associated_token_address_with_program_id(
            &category.to_account_info().key(),
            &mint.key(),
            &token_2022::ID,
        ),
        crate::error::ErrorCode::AtaMismatch,
    );

    require!(
        category.cliff_started_at != 0,
        crate::error::ErrorCode::TgeNotHappened
    );

    let since_tge = now.saturating_sub(category.cliff_started_at);
    let months_elapsed = since_tge / VESTING_MONTH;
    let total_months = months_elapsed
        .saturating_sub(category.months_claimed as u64)
        .min(48) as u8;

    msg!("Months since TGE:");
    msg!(&months_elapsed.to_string());
    msg!("Claiming for months:");
    msg!(&total_months.to_string());

    if total_months == 0 {
        msg!("No claim available.");
        return Ok(());
    }

    let cliff_months_claimed = total_months.min(category.cliff_months_remaining);
    let vesting_months_claimed =
        (total_months - cliff_months_claimed).min(category.vesting_months_remaining);
    let total_tokens = category.monthly_allocation * vesting_months_claimed as u64;

    if total_tokens > 0 {
        let cpi_accounts = TransferChecked {
            from: master_ata.to_account_info(),
            to: category_ata,
            authority: master_pda.to_account_info(),
            mint: mint.to_account_info(),
        };
        let cpi_ctx =
            CpiContext::new_with_signer(token_program.to_account_info(), cpi_accounts, pda_seeds);
        token_2022::transfer_checked(cpi_ctx, total_tokens, SBT_DECIMALS)?;
    }
    category.cliff_months_remaining -= cliff_months_claimed;
    category.vesting_months_remaining -= vesting_months_claimed;
    category.months_claimed = category.months_claimed.saturating_add(total_months);
    category.tokens_ready_for_claim +=
        category.total_allocated_tokens_monthly * vesting_months_claimed as u64;

    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn update_vesting_for_functional_category<'info>(
    now: u64,
    category: &mut Account<'info, FunctionalCategoryData>,
    category_ata: AccountInfo<'info>,
    master_pda: &AccountInfo<'info>,
    master_ata: &InterfaceAccount<'info, TokenAccount>,
    pda_seeds: &[&[&[u8]]],
    mint: &InterfaceAccount<'info, Mint>,
    token_program: &Program<'info, Token2022>,
) -> Result<()> {
    require_keys_eq!(
        category_ata.key(),
        get_associated_token_address_with_program_id(
            &category.wallet,
            &mint.key(),
            &token_2022::ID,
        ),
        crate::error::ErrorCode::AtaMismatch,
    );

    require!(
        category.cliff_started_at != 0,
        crate::error::ErrorCode::TgeNotHappened
    );

    let since_tge = now.saturating_sub(category.cliff_started_at);
    let months_elapsed = since_tge / VESTING_MONTH;
    let total_months = months_elapsed
        .saturating_sub(category.months_claimed as u64)
        .min(48u8.saturating_sub(category.months_claimed) as u64) as u8;

    msg!("Months since TGE:");
    msg!(&months_elapsed.to_string());
    msg!("Claiming for months:");
    msg!(&total_months.to_string());

    if total_months == 0 {
        msg!("No claim available.");
        return Ok(());
    }

    let cliff_months_claimed = total_months.min(category.cliff_months_remaining);
    let vesting_months_claimed =
        (total_months - cliff_months_claimed).min(category.vesting_months_remaining);
    let total_tokens = category.monthly_allocation_in_base_units * vesting_months_claimed as u64;

    if total_tokens > 0 {
        let cpi_accounts = TransferChecked {
            from: master_ata.to_account_info(),
            to: category_ata,
            authority: master_pda.to_account_info(),
            mint: mint.to_account_info(),
        };
        let cpi_ctx =
            CpiContext::new_with_signer(token_program.to_account_info(), cpi_accounts, pda_seeds);
        token_2022::transfer_checked(cpi_ctx, total_tokens, SBT_DECIMALS)?;
    }
    category.cliff_months_remaining -= cliff_months_claimed;
    category.vesting_months_remaining -= vesting_months_claimed;
    category.months_claimed = category.months_claimed.saturating_add(total_months);

    Ok(())
}

pub fn transfer_category_vestings<'info>(
    ctx: Context<'_, '_, '_, 'info, TransferCategoryVestings<'info>>,
) -> Result<()> {
    let now = Clock::get()?.unix_timestamp as u64;
    let master_ata = &ctx.accounts.master_ata;
    let master_pda = &ctx.accounts.master_pda;
    let mint = &ctx.accounts.mint;
    let token_program = &ctx.accounts.token_program;
    let master_seeds = &[b"master".as_ref(), &[ctx.bumps.master_pda]];
    let signer_seeds = &[&master_seeds[..]];

    if update_vesting_for_investor_category(
        now,
        &mut ctx.accounts.pre_seed_cat,
        ctx.accounts.pre_seed_ata.to_account_info(),
        master_pda,
        master_ata,
        signer_seeds,
        mint,
        token_program,
    )
    .is_err()
    {
        msg!("Failed to transfer tokens from master to pre-seed!");
    }
    if update_vesting_for_investor_category(
        now,
        &mut ctx.accounts.seed_cat,
        ctx.accounts.seed_ata.to_account_info(),
        master_pda,
        master_ata,
        signer_seeds,
        mint,
        token_program,
    )
    .is_err()
    {
        msg!("Failed to transfer tokens from master to seed!");
    }
    if update_vesting_for_investor_category(
        now,
        &mut ctx.accounts.institutional_cat,
        ctx.accounts.institutional_ata.to_account_info(),
        master_pda,
        master_ata,
        signer_seeds,
        mint,
        token_program,
    )
    .is_err()
    {
        msg!("Failed to transfer tokens from master to institutional!");
    }
    if update_vesting_for_investor_category(
        now,
        &mut ctx.accounts.vgp_cat,
        ctx.accounts.vgp_ata.to_account_info(),
        master_pda,
        master_ata,
        signer_seeds,
        mint,
        token_program,
    )
    .is_err()
    {
        msg!("Failed to transfer tokens from master to VGP!");
    }
    if update_vesting_for_investor_category(
        now,
        &mut ctx.accounts.founders_cat,
        ctx.accounts.founders_ata.to_account_info(),
        master_pda,
        master_ata,
        signer_seeds,
        mint,
        token_program,
    )
    .is_err()
    {
        msg!("Failed to transfer tokens from master to founders!");
    }
    if update_vesting_for_functional_category(
        now,
        &mut ctx.accounts.marketing_cat,
        ctx.accounts.marketing_ata.to_account_info(),
        master_pda,
        master_ata,
        signer_seeds,
        mint,
        token_program,
    )
    .is_err()
    {
        msg!("Failed to transfer tokens from master to marketing!");
    }
    if update_vesting_for_functional_category(
        now,
        &mut ctx.accounts.reserve_cat,
        ctx.accounts.reserve_ata.to_account_info(),
        master_pda,
        master_ata,
        signer_seeds,
        mint,
        token_program,
    )
    .is_err()
    {
        msg!("Failed to transfer tokens from master to reserve!");
    }
    if update_vesting_for_functional_category(
        now,
        &mut ctx.accounts.liquidity_cat,
        ctx.accounts.liquidity_ata.to_account_info(),
        master_pda,
        master_ata,
        signer_seeds,
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
        seeds = [b"master"],
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
        seeds = [PRE_SEED_CATEGORY.seed, mint.key().as_ref()],
        bump
    )]
    pub pre_seed_cat: Account<'info, InvestorCategoryData>,
    /// CHECK: created by Initialize
    #[account(mut)]
    pub pre_seed_ata: UncheckedAccount<'info>,

    #[account(
        mut,
        seeds = [SEED_CATEGORY.seed, mint.key().as_ref()],
        bump
    )]
    pub seed_cat: Account<'info, InvestorCategoryData>,
    /// CHECK: created by Initialize
    #[account(mut)]
    pub seed_ata: UncheckedAccount<'info>,

    #[account(
        mut,
        seeds = [INSTITUTIONAL_CATEGORY.seed, mint.key().as_ref()],
        bump
    )]
    pub institutional_cat: Account<'info, InvestorCategoryData>,
    /// CHECK: created by Initialize
    #[account(mut)]
    pub institutional_ata: UncheckedAccount<'info>,

    #[account(
        mut,
        seeds = [VGP_CATEGORY.seed, mint.key().as_ref()],
        bump
    )]
    pub vgp_cat: Account<'info, InvestorCategoryData>,
    /// CHECK: created by Initialize
    #[account(mut)]
    pub vgp_ata: UncheckedAccount<'info>,

    #[account(
        mut,
        seeds = [FOUNDERS_CATEGORY.seed, mint.key().as_ref()],
        bump
    )]
    pub founders_cat: Account<'info, InvestorCategoryData>,
    /// CHECK: created by Initialize
    #[account(mut)]
    pub founders_ata: UncheckedAccount<'info>,

    #[account(
        mut,
        seeds = [MARKETING_CATEGORY.seed, mint.key().as_ref()],
        bump
    )]
    pub marketing_cat: Account<'info, FunctionalCategoryData>,
    /// CHECK: created by Initialize
    #[account(mut)]
    pub marketing_ata: UncheckedAccount<'info>,

    #[account(
        mut,
        seeds = [RESERVE_CATEGORY.seed, mint.key().as_ref()],
        bump
    )]
    pub reserve_cat: Account<'info, FunctionalCategoryData>,
    /// CHECK: created by Initialize
    #[account(mut)]
    pub reserve_ata: UncheckedAccount<'info>,

    #[account(
        mut,
        seeds = [LIQUIDITY_CATEGORY.seed, mint.key().as_ref()],
        bump
    )]
    pub liquidity_cat: Account<'info, FunctionalCategoryData>,
    /// CHECK: created by Initialize
    #[account(mut)]
    pub liquidity_ata: UncheckedAccount<'info>,

    pub mint: Box<InterfaceAccount<'info, Mint>>,
    pub token_program: Program<'info, Token2022>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
}
