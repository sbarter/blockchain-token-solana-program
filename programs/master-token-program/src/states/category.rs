use anchor_lang::prelude::*;

use crate::{
    FOUNDERS_MONTHLY_SUPPLY, INSTITUTIONAL_MONTHLY_SUPPLY, LIQUIDITY_MONTHLY_SUPPLY,
    MARKETING_MONTHLY_SUPPLY, PRESEED_MONTHLY_SUPPLY, RESERVE_MONTHLY_SUPPLY, SEED_MONTHLY_SUPPLY,
    VGP_MONTHLY_SUPPLY,
};

#[derive(Debug, InitSpace)]
#[account]
pub struct InvestorCategoryData {
    pub monthly_allocation: u64,
    pub unallocated_total_tokens: u64,
    pub allocated_unclaimed_tokens: u64,

    pub cliff_started_at: u64,
    pub months_claimed: u8,
    pub cliff_months_remaining: u8,
    pub vesting_months_remaining: u8,
    pub investor_count: u16,
    pub is_open: bool,
}

#[derive(Debug, InitSpace)]
#[account]
pub struct FunctionalCategoryData {
    pub wallet: Pubkey,
    pub monthly_allocation: u64,

    pub cliff_started_at: u64,
    pub months_claimed: u8,
    pub cliff_months_remaining: u8,
    pub vesting_months_remaining: u8,
}

pub struct Category<T> {
    pub seed: &'static [u8],
    pub data: T,
    /// Closed categories have known amount of investors
    /// that a category must have initialized before TGE.
    pub pre_investors: u16,
}

// TODO: Use actual number of pre-investors for PRE_SEED and SEED,
// we expect to know it before TGE.
// However, we do not validate for concrete investor wallet pubkeys.

pub const PRE_SEED_CATEGORY: Category<InvestorCategoryData> = Category {
    seed: b"preseed",
    data: InvestorCategoryData {
        monthly_allocation: PRESEED_MONTHLY_SUPPLY,
        unallocated_total_tokens: PRESEED_MONTHLY_SUPPLY * 24,
        allocated_unclaimed_tokens: 0,
        cliff_started_at: 0,
        months_claimed: 0,
        cliff_months_remaining: 12,
        vesting_months_remaining: 24,
        investor_count: 0,
        is_open: false,
    },
    pre_investors: 3,
};

pub const SEED_CATEGORY: Category<InvestorCategoryData> = Category {
    seed: b"seed",
    data: InvestorCategoryData {
        monthly_allocation: SEED_MONTHLY_SUPPLY,
        unallocated_total_tokens: SEED_MONTHLY_SUPPLY * 18,
        allocated_unclaimed_tokens: 0,
        cliff_started_at: 0,
        months_claimed: 0,
        cliff_months_remaining: 6,
        vesting_months_remaining: 18,
        investor_count: 0,
        is_open: false,
    },
    pre_investors: 2,
};

pub const INSTITUTIONAL_CATEGORY: Category<InvestorCategoryData> = Category {
    seed: b"institutional",
    data: InvestorCategoryData {
        monthly_allocation: INSTITUTIONAL_MONTHLY_SUPPLY,
        unallocated_total_tokens: INSTITUTIONAL_MONTHLY_SUPPLY * 24,
        allocated_unclaimed_tokens: 0,
        cliff_started_at: 0,
        months_claimed: 0,
        cliff_months_remaining: 12,
        vesting_months_remaining: 24,
        investor_count: 0,
        is_open: true,
    },
    pre_investors: 0,
};

pub const VGP_CATEGORY: Category<InvestorCategoryData> = Category {
    seed: b"vgp",
    data: InvestorCategoryData {
        monthly_allocation: VGP_MONTHLY_SUPPLY,
        unallocated_total_tokens: VGP_MONTHLY_SUPPLY * 24,
        allocated_unclaimed_tokens: 0,
        cliff_started_at: 0,
        months_claimed: 0,
        cliff_months_remaining: 12,
        vesting_months_remaining: 24,
        investor_count: 0,
        is_open: true,
    },
    pre_investors: 0,
};

pub const FOUNDERS_CATEGORY: Category<InvestorCategoryData> = Category {
    seed: b"founders",
    data: InvestorCategoryData {
        monthly_allocation: FOUNDERS_MONTHLY_SUPPLY,
        unallocated_total_tokens: FOUNDERS_MONTHLY_SUPPLY * 24,
        allocated_unclaimed_tokens: 0,
        cliff_started_at: 0,
        months_claimed: 0,
        cliff_months_remaining: 12,
        vesting_months_remaining: 24,
        investor_count: 0,
        is_open: true,
    },
    pre_investors: 0,
};

// TODO: Use actual manager wallets for functional categories.

pub const MARKETING_CATEGORY: Category<FunctionalCategoryData> = Category {
    seed: b"marketing",
    data: FunctionalCategoryData {
        wallet: pubkey!("GSd6RQZ4o9AMpHeRYZEwcjZ9oAP1ZLAUeKbwbNdS2oJH"),
        monthly_allocation: MARKETING_MONTHLY_SUPPLY,
        cliff_started_at: 0,
        months_claimed: 0,
        cliff_months_remaining: 6,
        vesting_months_remaining: 36,
    },
    pre_investors: 0,
};

pub const RESERVE_CATEGORY: Category<FunctionalCategoryData> = Category {
    seed: b"reserve",
    data: FunctionalCategoryData {
        wallet: pubkey!("BdRUCurxjZvzBurS8QzzEKQ8iPCpzMYTz2YTgqnw9ZGY"),
        monthly_allocation: RESERVE_MONTHLY_SUPPLY,
        cliff_started_at: 0,
        months_claimed: 0,
        cliff_months_remaining: 0,
        vesting_months_remaining: 48,
    },
    pre_investors: 0,
};

pub const LIQUIDITY_CATEGORY: Category<FunctionalCategoryData> = Category {
    seed: b"liquidity",
    data: FunctionalCategoryData {
        wallet: pubkey!("2eg4xRrj742edVzGAfd3wnmXAMzhAcR1XdJoBARx3hcE"),
        monthly_allocation: LIQUIDITY_MONTHLY_SUPPLY,
        cliff_started_at: 0,
        months_claimed: 0,
        cliff_months_remaining: 0,
        vesting_months_remaining: 12,
    },
    pre_investors: 0,
};
