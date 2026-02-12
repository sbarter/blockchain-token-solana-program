use anchor_lang::prelude::*;

#[derive(Debug, InitSpace)]
#[account]
pub struct Investor {
    pub wallet: Pubkey,
    pub monthly_allocation_in_base_units: u64,

    pub months_claimed: u8,
    pub cliff_months_remaining: u8,
    pub vesting_months_remaining: u8,
}
