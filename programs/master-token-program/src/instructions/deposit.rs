use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token_2022::{self, Token2022, TransferChecked},
    token_interface::{Mint, TokenAccount},
};

use crate::{states::InvestorCategoryData, SBT_DECIMALS};

pub fn deposit_category_tokens<'info>(
    ctx: Context<'_, '_, '_, 'info, DepositCategoryTokens<'info>>,
    _category_seed: String,
    amount: u64,
) -> Result<()> {
    let category = &mut ctx.accounts.category;

    // NOTE: I'm not really sure if this should be constrained in any way.
    // Even the master signature is not necessary. Anybody could allocate tokens to the category,
    // knowing that the master wallet will manage them from that point on.

    let cpi_accounts = TransferChecked {
        from: ctx.accounts.sender_ata.to_account_info(),
        to: ctx.accounts.category_ata.to_account_info(),
        authority: ctx.accounts.sender.to_account_info(),
        mint: ctx.accounts.mint.to_account_info(),
    };
    let cpi_ctx = CpiContext::new(ctx.accounts.token_program.to_account_info(), cpi_accounts);
    token_2022::transfer_checked(cpi_ctx, amount, SBT_DECIMALS as u8)?;
    category.unallocated_total_tokens += amount;

    Ok(())
}

#[derive(Accounts)]
#[instruction(category_seed: String, amount: u64)]
pub struct DepositCategoryTokens<'info> {
    #[account(mut, signer)]
    pub sender: Signer<'info>,
    #[account(
        mut,
        associated_token::mint = mint,
        associated_token::authority = sender,
        associated_token::token_program = associated_token_program
    )]
    pub sender_ata: InterfaceAccount<'info, TokenAccount>,

    #[account(
        mut,
        seeds = [category_seed.as_bytes(), mint.key().as_ref()],
        bump
    )]
    pub category: Account<'info, InvestorCategoryData>,

    #[account(
        mut,
        associated_token::mint = mint,
        associated_token::authority = category,
        associated_token::token_program = associated_token_program
    )]
    pub category_ata: InterfaceAccount<'info, TokenAccount>,

    pub mint: Box<InterfaceAccount<'info, Mint>>,
    pub token_program: Program<'info, Token2022>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
}
