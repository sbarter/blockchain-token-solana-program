use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::{self, get_associated_token_address_with_program_id, AssociatedToken},
    token_2022::{self, Token2022},
    token_interface::Mint,
};

use crate::category::*;

#[allow(clippy::too_many_arguments)]
fn initialize_category<'info>(
    category: &mut Account<'info, CategoryData>,
    category_ata: &UncheckedAccount<'info>,
    data: CategoryData,
    master: &Signer<'info>,
    mint: &InterfaceAccount<'info, Mint>,
    token_program: &Program<'info, Token2022>,
    associated_token_program: &Program<'info, AssociatedToken>,
    system_program: &Program<'info, System>,
) -> Result<()> {
    category.monthly_allocation = data.monthly_allocation;
    category.cliff_months_remaining = data.cliff_months_remaining;
    category.vesting_months_remaining = data.vesting_months_remaining;
    category.investor_count = data.investor_count;
    category.is_open = data.is_open;

    let expected_category_ata =
        get_associated_token_address_with_program_id(&category.key(), &mint.key(), &token_2022::ID);
    require_keys_eq!(category_ata.key(), expected_category_ata);

    let cpi_accounts = associated_token::Create {
        payer: master.to_account_info(),
        associated_token: category_ata.as_ref().clone(),
        authority: category.to_account_info(),
        mint: mint.to_account_info(),
        system_program: system_program.to_account_info(),
        token_program: token_program.to_account_info(),
    };

    let cpi_prog = associated_token_program.to_account_info();
    let cpi_ctx = CpiContext::new(cpi_prog, cpi_accounts);
    associated_token::create(cpi_ctx)?;
    Ok(())
}

pub fn initialize<'info>(ctx: Context<'_, '_, '_, 'info, Initialize<'info>>) -> Result<()> {
    let mut failed = false;

    let expected_master_ata = get_associated_token_address_with_program_id(
        &ctx.accounts.master.key(),
        &ctx.accounts.mint.key(),
        &token_2022::ID,
    );
    require_keys_eq!(ctx.accounts.master_ata.key(), expected_master_ata);

    let cpi_accounts = associated_token::Create {
        payer: ctx.accounts.master.to_account_info(),
        associated_token: ctx.accounts.master_ata.as_ref().clone(),
        authority: ctx.accounts.master.to_account_info(),
        mint: ctx.accounts.mint.to_account_info(),
        system_program: ctx.accounts.system_program.to_account_info(),
        token_program: ctx.accounts.token_program.to_account_info(),
    };

    let cpi_prog = ctx.accounts.associated_token_program.to_account_info();
    let cpi_ctx = CpiContext::new(cpi_prog, cpi_accounts);
    if associated_token::create(cpi_ctx).is_err() {
        failed = true;
        msg!("Failed to initialize master ATA!");
    }

    let master = &ctx.accounts.master;
    let mint = &ctx.accounts.mint;
    let system_program = &ctx.accounts.system_program;
    let associated_token_program = &ctx.accounts.associated_token_program;
    let token_program = &ctx.accounts.token_program;

    if initialize_category(
        &mut ctx.accounts.pre_seed_cat,
        &ctx.accounts.pre_seed_ata,
        PRE_SEED_CATEGORY.1,
        master,
        mint,
        token_program,
        associated_token_program,
        system_program,
    )
    .is_err()
    {
        failed = true;
        msg!("Failed to initialize category for pre-seed!");
    }
    if initialize_category(
        &mut ctx.accounts.seed_cat,
        &ctx.accounts.seed_ata,
        SEED_CATEGORY.1,
        master,
        mint,
        token_program,
        associated_token_program,
        system_program,
    )
    .is_err()
    {
        failed = true;
        msg!("Failed to initialize category for seed!");
    }
    if initialize_category(
        &mut ctx.accounts.institutional_cat,
        &ctx.accounts.institutional_ata,
        INSTITUTIONAL_CATEGORY.1,
        master,
        mint,
        token_program,
        associated_token_program,
        system_program,
    )
    .is_err()
    {
        failed = true;
        msg!("Failed to initialize category for institutional!");
    }
    if initialize_category(
        &mut ctx.accounts.vgp_cat,
        &ctx.accounts.vgp_ata,
        VGP_CATEGORY.1,
        master,
        mint,
        token_program,
        associated_token_program,
        system_program,
    )
    .is_err()
    {
        failed = true;
        msg!("Failed to initialize category for vgp!");
    }
    if initialize_category(
        &mut ctx.accounts.marketing_cat,
        &ctx.accounts.marketing_ata,
        MARKETING_CATEGORY.1,
        master,
        mint,
        token_program,
        associated_token_program,
        system_program,
    )
    .is_err()
    {
        failed = true;
        msg!("Failed to initialize category for marketing!");
    }
    if initialize_category(
        &mut ctx.accounts.founders_cat,
        &ctx.accounts.founders_ata,
        FOUNDERS_CATEGORY.1,
        master,
        mint,
        token_program,
        associated_token_program,
        system_program,
    )
    .is_err()
    {
        failed = true;
        msg!("Failed to initialize category for founders!");
    }
    if initialize_category(
        &mut ctx.accounts.reserve_cat,
        &ctx.accounts.reserve_ata,
        RESERVE_CATEGORY.1,
        master,
        mint,
        token_program,
        associated_token_program,
        system_program,
    )
    .is_err()
    {
        failed = true;
        msg!("Failed to initialize category for reserve!");
    }
    if initialize_category(
        &mut ctx.accounts.liquidity_cat,
        &ctx.accounts.liquidity_ata,
        LIQUIDITY_CATEGORY.1,
        master,
        mint,
        token_program,
        associated_token_program,
        system_program,
    )
    .is_err()
    {
        failed = true;
        msg!("Failed to initialize category for liquidity!");
    }
    if failed {
        Err(crate::error::ErrorCode::InitError.into())
    } else {
        Ok(())
    }
}

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(mut, signer, address = crate::MASTER_WALLET)]
    pub master: Signer<'info>,

    #[account(mut)]
    /// CHECK: will be initialized
    pub master_ata: UncheckedAccount<'info>,

    #[account(
        init,
        payer = master,
        space = CategoryData::LEN,
        seeds = [PRE_SEED_CATEGORY.0, mint.key().as_ref()],
        bump
    )]
    pub pre_seed_cat: Box<Account<'info, CategoryData>>,
    /// CHECK: will be initialized
    #[account(mut)]
    pub pre_seed_ata: UncheckedAccount<'info>,

    #[account(
        init,
        payer = master,
        space = CategoryData::LEN,
        seeds = [SEED_CATEGORY.0, mint.key().as_ref()],
        bump
    )]
    pub seed_cat: Box<Account<'info, CategoryData>>,
    #[account(mut)]
    /// CHECK: will be initialized
    pub seed_ata: UncheckedAccount<'info>,

    #[account(
        init,
        payer = master,
        space = CategoryData::LEN,
        seeds = [INSTITUTIONAL_CATEGORY.0, mint.key().as_ref()],
        bump
    )]
    pub institutional_cat: Box<Account<'info, CategoryData>>,
    #[account(mut)]
    /// CHECK: will be initialized
    pub institutional_ata: UncheckedAccount<'info>,

    #[account(
        init,
        payer = master,
        space = CategoryData::LEN,
        seeds = [VGP_CATEGORY.0, mint.key().as_ref()],
        bump
    )]
    pub vgp_cat: Box<Account<'info, CategoryData>>,
    #[account(mut)]
    /// CHECK: will be initialized
    pub vgp_ata: UncheckedAccount<'info>,

    #[account(
        init,
        payer = master,
        space = CategoryData::LEN,
        seeds = [MARKETING_CATEGORY.0, mint.key().as_ref()],
        bump
    )]
    pub marketing_cat: Box<Account<'info, CategoryData>>,
    #[account(mut)]
    /// CHECK: will be initialized
    pub marketing_ata: UncheckedAccount<'info>,

    #[account(
        init,
        payer = master,
        space = CategoryData::LEN,
        seeds = [FOUNDERS_CATEGORY.0, mint.key().as_ref()],
        bump
    )]
    pub founders_cat: Box<Account<'info, CategoryData>>,
    #[account(mut)]
    /// CHECK: will be initialized
    pub founders_ata: UncheckedAccount<'info>,

    #[account(
        init,
        payer = master,
        space = CategoryData::LEN,
        seeds = [RESERVE_CATEGORY.0, mint.key().as_ref()],
        bump
    )]
    pub reserve_cat: Box<Account<'info, CategoryData>>,
    #[account(mut)]
    /// CHECK: will be initialized
    pub reserve_ata: UncheckedAccount<'info>,

    #[account(
        init,
        payer = master,
        space = CategoryData::LEN,
        seeds = [LIQUIDITY_CATEGORY.0, mint.key().as_ref()],
        bump
    )]
    pub liquidity_cat: Box<Account<'info, CategoryData>>,
    #[account(mut)]
    /// CHECK: will be initialized
    pub liquidity_ata: UncheckedAccount<'info>,

    #[account(mut, mint::authority = master)]
    pub mint: Box<InterfaceAccount<'info, Mint>>,
    pub token_program: Program<'info, Token2022>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
}
