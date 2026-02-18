use anchor_lang::{prelude::*};
use anchor_spl::{
    associated_token::AssociatedToken,
    token_2022::Token2022,
    token_interface::{Mint, TokenAccount},
};

use crate::{SBT_DECIMALS, VESTING_MONTH, states::{Investor, InvestorCategoryData, PRE_SEED_CATEGORY, SEED_CATEGORY}};

pub fn add_investor_to_category<'info>(
    ctx: Context<'_, '_, '_, 'info, AddInvestorToCategory<'info>>,
    category_seed: String,
    new_investor_index: u16,
    total_allocation_in_whole_sbts: u64,
) -> Result<()> {
    let Some(total_allocation_in_base_units) = total_allocation_in_whole_sbts.checked_mul(10u64.pow(SBT_DECIMALS as u32)) else {
        msg!("You are allocating WAY too many tokens. Do you know what you're doing?");
        msg!("The instruction expects the amount to be in whole SBTs, not base units!");
        return err!(crate::error::ErrorCode::TooManyTokensAllocated);
    };
    
    let investor = &mut ctx.accounts.investor_pda;
    let category = &mut ctx.accounts.category;

    let pre_investors = match category_seed.as_str() { 
        "preseed" => PRE_SEED_CATEGORY.pre_investors,
        "seed" => SEED_CATEGORY.pre_investors,
        _ => 0,
    };
    if pre_investors != 0 && new_investor_index > pre_investors {
        return err!(crate::error::ErrorCode::ClosedCategoryExceed);
    }

    require!(category.is_open || category.cliff_started_at == 0, crate::error::ErrorCode::CategoryClosed);
    require_eq!(category.investor_count + 1, new_investor_index, crate::error::ErrorCode::InvestorIndex);
    require_gte!(
        category.unallocated_total_tokens,
        total_allocation_in_base_units,
        crate::error::ErrorCode::TooManyTokensAllocated
    );

    let full_months_since_last_claim = if category.cliff_started_at != 0 {
        let now = Clock::get()?.unix_timestamp as u64;
        let since_tge = now.saturating_sub(category.cliff_started_at);
        let months_elapsed = since_tge / VESTING_MONTH;
        months_elapsed
            .saturating_sub(category.months_claimed as u64)
            .min(48) as u8
    } else {
        0
    };

    if full_months_since_last_claim > 0 {
        return err!(crate::error::ErrorCode::CategoryLevelUnclaimed);
    }

    investor.wallet = ctx.accounts.investor_wallet.key();
    investor.cliff_months_remaining = category.cliff_months_remaining;
    investor.vesting_months_remaining = category.vesting_months_remaining;
    investor.last_offset_months = category.months_claimed;
    
    if investor.cliff_months_remaining == 0 && investor.vesting_months_remaining == 0 {
        return err!(crate::error::ErrorCode::VestingScheduleFinished);
    }

    if investor.cliff_months_remaining == 0 && investor.vesting_months_remaining > 0{
        // has to wait an extra month if joined during vesting
        investor.cliff_months_remaining = 1;
        investor.vesting_months_remaining -= 1;
    }    
    
    // we can afford minor integer division error
    investor.monthly_allocation_in_base_units = total_allocation_in_base_units / investor.vesting_months_remaining as u64;

    category.investor_count += 1;
    category.unallocated_total_tokens -= total_allocation_in_base_units;
    category.allocated_unclaimed_tokens += total_allocation_in_base_units;

    Ok(())
}

#[derive(Accounts)]
#[instruction(category_seed: String, new_investor_index: u16, total_allocation_in_whole_sbts: u64)]
pub struct AddInvestorToCategory<'info> {
    #[account(
        mut,
        signer @ crate::error::ErrorCode::MasterMustSign,
        constraint = crate::LOCAL_TESTING || master.key() == crate::MASTER_WALLET
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
        seeds = [category_seed.as_bytes(), new_investor_index.to_le_bytes().as_ref(),
        mint.key().as_ref()],
        space = 8 + Investor::INIT_SPACE,
        bump
    )]
    pub investor_pda: Account<'info, Investor>,

    /// CHECK: any investor wallet
    pub investor_wallet: UncheckedAccount<'info>,

    #[account(
        init_if_needed,
        payer = master,
        associated_token::mint = mint, 
        associated_token::authority = investor_wallet, 
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
