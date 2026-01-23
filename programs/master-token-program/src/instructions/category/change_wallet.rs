use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token_2022::Token2022,
    token_interface::{Mint, TokenAccount},
};

use crate::states::FunctionalCategoryData;

pub fn admin_change_functional_category_wallet<'info>(
    ctx: Context<'_, '_, '_, 'info, ChangeCategoryWallet<'info>>,
    _category_seed: String,
) -> Result<()> {
    let category = &mut ctx.accounts.category;
    category.wallet = ctx.accounts.new_manager_wallet.key();
    Ok(())
}

#[derive(Accounts)]
#[instruction(category_seed: String)]
pub struct ChangeCategoryWallet<'info> {
    #[account(
        mut,
        signer @ crate::error::ErrorCode::MasterMustSign,
        constraint = crate::TESTING || master.key() == crate::MASTER_WALLET
    )]
    pub master: Signer<'info>,

    /// CHECK: old manager wallet from PDA
    #[account(address = category.wallet @ crate::error::ErrorCode::OldPubkeyMismatch)]
    pub old_manager_wallet: UncheckedAccount<'info>,
    /// CHECK: any wallet
    pub new_manager_wallet: UncheckedAccount<'info>,

    #[account(
        init,
        payer = master,
        associated_token::mint = mint,
        associated_token::authority = new_manager_wallet,
        associated_token::token_program = token_program
    )]
    pub new_manager_ata: InterfaceAccount<'info, TokenAccount>,

    #[account(
        mut,
        seeds = [category_seed.as_bytes(), mint.key().as_ref()],
        bump
    )]
    pub category: Account<'info, FunctionalCategoryData>,

    pub mint: Box<InterfaceAccount<'info, Mint>>,
    pub token_program: Program<'info, Token2022>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
}
