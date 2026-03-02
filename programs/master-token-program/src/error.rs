use anchor_lang::error_code;

#[error_code]
pub enum ErrorCode {
    #[msg("The Master multisig wallet must sign this transaction")]
    MasterMustSign,
    #[msg("One of associated token accounts provided is wrong")]
    AtaMismatch,
    #[msg("One of the provided functional category authority wallets is wrong")]
    FunctionalCategoryAuthority,

    // TGE
    #[msg("TGE already happened or wrong mint authority")]
    MintAuthorityMismatch,
    #[msg("TGE has not happened yet")]
    TgeNotHappened,
    #[msg("Closed categories have to have exactly the agreed amount of investors initialized before TGE")]
    UnintializedInvestors,

    #[msg("Category-level claim for this cycle has to happen first")]
    CategoryLevelUnclaimed,

    // Investors, wallets and allocation
    #[msg("Wrong investor index provided")]
    InvestorIndex,
    #[msg("Investor index exceeds the agreed amount of investors/members in a closed category")]
    ClosedCategoryExceed,
    #[msg(
        "Investor token allocation must not be 0. Make sure you have set the allocation correctly."
    )]
    InvestorAllocation,
    #[msg("Token vesting plan has been already completed. No new investors/members can be added")]
    VestingScheduleFinished,
    #[msg("Invalid category seed")]
    CategorySeed,
    #[msg("Category is closed. No investors/members can be added")]
    CategoryClosed,
    #[msg("In order to change wallet pubkey, provide the old (current) wallet pubkey")]
    OldPubkeyMismatch,
    #[msg("Unable to allocate or move this amount of tokens because of overallocation")]
    TooManyTokensAllocated,
    #[msg("Unable to move this amount of tokens because they are currently unavailable in the category")]
    TokensUnavailable,
    #[msg("Your balance is insufficient to deposit this amount of tokens")]
    BalanceInsufficient,
}
