use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token_2022::Token2022,
    token_interface::{Mint, TokenAccount},
};

use crate::states::{Investor, InvestorCategoryData};

pub fn add_investor_to_category<'info>(
    ctx: Context<'_, '_, '_, 'info, AddInvestorToCategory<'info>>,
    _category_seed: String,
    new_investor_index: u32,
    monthly_allocation: u64
) -> Result<()> {
    let investor = &mut ctx.accounts.investor_pda;
    let category = &mut ctx.accounts.category;

    require!(category.cliff_started_at != 0, crate::error::ErrorCode::TgeNotHappened);
    require_eq!(category.investor_count + 1, new_investor_index, crate::error::ErrorCode::InvestorIndex);
    require_gte!(
        category.unallocated_total_tokens,
        monthly_allocation * category.vesting_months_remaining as u64,
        crate::error::ErrorCode::TooManyTokensAllocated
    );

    
    investor.wallet = ctx.accounts.investor_wallet.key();
    // has to wait an extra month if joined during vesting
    investor.first_month_skipped = category.cliff_months_remaining > 0;
    investor.monthly_allocation = monthly_allocation;
    investor.cliff_started_at = category.cliff_started_at;
    investor.months_claimed = 0;
    investor.cliff_months_remaining = category.cliff_months_remaining;
    investor.vesting_months_remaining = category.vesting_months_remaining;

    category.investor_count += 1;
    category.unallocated_total_tokens -= monthly_allocation * category.vesting_months_remaining as u64;
    
    Ok(())
}

#[derive(Accounts)]
#[instruction(category_seed: String, new_investor_index: u32, monthly_allocation: u64)]
pub struct AddInvestorToCategory<'info> {
    #[account(mut, signer, address = crate::MASTER_WALLET)]
    pub master: Signer<'info>,

    #[account(
        init,
        payer = master,
        seeds = [category_seed.as_bytes(), new_investor_index.to_le_bytes().as_ref(),
        mint.key().as_ref()],
        space = Investor::LEN,
        bump
    )]
    pub investor_pda: Account<'info, Investor>,
    
    /// CHECK: Frankly we don't care if it's funded or anything.
    pub investor_wallet: UncheckedAccount<'info>,
    
    #[account(
        init_if_needed,
        payer = master,
        associated_token::mint = mint, 
        associated_token::authority = investor_wallet, 
        associated_token::token_program = associated_token_program
    )]
    pub investor_ata: InterfaceAccount<'info, TokenAccount>,
    #[account(
        mut,
        seeds = [category_seed.as_bytes(), mint.key().as_ref()],
        bump
    )]
    pub category: Account<'info, InvestorCategoryData>,

    pub mint: Box<InterfaceAccount<'info, Mint>>,
    pub token_program: Program<'info, Token2022>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
}
