use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token_2022::Token2022,
    token_interface::{Mint, TokenAccount},
};

use crate::{instructions::init::initialize_investor_category, states::category::*};

pub fn initialize_investor<'info>(
    ctx: Context<'_, '_, '_, 'info, InitializeInvestorCategories<'info>>,
) -> Result<()> {
    if let Err(e) =
        initialize_investor_category(&mut ctx.accounts.pre_seed_cat, PRE_SEED_CATEGORY.data)
    {
        msg!("Failed to initialize category for pre-seed!");
        return Err(e);
    }
    if let Err(e) = initialize_investor_category(&mut ctx.accounts.seed_cat, SEED_CATEGORY.data) {
        msg!("Failed to initialize category for seed!");
        return Err(e);
    }
    if let Err(e) = initialize_investor_category(
        &mut ctx.accounts.institutional_cat,
        INSTITUTIONAL_CATEGORY.data,
    ) {
        msg!("Failed to initialize category for institutional!");
        return Err(e);
    }
    if let Err(e) = initialize_investor_category(&mut ctx.accounts.vgp_cat, VGP_CATEGORY.data) {
        msg!("Failed to initialize category for vgp!");
        return Err(e);
    }
    if let Err(e) =
        initialize_investor_category(&mut ctx.accounts.founders_cat, FOUNDERS_CATEGORY.data)
    {
        msg!("Failed to initialize category for founders!");
        return Err(e);
    }
    Ok(())
}

#[derive(Accounts)]
pub struct InitializeInvestorCategories<'info> {
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

    #[account(
        init,
        payer = master,
        space = 8 + InvestorCategoryData::INIT_SPACE,
        seeds = [PRE_SEED_CATEGORY.seed, mint.key().as_ref()],
        bump
    )]
    pub pre_seed_cat: Box<Account<'info, InvestorCategoryData>>,
    #[account(
        init_if_needed,
        payer = master,
        associated_token::mint = mint,
        associated_token::authority = pre_seed_cat,
        associated_token::token_program = token_program
    )]
    pub pre_seed_ata: Box<InterfaceAccount<'info, TokenAccount>>,

    #[account(
        init,
        payer = master,
        space = 8 + InvestorCategoryData::INIT_SPACE,
        seeds = [SEED_CATEGORY.seed, mint.key().as_ref()],
        bump
    )]
    pub seed_cat: Box<Account<'info, InvestorCategoryData>>,
    #[account(
        init_if_needed,
        payer = master,
        associated_token::mint = mint,
        associated_token::authority = seed_cat,
        associated_token::token_program = token_program
    )]
    pub seed_ata: Box<InterfaceAccount<'info, TokenAccount>>,

    #[account(
        init,
        payer = master,
        space = 8 + InvestorCategoryData::INIT_SPACE,
        seeds = [INSTITUTIONAL_CATEGORY.seed, mint.key().as_ref()],
        bump
    )]
    pub institutional_cat: Box<Account<'info, InvestorCategoryData>>,
    #[account(
        init_if_needed,
        payer = master,
        associated_token::mint = mint,
        associated_token::authority = institutional_cat,
        associated_token::token_program = token_program
    )]
    pub institutional_ata: Box<InterfaceAccount<'info, TokenAccount>>,

    #[account(
        init,
        payer = master,
        space = 8 + InvestorCategoryData::INIT_SPACE,
        seeds = [VGP_CATEGORY.seed, mint.key().as_ref()],
        bump
    )]
    pub vgp_cat: Box<Account<'info, InvestorCategoryData>>,
    #[account(
        init_if_needed,
        payer = master,
        associated_token::mint = mint,
        associated_token::authority = vgp_cat,
        associated_token::token_program = token_program
    )]
    pub vgp_ata: Box<InterfaceAccount<'info, TokenAccount>>,

    #[account(
        init,
        payer = master,
        space = 8 + InvestorCategoryData::INIT_SPACE,
        seeds = [FOUNDERS_CATEGORY.seed, mint.key().as_ref()],
        bump
    )]
    pub founders_cat: Box<Account<'info, InvestorCategoryData>>,
    #[account(
        init_if_needed,
        payer = master,
        associated_token::mint = mint,
        associated_token::authority = founders_cat,
        associated_token::token_program = token_program
    )]
    pub founders_ata: Box<InterfaceAccount<'info, TokenAccount>>,

    #[account(
        mut,
        mint::authority = master_pda
    )]
    pub mint: Box<InterfaceAccount<'info, Mint>>,
    pub token_program: Program<'info, Token2022>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
}
