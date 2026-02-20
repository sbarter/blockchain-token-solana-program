pub mod functional_categories;
pub mod investor_categories;
#[cfg(feature = "local-testing")]
pub mod mint;

pub use functional_categories::*;
pub use investor_categories::*;
#[cfg(feature = "local-testing")]
pub use mint::*;

use anchor_lang::prelude::*;

use crate::states::{FunctionalCategoryData, InvestorCategoryData};

fn initialize_investor_category<'info>(
    category: &mut Account<'info, InvestorCategoryData>,
    data: InvestorCategoryData,
) -> Result<()> {
    category.monthly_allocation = data.monthly_allocation;
    category.unallocated_tokens_left = data.unallocated_tokens_left;
    category.tokens_ready_for_claim = data.tokens_ready_for_claim;
    category.cliff_months_remaining = data.cliff_months_remaining;
    category.vesting_months_remaining = data.vesting_months_remaining;
    category.investor_count = data.investor_count;
    category.is_open = data.is_open;
    Ok(())
}

fn initialize_functional_category<'info>(
    category: &mut Account<'info, FunctionalCategoryData>,
    data: FunctionalCategoryData,
) -> Result<()> {
    category.wallet = data.wallet;
    category.monthly_allocation_in_base_units = data.monthly_allocation_in_base_units;
    category.cliff_months_remaining = data.cliff_months_remaining;
    category.vesting_months_remaining = data.vesting_months_remaining;
    Ok(())
}
