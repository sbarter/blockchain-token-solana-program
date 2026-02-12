pub mod functional_categories;
pub mod investor_categories;
pub mod mint;

pub use functional_categories::*;
pub use investor_categories::*;
pub use mint::*;

use anchor_lang::prelude::*;

use crate::states::{FunctionalCategoryData, InvestorCategoryData};

fn initialize_investor_category<'info>(
    category: &mut Account<'info, InvestorCategoryData>,
    data: InvestorCategoryData,
) -> Result<()> {
    category.monthly_allocation = data.monthly_allocation;
    category.unallocated_total_tokens = data.unallocated_total_tokens;
    category.allocated_unclaimed_tokens = data.allocated_unclaimed_tokens;
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
