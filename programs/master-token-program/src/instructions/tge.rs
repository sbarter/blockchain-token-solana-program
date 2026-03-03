use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::{get_associated_token_address_with_program_id, AssociatedToken},
    token_2022::{
        self, spl_token_2022::instruction::AuthorityType, MintTo, SetAuthority, Token2022,
        TransferChecked,
    },
    token_interface::{Mint, TokenAccount},
};

use crate::{
    states::{
        category::{
            FunctionalCategoryData, LIQUIDITY_CATEGORY, MARKETING_CATEGORY, RESERVE_CATEGORY,
        },
        InvestorCategoryData, FOUNDERS_CATEGORY, INSTITUTIONAL_CATEGORY, PRE_SEED_CATEGORY,
        SEED_CATEGORY, VGP_CATEGORY,
    },
    LIQUIDITY_LIQUID_SUPPLY, MARKETING_LIQUID_SUPPLY, RESERVE_LIQUID_SUPPLY, RESERVE_PADDING,
    SBT_DECIMALS, TOTAL_MINT_SUPPLY,
};

fn has_enough_investors<'info>(
    category: &Account<'info, InvestorCategoryData>,
    target_count: u16,
) -> bool {
    !category.is_open && category.investor_count == target_count
}

pub fn start_tge<'info>(ctx: Context<'_, '_, '_, 'info, Tge<'info>>) -> Result<()> {
    require_keys_eq!(
        ctx.accounts.master_ata.key(),
        get_associated_token_address_with_program_id(
            &ctx.accounts.master_pda.key(),
            &ctx.accounts.mint.key(),
            &token_2022::ID,
        ),
        crate::error::ErrorCode::AtaMismatch,
    );
    require_keys_eq!(
        ctx.accounts.marketing_ata.key(),
        get_associated_token_address_with_program_id(
            &ctx.accounts.marketing_cat.wallet,
            &ctx.accounts.mint.key(),
            &token_2022::ID,
        ),
        crate::error::ErrorCode::AtaMismatch,
    );
    require_keys_eq!(
        ctx.accounts.liquidity_ata.key(),
        get_associated_token_address_with_program_id(
            &ctx.accounts.liquidity_cat.wallet,
            &ctx.accounts.mint.key(),
            &token_2022::ID,
        ),
        crate::error::ErrorCode::AtaMismatch,
    );
    require_keys_eq!(
        ctx.accounts.reserve_ata.key(),
        get_associated_token_address_with_program_id(
            &ctx.accounts.reserve_cat.wallet,
            &ctx.accounts.mint.key(),
            &token_2022::ID,
        ),
        crate::error::ErrorCode::AtaMismatch,
    );

    require!(
        has_enough_investors(&ctx.accounts.pre_seed_cat, PRE_SEED_CATEGORY.pre_investors),
        crate::error::ErrorCode::UnintializedInvestors
    );
    require!(
        has_enough_investors(&ctx.accounts.seed_cat, SEED_CATEGORY.pre_investors),
        crate::error::ErrorCode::UnintializedInvestors
    );

    let master_seeds = &[b"master".as_ref(), &[ctx.bumps.master_pda]];
    let signer_seeds = &[&master_seeds[..]];

    let cpi_accounts = MintTo {
        mint: ctx.accounts.mint.to_account_info(),
        to: ctx.accounts.master_ata.to_account_info(),
        authority: ctx.accounts.master_pda.to_account_info(),
    };
    token_2022::mint_to(
        CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            cpi_accounts,
            signer_seeds,
        ),
        TOTAL_MINT_SUPPLY,
    )?;

    let cpi_accounts_set = SetAuthority {
        account_or_mint: ctx.accounts.mint.to_account_info(),
        current_authority: ctx.accounts.master_pda.to_account_info(),
    };
    token_2022::set_authority(
        CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            cpi_accounts_set,
            signer_seeds,
        ),
        AuthorityType::MintTokens,
        None,
    )?;

    let now = Clock::get()?.unix_timestamp as u64;
    ctx.accounts.pre_seed_cat.cliff_started_at = now;
    ctx.accounts.seed_cat.cliff_started_at = now;
    ctx.accounts.institutional_cat.cliff_started_at = now;
    ctx.accounts.vgp_cat.cliff_started_at = now;
    ctx.accounts.founders_cat.cliff_started_at = now;
    ctx.accounts.marketing_cat.cliff_started_at = now;
    ctx.accounts.liquidity_cat.cliff_started_at = now;
    ctx.accounts.reserve_cat.cliff_started_at = now;

    let cpi_accounts = TransferChecked {
        from: ctx.accounts.master_ata.to_account_info(),
        to: ctx.accounts.marketing_ata.to_account_info(),
        authority: ctx.accounts.master_pda.to_account_info(),
        mint: ctx.accounts.mint.to_account_info(),
    };
    let cpi_ctx = CpiContext::new_with_signer(
        ctx.accounts.token_program.to_account_info(),
        cpi_accounts,
        signer_seeds,
    );
    token_2022::transfer_checked(cpi_ctx, MARKETING_LIQUID_SUPPLY, SBT_DECIMALS)?;

    let cpi_accounts = TransferChecked {
        from: ctx.accounts.master_ata.to_account_info(),
        to: ctx.accounts.liquidity_ata.to_account_info(),
        authority: ctx.accounts.master_pda.to_account_info(),
        mint: ctx.accounts.mint.to_account_info(),
    };
    let cpi_ctx = CpiContext::new_with_signer(
        ctx.accounts.token_program.to_account_info(),
        cpi_accounts,
        signer_seeds,
    );
    token_2022::transfer_checked(cpi_ctx, LIQUIDITY_LIQUID_SUPPLY, SBT_DECIMALS)?;

    let cpi_accounts = TransferChecked {
        from: ctx.accounts.master_ata.to_account_info(),
        to: ctx.accounts.reserve_ata.to_account_info(),
        authority: ctx.accounts.master_pda.to_account_info(),
        mint: ctx.accounts.mint.to_account_info(),
    };
    let cpi_ctx = CpiContext::new_with_signer(
        ctx.accounts.token_program.to_account_info(),
        cpi_accounts,
        signer_seeds,
    );
    token_2022::transfer_checked(
        cpi_ctx,
        RESERVE_LIQUID_SUPPLY + RESERVE_PADDING,
        SBT_DECIMALS,
    )?;

    Ok(())
}

#[derive(Accounts)]
pub struct Tge<'info> {
    #[account(
        mut,
        signer @ crate::error::ErrorCode::MasterMustSign,
        constraint = crate::LOCAL_TESTING || master.key() == crate::SBARTER_MULTISIG
    )]
    pub master: Signer<'info>,
    #[account(
        mut,
        seeds = [b"master"],
        bump
    )]
    /// CHECK: pda authority
    pub master_pda: AccountInfo<'info>,
    /// CHECK: created by Initialize, checked to be owned by master_pda
    #[account(
        mut,
        associated_token::mint = mint,
        associated_token::authority = master_pda,
        associated_token::token_program = token_program
    )]
    pub master_ata: InterfaceAccount<'info, TokenAccount>,

    #[account(
        mut,
        seeds = [PRE_SEED_CATEGORY.seed, mint.key().as_ref()],
        bump
    )]
    pub pre_seed_cat: Box<Account<'info, InvestorCategoryData>>,

    #[account(
        mut,
        seeds = [SEED_CATEGORY.seed, mint.key().as_ref()],
        bump
    )]
    pub seed_cat: Box<Account<'info, InvestorCategoryData>>,

    #[account(
        mut,
        seeds = [INSTITUTIONAL_CATEGORY.seed, mint.key().as_ref()],
        bump
    )]
    pub institutional_cat: Box<Account<'info, InvestorCategoryData>>,

    #[account(
        mut,
        seeds = [VGP_CATEGORY.seed, mint.key().as_ref()],
        bump
    )]
    pub vgp_cat: Box<Account<'info, InvestorCategoryData>>,

    #[account(
        mut,
        seeds = [FOUNDERS_CATEGORY.seed, mint.key().as_ref()],
        bump
    )]
    pub founders_cat: Box<Account<'info, InvestorCategoryData>>,

    #[account(
        mut,
        seeds = [MARKETING_CATEGORY.seed, mint.key().as_ref()],
        bump
    )]
    pub marketing_cat: Box<Account<'info, FunctionalCategoryData>>,
    /// CHECK: provided category authority, no checks
    pub marketing_authority: AccountInfo<'info>,
    #[account(
        mut,
        associated_token::mint = mint,
        associated_token::authority = marketing_authority,
        associated_token::token_program = token_program
    )]
    pub marketing_ata: Box<InterfaceAccount<'info, TokenAccount>>,

    #[account(
        mut,
        seeds = [LIQUIDITY_CATEGORY.seed, mint.key().as_ref()],
        bump
    )]
    pub liquidity_cat: Box<Account<'info, FunctionalCategoryData>>,
    /// CHECK: provided category authority, no checks
    pub liquidity_authority: AccountInfo<'info>,
    /// CHECK: created by Initialize, checked to be owned by liquidity_cat
    #[account(
        mut,
        associated_token::mint = mint,
        associated_token::authority = liquidity_authority,
        associated_token::token_program = token_program
    )]
    pub liquidity_ata: Box<InterfaceAccount<'info, TokenAccount>>,

    #[account(
        mut,
        seeds = [RESERVE_CATEGORY.seed, mint.key().as_ref()],
        bump
    )]
    pub reserve_cat: Box<Account<'info, FunctionalCategoryData>>,
    /// CHECK: provided category authority, no checks
    pub reserve_authority: AccountInfo<'info>,
    /// CHECK: created by Initialize, checked to be owned by liquidity_cat
    #[account(
        mut,
        associated_token::mint = mint,
        associated_token::authority = reserve_authority,
        associated_token::token_program = token_program
    )]
    pub reserve_ata: Box<InterfaceAccount<'info, TokenAccount>>,

    #[account(mut, mint::authority = master_pda)]
    pub mint: Box<InterfaceAccount<'info, Mint>>,
    pub token_program: Program<'info, Token2022>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
}
