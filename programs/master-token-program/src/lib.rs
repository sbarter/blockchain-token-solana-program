use anchor_lang::prelude::*;

#[allow(unused_imports)]
use category::*;
#[allow(unused_imports)]
use instructions::*;

pub mod category;
pub mod error;
pub mod instructions;

pub const MASTER_WALLET: Pubkey = pubkey!("GSd6RQZ4o9AMpHeRYZEwcjZ9oAP1ZLAUeKbwbNdS2oJH");

pub const SBT_DECIMALS: u32 = 6;
pub const fn units(sbt: u64) -> u64 {
    sbt * 10u64.pow(SBT_DECIMALS)
}

pub const TOTAL_MINT_SUPPLY: u64 = units(25_000_000_000);

pub const MARKETING_LIQUID_SUPPLY: u64 = units(100_000_000);
pub const RESERVE_LIQUID_SUPPLY: u64 = units(0); // TODO:
pub const LIQUIDITY_LIQUID_SUPPLY: u64 = units(937_500_000);

pub const PRESEED_MONTHLY_SUPPLY: u64 = units(2_000_000_000) / 24;
pub const SEED_MONTHLY_SUPPLY: u64 = units(1_000_000_000) / 18;
pub const INSTITUTIONAL_MONTHLY_SUPPLY: u64 = units(6_000_000_000) / 24;
pub const VGP_MONTHLY_SUPPLY: u64 = units(5_000_000_000) / 24;
pub const MARKETING_MONTHLY_SUPPLY: u64 = (units(2_000_000_000) - MARKETING_LIQUID_SUPPLY) / 36;
pub const FOUNDERS_MONTHLY_SUPPLY: u64 = units(4_200_000_000) / 24;
pub const RESERVE_MONTHLY_SUPPLY: u64 = (units(2_925_000_000) - RESERVE_LIQUID_SUPPLY) / 48;
pub const LIQUIDITY_MONTHLY_SUPPLY: u64 = (units(1_875_000_000) - LIQUIDITY_LIQUID_SUPPLY) / 12;

declare_id!("Hvpe662GeFcr5oVsjhvFZ2dyfuVtHCVVmcjBU6ozQYzE");
#[program]
pub mod sbarter_token_programs {
    use super::*;

    pub fn initialize<'info>(ctx: Context<'_, '_, '_, 'info, Initialize<'info>>) -> Result<()> {
        instructions::init::initialize(ctx)
    }

    pub fn tge<'info>(ctx: Context<'_, '_, '_, 'info, Tge<'info>>) -> Result<()> {
        // Mint tokens, set authority to None, transfer to the master wallet signer.
        // Transfer 100M SBT to the marketing category.
        // Transfer 937.5M to the liquidity category.
        // Create a Switchboard job with the TGE timestamp-relative trigger?
        instructions::tge::start_tge(ctx)
    }

    pub fn transfer_category_vestings<'info>(
        ctx: Context<'_, '_, '_, 'info, TransferCategoryVestings<'info>>,
    ) -> Result<()> {
        // Triggered by a Switchboard cron-job or similar.
        // 30-days has passed (does switchboard sign to verify the claim?),
        // transfer monthly portions of SBT to each category,
        // invoke their distribution instructions?
        //
        // but how would the cliff work?
        instructions::category::transfer_vestings(ctx)
    }
}
