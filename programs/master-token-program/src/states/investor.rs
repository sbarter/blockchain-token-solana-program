use anchor_lang::prelude::*;

#[derive(Debug)]
#[account]
pub struct Investor {
    pub wallet: Pubkey,
    pub monthly_allocation: u64,
    pub first_month_skipped: bool,

    pub cliff_started_at: u64,
    pub months_claimed: u8,
    pub cliff_months_remaining: u8,
    pub vesting_months_remaining: u8,
}

impl Investor {
    pub const LEN: usize = 8 + 32 + 8 + 1 + 8 + 1 + 1 + 1;
}
