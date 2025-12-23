use anchor_lang::prelude::*;

use crate::{
    FOUNDERS_MONTHLY_SUPPLY, INSTITUTIONAL_MONTHLY_SUPPLY, LIQUIDITY_MONTHLY_SUPPLY,
    MARKETING_MONTHLY_SUPPLY, PRESEED_MONTHLY_SUPPLY, RESERVE_MONTHLY_SUPPLY, SEED_MONTHLY_SUPPLY,
    VGP_MONTHLY_SUPPLY,
};

#[derive(Debug)]
#[account]
pub struct InvestorCategoryData {
    pub monthly_allocation: u64,
    pub unallocated_total_tokens: u64,

    pub cliff_started_at: u64,
    pub months_claimed: u8,
    pub cliff_months_remaining: u8,
    pub vesting_months_remaining: u8,
    pub investor_count: u32,
    pub is_open: bool,
}

impl InvestorCategoryData {
    pub const LEN: usize = 8 + 8 + 8 + 8 + 1 + 1 + 1 + 4 + 1;
}

#[derive(Debug)]
#[account]
pub struct FunctionalCategoryData {
    pub wallet: Pubkey,
    pub monthly_allocation: u64,

    pub cliff_started_at: u64,
    pub months_claimed: u8,
    pub cliff_months_remaining: u8,
    pub vesting_months_remaining: u8,
}

impl FunctionalCategoryData {
    pub const LEN: usize = 8 + 32 + 8 + 1 + 1;
}

pub const PRE_SEED_CATEGORY: (&[u8], InvestorCategoryData) = (
    b"preseed",
    InvestorCategoryData {
        monthly_allocation: PRESEED_MONTHLY_SUPPLY,
        unallocated_total_tokens: PRESEED_MONTHLY_SUPPLY * 24,
        cliff_started_at: 0,
        months_claimed: 0,
        cliff_months_remaining: 12,
        vesting_months_remaining: 24,
        investor_count: 0,
        is_open: false,
    },
);

pub const SEED_CATEGORY: (&[u8], InvestorCategoryData) = (
    b"seed",
    InvestorCategoryData {
        monthly_allocation: SEED_MONTHLY_SUPPLY,
        unallocated_total_tokens: SEED_MONTHLY_SUPPLY * 18,
        cliff_started_at: 0,
        months_claimed: 0,
        cliff_months_remaining: 6,
        vesting_months_remaining: 18,
        investor_count: 0,
        is_open: false,
    },
);

pub const INSTITUTIONAL_CATEGORY: (&[u8], InvestorCategoryData) = (
    b"institutional",
    InvestorCategoryData {
        monthly_allocation: INSTITUTIONAL_MONTHLY_SUPPLY,
        unallocated_total_tokens: INSTITUTIONAL_MONTHLY_SUPPLY * 24,
        cliff_started_at: 0,
        months_claimed: 0,
        cliff_months_remaining: 12,
        vesting_months_remaining: 24,
        investor_count: 0,
        is_open: true,
    },
);

pub const VGP_CATEGORY: (&[u8], InvestorCategoryData) = (
    b"vgp",
    InvestorCategoryData {
        monthly_allocation: VGP_MONTHLY_SUPPLY,
        unallocated_total_tokens: VGP_MONTHLY_SUPPLY * 24,
        cliff_started_at: 0,
        months_claimed: 0,
        cliff_months_remaining: 12,
        vesting_months_remaining: 24,
        investor_count: 0,
        is_open: true,
    },
);

pub const FOUNDERS_CATEGORY: (&[u8], InvestorCategoryData) = (
    b"founders",
    InvestorCategoryData {
        monthly_allocation: FOUNDERS_MONTHLY_SUPPLY,
        unallocated_total_tokens: FOUNDERS_MONTHLY_SUPPLY * 24,
        cliff_started_at: 0,
        months_claimed: 0,
        cliff_months_remaining: 12,
        vesting_months_remaining: 24,
        investor_count: 0,
        is_open: true,
    },
);

pub const MARKETING_CATEGORY: (&[u8], FunctionalCategoryData) = (
    b"marketing",
    FunctionalCategoryData {
        wallet: pubkey!("GSd6RQZ4o9AMpHeRYZEwcjZ9oAP1ZLAUeKbwbNdS2oJH"),
        monthly_allocation: MARKETING_MONTHLY_SUPPLY,
        cliff_started_at: 0,
        months_claimed: 0,
        cliff_months_remaining: 6,
        vesting_months_remaining: 36,
    },
);

pub const RESERVE_CATEGORY: (&[u8], FunctionalCategoryData) = (
    b"reserve",
    FunctionalCategoryData {
        wallet: pubkey!("GSd6RQZ4o9AMpHeRYZEwcjZ9oAP1ZLAUeKbwbNdS2oJH"),
        monthly_allocation: RESERVE_MONTHLY_SUPPLY,
        cliff_started_at: 0,
        months_claimed: 0,
        cliff_months_remaining: 0,
        vesting_months_remaining: 48,
    },
);

pub const LIQUIDITY_CATEGORY: (&[u8], FunctionalCategoryData) = (
    b"liquidity",
    FunctionalCategoryData {
        wallet: pubkey!("GSd6RQZ4o9AMpHeRYZEwcjZ9oAP1ZLAUeKbwbNdS2oJH"),
        monthly_allocation: LIQUIDITY_MONTHLY_SUPPLY,
        cliff_started_at: 0,
        months_claimed: 0,
        cliff_months_remaining: 0,
        vesting_months_remaining: 12,
    },
);
