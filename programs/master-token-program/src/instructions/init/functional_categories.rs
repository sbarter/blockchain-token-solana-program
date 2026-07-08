use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token_2022::Token2022,
    token_interface::{Mint, TokenAccount},
};

use crate::{
    instructions::init::initialize_functional_category,
    states::{FunctionalCategoryData, LIQUIDITY_CATEGORY, MARKETING_CATEGORY, RESERVE_CATEGORY},
};

pub fn initialize_functional<'info>(
    ctx: Context<'_, '_, '_, 'info, InitializeFunctionalCategories<'info>>,
) -> Result<()> {
    if let Err(e) =
        initialize_functional_category(&mut ctx.accounts.marketing_cat, MARKETING_CATEGORY.data)
    {
        msg!("Failed to initialize category for marketing!");
        return Err(e);
    }
    if let Err(e) =
        initialize_functional_category(&mut ctx.accounts.reserve_cat, RESERVE_CATEGORY.data)
    {
        msg!("Failed to initialize category for reserve!");
        return Err(e);
    }
    if let Err(e) =
        initialize_functional_category(&mut ctx.accounts.liquidity_cat, LIQUIDITY_CATEGORY.data)
    {
        msg!("Failed to initialize category for liquidity!");
        return Err(e);
    }
    Ok(())
}

#[derive(Accounts)]
pub struct InitializeFunctionalCategories<'info> {
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
    /// CHECK: master pda authority
    pub master_pda: AccountInfo<'info>,

    #[account(
        init_if_needed,
        payer = master,
        associated_token::mint = mint,
        associated_token::authority = master_pda,
        associated_token::token_program = token_program
    )]
    pub master_ata: Box<InterfaceAccount<'info, TokenAccount>>,

    #[account(
        init,
        payer = master,
        space = 8 + FunctionalCategoryData::INIT_SPACE,
        seeds = [MARKETING_CATEGORY.seed, mint.key().as_ref()],
        bump
    )]
    pub marketing_cat: Box<Account<'info, FunctionalCategoryData>>,
    #[account(
        address = MARKETING_CATEGORY.data.wallet @ crate::error::ErrorCode::FunctionalCategoryAuthority
    )]
    /// CHECK: marketing authority wallet
    pub marketing_authority: AccountInfo<'info>,
    #[account(
        init_if_needed,
        payer = master,
        associated_token::mint = mint,
        associated_token::authority = marketing_authority,
        associated_token::token_program = token_program
    )]
    pub marketing_ata: Box<InterfaceAccount<'info, TokenAccount>>,

    #[account(
        init,
        payer = master,
        space = 8 + FunctionalCategoryData::INIT_SPACE,
        seeds = [RESERVE_CATEGORY.seed, mint.key().as_ref()],
        bump
    )]
    pub reserve_cat: Box<Account<'info, FunctionalCategoryData>>,
    #[account(
        address = RESERVE_CATEGORY.data.wallet @ crate::error::ErrorCode::FunctionalCategoryAuthority
    )]
    /// CHECK: reserve authority wallet
    pub reserve_authority: AccountInfo<'info>,
    #[account(
        init_if_needed,
        payer = master,
        associated_token::mint = mint,
        associated_token::authority = reserve_authority,
        associated_token::token_program = token_program
    )]
    pub reserve_ata: Box<InterfaceAccount<'info, TokenAccount>>,

    #[account(
        init,
        payer = master,
        space = 8 + FunctionalCategoryData::INIT_SPACE,
        seeds = [LIQUIDITY_CATEGORY.seed, mint.key().as_ref()],
        bump
    )]
    pub liquidity_cat: Box<Account<'info, FunctionalCategoryData>>,
    #[account(
        address = LIQUIDITY_CATEGORY.data.wallet @ crate::error::ErrorCode::FunctionalCategoryAuthority
    )]
    /// CHECK: liquidity authority wallet
    pub liquidity_authority: AccountInfo<'info>,
    #[account(
        init_if_needed,
        payer = master,
        associated_token::mint = mint,
        associated_token::authority = liquidity_authority,
        associated_token::token_program = token_program
    )]
    pub liquidity_ata: Box<InterfaceAccount<'info, TokenAccount>>,

    #[account(
        mut,
        mint::authority = master_pda
    )]
    pub mint: Box<InterfaceAccount<'info, Mint>>,
    pub token_program: Program<'info, Token2022>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
}
