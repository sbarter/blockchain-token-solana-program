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

    /// Total category allocation minus total allocations of all investors.
    /// Used as the upper limit of allocation when adding new investors.
    pub unallocated_tokens_left: u64,
    /// Total monthly allocations of all investors in the category.
    /// For each month that the category claims for, this gets added to
    /// [`Self::allocated_unclaimed_tokens`].
    pub total_allocated_tokens_monthly: u64,
    /// The number of tokens that must stay in the category,
    /// because they are currently available for investors to claim.
    /// Used as the upper limit for manual category withdrawal.
    pub tokens_ready_for_claim: u64,

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
    pub monthly_allocation_in_base_units: u64,

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

pub fn investor_category_seed_is_valid(category_seed: &str) -> bool {
    ["preseed", "seed", "institutional", "vgp", "founders"].contains(&category_seed)
}

pub fn functional_category_seed_is_valid(category_seed: &str) -> bool {
    ["marketing", "reserve", "liquidity"].contains(&category_seed)
}

pub const PRE_SEED_CATEGORY: Category<InvestorCategoryData> = Category {
    seed: b"preseed",
    data: InvestorCategoryData {
        monthly_allocation: PRESEED_MONTHLY_SUPPLY,
        unallocated_tokens_left: PRESEED_MONTHLY_SUPPLY * 24,
        total_allocated_tokens_monthly: 0,
        tokens_ready_for_claim: 0,
        cliff_started_at: 0,
        months_claimed: 0,
        cliff_months_remaining: 12,
        vesting_months_remaining: 24,
        investor_count: 0,
        is_open: false,
    },
    pre_investors: 68,
};

pub const SEED_CATEGORY: Category<InvestorCategoryData> = Category {
    seed: b"seed",
    data: InvestorCategoryData {
        monthly_allocation: SEED_MONTHLY_SUPPLY,
        unallocated_tokens_left: SEED_MONTHLY_SUPPLY * 18,
        total_allocated_tokens_monthly: 0,
        tokens_ready_for_claim: 0,
        cliff_started_at: 0,
        months_claimed: 0,
        cliff_months_remaining: 6,
        vesting_months_remaining: 18,
        investor_count: 0,
        is_open: false,
    },
    pre_investors: 96,
};

pub const INSTITUTIONAL_CATEGORY: Category<InvestorCategoryData> = Category {
    seed: b"institutional",
    data: InvestorCategoryData {
        monthly_allocation: INSTITUTIONAL_MONTHLY_SUPPLY,
        unallocated_tokens_left: INSTITUTIONAL_MONTHLY_SUPPLY * 24,
        total_allocated_tokens_monthly: 0,
        tokens_ready_for_claim: 0,
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
        unallocated_tokens_left: VGP_MONTHLY_SUPPLY * 24,
        total_allocated_tokens_monthly: 0,
        tokens_ready_for_claim: 0,
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
        unallocated_tokens_left: FOUNDERS_MONTHLY_SUPPLY * 24,
        total_allocated_tokens_monthly: 0,
        tokens_ready_for_claim: 0,
        cliff_started_at: 0,
        months_claimed: 0,
        cliff_months_remaining: 12,
        vesting_months_remaining: 24,
        investor_count: 0,
        is_open: true,
    },
    pre_investors: 0,
};

pub const MARKETING_CATEGORY: Category<FunctionalCategoryData> = Category {
    seed: b"marketing",
    data: FunctionalCategoryData {
        wallet: pubkey!("2GRnFCAkd8Smm8uJ2zFhZQgjCPgi341MzU9FS2U3De2q"),
        monthly_allocation_in_base_units: MARKETING_MONTHLY_SUPPLY,
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
        wallet: pubkey!("3kGsEXbQxWjNoVTZ7og1CVivPkBxAjtBPJYuSUn69eWi"),
        monthly_allocation_in_base_units: RESERVE_MONTHLY_SUPPLY,
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
        wallet: pubkey!("6RQboL2DeTM8jUQubgCYLSHZMJSLbUTRQZTEL2jjDa1M"),
        monthly_allocation_in_base_units: LIQUIDITY_MONTHLY_SUPPLY,
        cliff_started_at: 0,
        months_claimed: 0,
        cliff_months_remaining: 0,
        vesting_months_remaining: 12,
    },
    pre_investors: 0,
};
