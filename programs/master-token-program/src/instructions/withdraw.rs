use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token_2022::{self, Token2022, TransferChecked},
    token_interface::{Mint, TokenAccount},
};

use crate::{states::InvestorCategoryData, SBT_DECIMALS};

pub fn withdraw_category_tokens<'info>(
    ctx: Context<'_, '_, '_, 'info, WithdrawCategoryTokens<'info>>,
    _category_seed: String,
    amount: u64,
) -> Result<()> {
    let category = &mut ctx.accounts.category;
    require_gte!(
        category.unallocated_total_tokens,
        amount,
        crate::error::ErrorCode::TooManyTokensAllocated
    );
    require_neq!(
        category.cliff_started_at,
        0,
        crate::error::ErrorCode::TgeNotHappened
    );

    let cpi_accounts = TransferChecked {
        from: ctx.accounts.category_ata.to_account_info(),
        to: ctx.accounts.recipient_ata.to_account_info(),
        authority: ctx.accounts.program_authority.to_account_info(),
        mint: ctx.accounts.mint.to_account_info(),
    };
    let cpi_ctx = CpiContext::new(ctx.accounts.token_program.to_account_info(), cpi_accounts);
    token_2022::transfer_checked(cpi_ctx, amount, SBT_DECIMALS as u8)?;
    category.unallocated_total_tokens -= amount;

    Ok(())
}

#[derive(Accounts)]
#[instruction(category_seed: String, amount: u64)]
pub struct WithdrawCategoryTokens<'info> {
    #[account(mut, signer, address = crate::MASTER_WALLET)]
    pub master: Signer<'info>,

    #[account(
        mut,
        seeds = [category_seed.as_bytes(), mint.key().as_ref()],
        bump
    )]
    pub category: Account<'info, InvestorCategoryData>,

    #[account(
        mut,
        associated_token::mint = mint,
        associated_token::authority = program_authority,
        associated_token::token_program = associated_token_program
    )]
    pub category_ata: InterfaceAccount<'info, TokenAccount>,

    /// CHECK: Wallet of `recipient_ata`
    pub recipient: AccountInfo<'info>,

    #[account(
        init_if_needed,
        payer = master,
        associated_token::mint = mint,
        associated_token::authority = recipient,
        associated_token::token_program = associated_token_program
    )]
    pub recipient_ata: InterfaceAccount<'info, TokenAccount>,

    pub mint: Box<InterfaceAccount<'info, Mint>>,
    #[account(address = crate::ID)]
    /// CHECK: Has to be this program's ID
    pub program_authority: AccountInfo<'info>,
    pub token_program: Program<'info, Token2022>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
}
