use anchor_lang::prelude::*;

use crate::{
    FOUNDERS_MONTHLY_SUPPLY, INSTITUTIONAL_MONTHLY_SUPPLY, LIQUIDITY_MONTHLY_SUPPLY,
    MARKETING_MONTHLY_SUPPLY, PRESEED_MONTHLY_SUPPLY, RESERVE_MONTHLY_SUPPLY, SEED_MONTHLY_SUPPLY,
    VGP_MONTHLY_SUPPLY,
};

#[derive(Debug)]
#[account]
pub struct CategoryData {
    pub monthly_allocation: u64,

    pub cliff_months_remaining: u8,
    pub vesting_months_remaining: u8,
    pub investor_count: u32,
    pub is_open: bool,
}

impl CategoryData {
    pub const LEN: usize = 8 + 8 + 1 + 1 + 4 + 1;
}

// NOTE: if these are PDAs, how do we make them accessible to the treasury manager? if they link to
// real wallets instead, either switchboard will have to sign ixs each time (probably impossible?)
// otherwise, we will need a set of instructions to manage remaining tokens

pub const PRE_SEED_CATEGORY: (&[u8], CategoryData) = (
    b"preseed",
    CategoryData {
        monthly_allocation: PRESEED_MONTHLY_SUPPLY,
        cliff_months_remaining: 12,
        vesting_months_remaining: 24,
        investor_count: 0,
        is_open: false,
    },
);

pub const SEED_CATEGORY: (&[u8], CategoryData) = (
    b"seed",
    CategoryData {
        monthly_allocation: SEED_MONTHLY_SUPPLY,
        cliff_months_remaining: 6,
        vesting_months_remaining: 18,
        investor_count: 0,
        is_open: false,
    },
);

pub const INSTITUTIONAL_CATEGORY: (&[u8], CategoryData) = (
    b"institutional",
    CategoryData {
        monthly_allocation: INSTITUTIONAL_MONTHLY_SUPPLY,
        cliff_months_remaining: 12,
        vesting_months_remaining: 24,
        investor_count: 0,
        is_open: true,
    },
);

pub const VGP_CATEGORY: (&[u8], CategoryData) = (
    b"vgp",
    CategoryData {
        monthly_allocation: VGP_MONTHLY_SUPPLY,
        cliff_months_remaining: 12,
        vesting_months_remaining: 24,
        investor_count: 0,
        is_open: true,
    },
);

pub const MARKETING_CATEGORY: (&[u8], CategoryData) = (
    b"marketing",
    CategoryData {
        monthly_allocation: MARKETING_MONTHLY_SUPPLY,
        cliff_months_remaining: 6,
        vesting_months_remaining: 36,
        investor_count: 0,
        is_open: true,
    },
);

pub const FOUNDERS_CATEGORY: (&[u8], CategoryData) = (
    b"founders",
    CategoryData {
        monthly_allocation: FOUNDERS_MONTHLY_SUPPLY,
        cliff_months_remaining: 12,
        vesting_months_remaining: 24,
        investor_count: 0,
        is_open: true,
    },
);

pub const RESERVE_CATEGORY: (&[u8], CategoryData) = (
    b"reserve",
    CategoryData {
        monthly_allocation: RESERVE_MONTHLY_SUPPLY,
        cliff_months_remaining: 0,
        vesting_months_remaining: 48,
        investor_count: 0,
        is_open: true,
    },
);

pub const LIQUIDITY_CATEGORY: (&[u8], CategoryData) = (
    b"liquidity",
    CategoryData {
        monthly_allocation: LIQUIDITY_MONTHLY_SUPPLY,
        cliff_months_remaining: 0,
        vesting_months_remaining: 12,
        investor_count: 0,
        is_open: true,
    },
);
