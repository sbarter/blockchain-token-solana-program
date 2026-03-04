use anchor_lang::prelude::*;

#[allow(unused_imports)]
use instructions::*;

pub mod error;
pub mod instructions;
pub mod states;

#[cfg(feature = "local-testing")]
pub const LOCAL_TESTING: bool = true;
#[cfg(not(feature = "local-testing"))]
pub const LOCAL_TESTING: bool = false;

/// Master multisig wallet that authorizes the operations (unless LOCAL_TESTING).
pub const SBARTER_MULTISIG: Pubkey = pubkey!("HUp2467gcy1qBXNjFeaY4VpFyTMUgStMJQTmuFbyCnTx");

#[cfg(feature = "mainnet-testing")]
pub const SBT_METADATA_URL: &str =
    "https://developed-amaranth-skunk.myfilebase.com/ipfs/QmVhZPS7T3oCmL8GJio8gZ1msuQgMHqu9xnPmVfeQWtAz9";
#[cfg(not(feature = "mainnet-testing"))]
pub const SBT_METADATA_URL: &str =
    "https://ipfs.io/ipfs/Qmdgzq9Rj8nUHr5erKTfqq3J5rm5XrWPTi1ceJCZmezp7S";

pub const SBT_DECIMALS: u8 = 6;
pub const fn tokens(sbt: u64) -> u64 {
    sbt * 10u64.pow(SBT_DECIMALS as u32)
}

#[cfg(feature = "local-testing")]
pub const VESTING_MONTH: u64 = 10;
#[cfg(feature = "mainnet-testing")]
pub const VESTING_MONTH: u64 = 900;
#[cfg(all(not(feature = "mainnet-testing"), not(feature = "local-testing")))]
pub const VESTING_MONTH: u64 = 30 * 24 * 60 * 60;

pub const TOTAL_MINT_SUPPLY: u64 = tokens(25_000_000_000);

pub const MARKETING_LIQUID_SUPPLY: u64 = tokens(100_000_000);
pub const RESERVE_LIQUID_SUPPLY: u64 = 0;
pub const RESERVE_PADDING: u64 = 54; // gets immediately put in reserve to even out the math
pub const LIQUIDITY_LIQUID_SUPPLY: u64 = tokens(937_500_000);

pub const PRESEED_MONTHLY_SUPPLY: u64 = tokens(2_000_000_000) / 24;
pub const SEED_MONTHLY_SUPPLY: u64 = tokens(1_000_000_000) / 18;
pub const INSTITUTIONAL_MONTHLY_SUPPLY: u64 = tokens(6_000_000_000) / 24;
pub const VGP_MONTHLY_SUPPLY: u64 = tokens(5_000_000_000) / 24;
pub const MARKETING_MONTHLY_SUPPLY: u64 = (tokens(2_000_000_000) - MARKETING_LIQUID_SUPPLY) / 36;
pub const FOUNDERS_MONTHLY_SUPPLY: u64 = tokens(4_200_000_000) / 24;
pub const RESERVE_MONTHLY_SUPPLY: u64 = (tokens(2_925_000_000) - RESERVE_LIQUID_SUPPLY) / 48;
pub const LIQUIDITY_MONTHLY_SUPPLY: u64 = (tokens(1_875_000_000) - LIQUIDITY_LIQUID_SUPPLY) / 12;

/// Consteval assurance that all tokens eventually get distributed to categories.
const _: () = {
    const TOTAL_DISTRIBUTED: u64 = PRESEED_MONTHLY_SUPPLY * 24
        + SEED_MONTHLY_SUPPLY * 18
        + INSTITUTIONAL_MONTHLY_SUPPLY * 24
        + VGP_MONTHLY_SUPPLY * 24
        + MARKETING_MONTHLY_SUPPLY * 36
        + MARKETING_LIQUID_SUPPLY
        + FOUNDERS_MONTHLY_SUPPLY * 24
        + RESERVE_MONTHLY_SUPPLY * 48
        + RESERVE_LIQUID_SUPPLY
        + RESERVE_PADDING
        + LIQUIDITY_MONTHLY_SUPPLY * 12
        + LIQUIDITY_LIQUID_SUPPLY;
    assert!(
        TOTAL_DISTRIBUTED == TOTAL_MINT_SUPPLY,
        "{}",
        const_format::formatcp!(
            "{} SBT decimal units ({} decimals) difference with total supply",
            TOTAL_DISTRIBUTED.abs_diff(TOTAL_MINT_SUPPLY),
            SBT_DECIMALS
        )
    )
};

declare_id!("D4FEEraLy45Yz3zV2VHSi1Hm7DF9DddibSVzPrbK4UYm");
#[program]
pub mod sbarter_token_programs {

    use super::*;

    #[cfg(feature = "local-testing")]
    pub fn initialize_mint<'info>(
        ctx: Context<'_, '_, '_, 'info, InitializeMint<'info>>,
    ) -> Result<()> {
        instructions::init::mint::initialize_mint(ctx)
    }

    pub fn initialize_investor_categories<'info>(
        ctx: Context<'_, '_, '_, 'info, InitializeInvestorCategories<'info>>,
    ) -> Result<()> {
        instructions::init::investor_categories::initialize_investor(ctx)
    }

    pub fn initialize_functional_categories<'info>(
        ctx: Context<'_, '_, '_, 'info, InitializeFunctionalCategories<'info>>,
    ) -> Result<()> {
        instructions::init::functional_categories::initialize_functional(ctx)
    }

    pub fn tge<'info>(ctx: Context<'_, '_, '_, 'info, Tge<'info>>) -> Result<()> {
        instructions::tge::start_tge(ctx)
    }

    pub fn category_add_investor<'info>(
        ctx: Context<'_, '_, '_, 'info, AddInvestorToCategory<'info>>,
        category_seed: String,
        new_investor_index: u16,
        total_allocation_in_whole_sbts: u64,
    ) -> Result<()> {
        instructions::investor::add::add_investor_to_category(
            ctx,
            category_seed,
            new_investor_index,
            total_allocation_in_whole_sbts,
        )
    }

    pub fn category_transfer_vestings<'info>(
        ctx: Context<'_, '_, '_, 'info, TransferCategoryVestings<'info>>,
    ) -> Result<()> {
        instructions::category_claim::transfer_category_vestings(ctx)
    }

    pub fn category_change_manager_wallet<'info>(
        ctx: Context<'_, '_, '_, 'info, ChangeCategoryWallet<'info>>,
        category_seed: String,
    ) -> Result<()> {
        instructions::category::change_wallet::admin_change_functional_category_wallet(
            ctx,
            category_seed,
        )
    }

    pub fn category_withdraw<'info>(
        ctx: Context<'_, '_, '_, 'info, WithdrawCategoryTokens<'info>>,
        category_seed: String,
        amount_in_whole_sbts: u64,
    ) -> Result<()> {
        instructions::category::withdraw::withdraw_category_tokens(
            ctx,
            category_seed,
            amount_in_whole_sbts,
        )
    }

    pub fn category_deposit<'info>(
        ctx: Context<'_, '_, '_, 'info, DepositCategoryTokens<'info>>,
        category_seed: String,
        amount_in_whole_sbts: u64,
    ) -> Result<()> {
        instructions::category::deposit::deposit_category_tokens(
            ctx,
            category_seed,
            amount_in_whole_sbts,
        )
    }

    pub fn investor_claim_tokens<'info>(
        ctx: Context<'_, '_, '_, 'info, InvestorClaimTokens<'info>>,
        category_seed: String,
        investor_index: u16,
    ) -> Result<()> {
        instructions::investor_claim::investor_claim_tokens(ctx, category_seed, investor_index)
    }

    pub fn investor_change_wallet<'info>(
        ctx: Context<'_, '_, '_, 'info, ChangeInvestorWallet<'info>>,
        category_seed: String,
        investor_index: u16,
    ) -> Result<()> {
        instructions::investor::change_wallet::admin_change_investor_wallet(
            ctx,
            category_seed,
            investor_index,
        )
    }
}
