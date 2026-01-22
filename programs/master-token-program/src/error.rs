use anchor_lang::error_code;

#[error_code]
pub enum ErrorCode {
    #[msg("TGE already happened or wrong mint authority")]
    MintAuthorityMismatch,
    #[msg("TGE has not happened yet")]
    TgeNotHappened,
    #[msg("One of associated token accounts is wrong")]
    AtaMismatch,
    #[msg("Wrong investor index provided")]
    InvestorIndex,
    #[msg("In order to change wallet pubkey, provide the old (current) wallet pubkey")]
    OldPubkeyMismatch,
    #[msg("Invalid category seed")]
    CategorySeed,
    #[msg("Unable to allocate or move this amount of tokens because of overallocation")]
    TooManyTokensAllocated,
    #[msg("Unable to move this amount of tokens because they are currently unavailable in the category")]
    TokensUnavailable,
    #[msg("Category is closed. No investors can be added")]
    CategoryClosed,
    #[msg("Closed categories have to have exactly the agreed amount of investors initialized before TGE")]
    UnintializedInvestors,
}
