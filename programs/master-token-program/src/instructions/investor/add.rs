use anchor_lang::{prelude::*};
use anchor_spl::{
    associated_token::AssociatedToken,
    token_2022::Token2022,
    token_interface::{Mint, TokenAccount},
};

use crate::{SBT_DECIMALS, VESTING_MONTH, states::{Investor, InvestorCategoryData, PRE_SEED_CATEGORY, SEED_CATEGORY, investor_category_seed_is_valid}};

pub fn add_investor_to_category<'info>(
    ctx: Context<'_, '_, '_, 'info, AddInvestorToCategory<'info>>,
    category_seed: String,
    new_investor_index: u16,
    total_allocation_in_whole_sbts: u64,
) -> Result<()> {
    require!(
        investor_category_seed_is_valid(&category_seed),
        crate::error::ErrorCode::CategorySeed
    );
    
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
    require_gt!(
        total_allocation_in_base_units,
        0,
        crate::error::ErrorCode::InvestorAllocation
    );
    require_gte!(
        category.unallocated_tokens_left,
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

    #[cfg(not(feature = "local-testing"))]
    if full_months_since_last_claim > 0 {
        return err!(crate::error::ErrorCode::CategoryLevelUnclaimed);
    }

    investor.wallet = ctx.accounts.investor_wallet.key();

    investor.cliff_months_remaining = category.cliff_months_remaining;
    investor.vesting_months_remaining = category.vesting_months_remaining;
    investor.last_offset_months = category.months_claimed;
    
    // removing the requirement to always have the category claimed up to the current cycle
    // when testing locally, because it's inconvenient when working with minute-cycles,
    // requires accounting for amount of months not claimed for on category level yet
    #[cfg(feature = "local-testing")]
    {
        let cliff_months_skipped = full_months_since_last_claim.min(category.cliff_months_remaining);
        let vesting_months_skipped =
            (full_months_since_last_claim - cliff_months_skipped).min(category.vesting_months_remaining);
        investor.cliff_months_remaining -= cliff_months_skipped;
        investor.vesting_months_remaining -= vesting_months_skipped;
        investor.last_offset_months += full_months_since_last_claim;
    }
    
    if investor.cliff_months_remaining == 0 && investor.vesting_months_remaining <= 1 {
        return err!(crate::error::ErrorCode::VestingScheduleFinished);
    }

    if investor.cliff_months_remaining == 0 && investor.vesting_months_remaining > 0{
        // has to wait an extra month if joined during vesting, 
        // because the process of adding an investor always happens half way through a cycle,
        // so we skip the remainder of the first month
        investor.cliff_months_remaining = 1;
        investor.vesting_months_remaining -= 1;
        // the total allocation, spread evenly over `vesting_months_remaining + 1`,
        // used for token reservation in the category specifically, while
        // the actual token movement goes according to the schedule with 1 month of extra cliff,
        // and thus 1 month of vesting less
        category.total_allocated_tokens_monthly += total_allocation_in_base_units / (investor.vesting_months_remaining + 1) as u64;
    } else {
        category.total_allocated_tokens_monthly += total_allocation_in_base_units / investor.vesting_months_remaining as u64;
    }
    
    // we can afford minor integer division error
    investor.monthly_allocation_in_base_units = total_allocation_in_base_units / investor.vesting_months_remaining as u64;
    category.investor_count += 1;
    category.unallocated_tokens_left -= total_allocation_in_base_units;

    Ok(())
}

#[derive(Accounts)]
#[instruction(category_seed: String, new_investor_index: u16, total_allocation_in_whole_sbts: u64)]
pub struct AddInvestorToCategory<'info> {
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
