use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token_2022::Token2022,
    token_interface::{Mint, TokenAccount},
};

use crate::states::{investor_category_seed_is_valid, Investor, InvestorCategoryData};

pub fn admin_change_investor_wallet<'info>(
    ctx: Context<'_, '_, '_, 'info, ChangeInvestorWallet<'info>>,
    category_seed: String,
    _investor_index: u16,
) -> Result<()> {
    require!(
        investor_category_seed_is_valid(&category_seed),
        crate::error::ErrorCode::CategorySeed
    );

    let investor = &mut ctx.accounts.investor_pda;
    investor.wallet = ctx.accounts.new_investor_wallet.key();
    Ok(())
}

#[derive(Accounts)]
#[instruction(category_seed: String, investor_index: u16)]
pub struct ChangeInvestorWallet<'info> {
    #[account(
        mut,
        signer @ crate::error::ErrorCode::MasterMustSign,
        constraint = crate::LOCAL_TESTING || master.key() == crate::SBARTER_MULTISIG
    )]
    pub master: Signer<'info>,

    #[account(
        mut,
        seeds = [category_seed.as_bytes(), investor_index.to_le_bytes().as_ref(),
        mint.key().as_ref()],
        bump
    )]
    pub investor_pda: Account<'info, Investor>,

    /// CHECK: old investor wallet from PDA
    #[account(address = investor_pda.wallet @ crate::error::ErrorCode::OldPubkeyMismatch)]
    pub old_investor_wallet: UncheckedAccount<'info>,
    /// CHECK: any wallet
    pub new_investor_wallet: UncheckedAccount<'info>,

    #[account(
        init_if_needed,
        payer = master,
        associated_token::mint = mint,
        associated_token::authority = new_investor_wallet,
        associated_token::token_program = token_program
    )]
    pub new_investor_ata: InterfaceAccount<'info, TokenAccount>,

    #[account(
        seeds = [category_seed.as_bytes(), mint.key().as_ref()],
        bump
    )]
    pub category: Account<'info, InvestorCategoryData>,

    pub mint: Box<InterfaceAccount<'info, Mint>>,
    pub token_program: Program<'info, Token2022>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
}
