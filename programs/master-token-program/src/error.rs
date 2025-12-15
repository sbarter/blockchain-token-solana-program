use anchor_lang::error_code;

#[error_code]
pub enum ErrorCode {
    #[msg("One or more category PDAs/ATAs failed to initialize")]
    InitError,
    #[msg("TGE already happened or wrong mint authority")]
    MintAuthorityMismatch,
    #[msg("One of associated token accounts is wrong")]
    AtaMismatch,
}
