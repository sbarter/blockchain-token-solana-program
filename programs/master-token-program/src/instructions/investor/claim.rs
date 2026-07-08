use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token_2022::{self, Token2022, TransferChecked},
    token_interface::{Mint, TokenAccount},
};

use crate::{
    states::{investor_category_seed_is_valid, Investor, InvestorCategoryData},
    SBT_DECIMALS, VESTING_MONTH,
};

pub fn investor_claim_tokens<'info>(
    ctx: Context<'_, '_, '_, 'info, InvestorClaimTokens<'info>>,
    category_seed: String,
    _investor_index: u16,
) -> Result<()> {
    require!(
        investor_category_seed_is_valid(&category_seed),
        crate::error::ErrorCode::CategorySeed
    );

    let category = &ctx.accounts.category;
    let investor = &mut ctx.accounts.investor_pda;

    let master_seeds = &[
        category_seed.as_bytes(),
        &ctx.accounts.mint.key().to_bytes(),
        &[ctx.bumps.category],
    ];
    let signer_seeds = &[&master_seeds[..]];

    let now = Clock::get()?.unix_timestamp as u64;
    require!(
        category.cliff_started_at != 0,
        crate::error::ErrorCode::TgeNotHappened
    );

    // TODO: maybe as a solution to what Marco wants:
    // only claim for category.vesting_months_remaining - investor.vesting_months_remaining,
    // this way the claim for a month only becomes accessible after
    // the category has claimed for that month.
    let since_tge = now.saturating_sub(category.cliff_started_at);
    let months_elapsed = since_tge / VESTING_MONTH;
    let total_months = months_elapsed
        .saturating_sub(investor.last_offset_months as u64)
        .min(48u8.saturating_sub(investor.last_offset_months) as u64) as u8;

    msg!("Months since TGE:");
    msg!(&months_elapsed.to_string());
    msg!("Claiming for months:");
    msg!(&total_months.to_string());

    #[cfg(not(feature = "local-testing"))]
    if months_elapsed != category.months_claimed as u64 {
        msg!("Category-level claim for this cycle has to happen first. Try again later");
        return Ok(());
    }

    if total_months == 0 {
        msg!("No claim available.");
        return Ok(());
    }

    let cliff_months_claimed = total_months.min(investor.cliff_months_remaining);
    let vesting_months_claimed =
        (total_months - cliff_months_claimed).min(investor.vesting_months_remaining);
    let total_tokens = investor.monthly_allocation_in_base_units * vesting_months_claimed as u64;

    if total_tokens > 0 {
        if ctx.accounts.category_ata.amount < total_tokens {
            msg!("No available tokens in the category at the moment. You can always try again.");
            return Ok(());
        }
        let cpi_accounts = TransferChecked {
            from: ctx.accounts.category_ata.to_account_info(),
            to: ctx.accounts.investor_ata.to_account_info(),
            authority: ctx.accounts.category.to_account_info(),
            mint: ctx.accounts.mint.to_account_info(),
        };
        let cpi_ctx = CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            cpi_accounts,
            signer_seeds,
        );
        token_2022::transfer_checked(cpi_ctx, total_tokens, SBT_DECIMALS)?;
    }
    investor.cliff_months_remaining -= cliff_months_claimed;
    investor.vesting_months_remaining -= vesting_months_claimed;
    investor.last_offset_months = investor.last_offset_months.saturating_add(total_months);
    ctx.accounts.category.tokens_ready_for_claim = ctx
        .accounts
        .category
        .tokens_ready_for_claim
        .saturating_sub(total_tokens);

    Ok(())
}

#[derive(Accounts)]
#[instruction(category_seed: String, investor_index: u16)]
pub struct InvestorClaimTokens<'info> {
    #[account(
        mut,
        seeds = [category_seed.as_bytes(), investor_index.to_le_bytes().as_ref(),
        mint.key().as_ref()],
        bump
    )]
    pub investor_pda: Account<'info, Investor>,

    #[account(
        mut,
        associated_token::mint = mint,
        associated_token::authority = investor_pda.wallet,
        associated_token::token_program = token_program
    )]
    pub investor_ata: InterfaceAccount<'info, TokenAccount>,

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

    pub mint: Box<InterfaceAccount<'info, Mint>>,
    pub token_program: Program<'info, Token2022>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
}
