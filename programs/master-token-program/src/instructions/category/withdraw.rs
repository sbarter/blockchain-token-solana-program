use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token_2022::{self, Token2022, TransferChecked},
    token_interface::{Mint, TokenAccount},
};

use crate::{
    states::{investor_category_seed_is_valid, InvestorCategoryData},
    SBT_DECIMALS,
};

pub fn withdraw_category_tokens<'info>(
    ctx: Context<'_, '_, '_, 'info, WithdrawCategoryTokens<'info>>,
    category_seed: String,
    amount_in_whole_sbts: u64,
) -> Result<()> {
    require!(
        investor_category_seed_is_valid(&category_seed),
        crate::error::ErrorCode::CategorySeed
    );

    let Some(amount_in_base_units) =
        amount_in_whole_sbts.checked_mul(10u64.pow(SBT_DECIMALS as u32))
    else {
        msg!("You are withdrawing WAY too many tokens. Do you know what you're doing?");
        msg!("The instruction expects the amount to be in whole SBTs, not base units!");
        return err!(crate::error::ErrorCode::TooManyTokensAllocated);
    };

    let category = &ctx.accounts.category;
    require_gte!(
        category.unallocated_tokens_left,
        amount_in_base_units,
        crate::error::ErrorCode::TooManyTokensAllocated
    );
    require_gte!(
        ctx.accounts
            .category_ata
            .amount
            .saturating_sub(category.tokens_ready_for_claim),
        amount_in_base_units,
        crate::error::ErrorCode::TokensUnavailable
    );
    require_neq!(
        category.cliff_started_at,
        0,
        crate::error::ErrorCode::TgeNotHappened
    );

    let master_seeds = &[
        category_seed.as_bytes(),
        &ctx.accounts.mint.key().to_bytes(),
        &[ctx.bumps.category],
    ];
    let signer_seeds = &[&master_seeds[..]];

    let cpi_accounts = TransferChecked {
        from: ctx.accounts.category_ata.to_account_info(),
        to: ctx.accounts.recipient_ata.to_account_info(),
        authority: ctx.accounts.category.to_account_info(),
        mint: ctx.accounts.mint.to_account_info(),
    };
    let cpi_ctx = CpiContext::new_with_signer(
        ctx.accounts.token_program.to_account_info(),
        cpi_accounts,
        signer_seeds,
    );
    token_2022::transfer_checked(cpi_ctx, amount_in_base_units, SBT_DECIMALS)?;
    ctx.accounts.category.unallocated_tokens_left -= amount_in_base_units;

    Ok(())
}

#[derive(Accounts)]
#[instruction(category_seed: String, amount_in_whole_sbts: u64)]
pub struct WithdrawCategoryTokens<'info> {
    #[account(
        mut,
        signer @ crate::error::ErrorCode::MasterMustSign,
        constraint = crate::LOCAL_TESTING || master.key() == crate::SBARTER_MULTISIG
    )]
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
        associated_token::authority = category,
        associated_token::token_program = token_program
    )]
    pub category_ata: InterfaceAccount<'info, TokenAccount>,

    /// CHECK: Wallet of `recipient_ata`
    pub recipient: AccountInfo<'info>,

    #[account(
        init_if_needed,
        payer = master,
        associated_token::mint = mint,
        associated_token::authority = recipient,
        associated_token::token_program = token_program
    )]
    pub recipient_ata: InterfaceAccount<'info, TokenAccount>,

    pub mint: Box<InterfaceAccount<'info, Mint>>,
    pub token_program: Program<'info, Token2022>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
}
