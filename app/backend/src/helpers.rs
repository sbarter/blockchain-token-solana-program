use anchor_client::{
    anchor_lang::AccountDeserialize, solana_client::nonblocking::rpc_client::RpcClient,
    solana_sdk::pubkey::Pubkey,
};
use master_token_program::states::FunctionalCategoryData;

use crate::tx_sender::RPC_URI;

pub const TOKEN_2022_PROGRAM_ID: Pubkey =
    Pubkey::from_str_const("TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb");
pub const ASSOCIATED_TOKEN_PROGRAM_ID: Pubkey =
    Pubkey::from_str_const("ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL");

pub fn get_master_pda() -> Pubkey {
    Pubkey::find_program_address(&[b"master"], &master_token_program::ID).0
}

pub fn derive_category_pda(category_seed: &str, mint: Pubkey) -> Pubkey {
    Pubkey::find_program_address(
        &[category_seed.as_bytes(), &mint.to_bytes()],
        &master_token_program::ID,
    )
    .0
}

pub fn derive_investor_pda(category_seed: &str, investor_id: u16, mint: Pubkey) -> Pubkey {
    Pubkey::find_program_address(
        &[
            category_seed.as_bytes(),
            &investor_id.to_le_bytes(),
            &mint.to_bytes(),
        ],
        &master_token_program::ID,
    )
    .0
}

pub fn derive_ata(pubkey: Pubkey, mint: Pubkey) -> Pubkey {
    Pubkey::find_program_address(
        &[
            &pubkey.to_bytes(),
            &TOKEN_2022_PROGRAM_ID.to_bytes(),
            &mint.to_bytes(),
        ],
        &ASSOCIATED_TOKEN_PROGRAM_ID,
    )
    .0
}

pub async fn find_functional_category_ata(
    category_pda: Pubkey,
    mint: Pubkey,
) -> anyhow::Result<Pubkey> {
    let rpc = RpcClient::new(RPC_URI.to_string());
    let category = {
        let category_data = rpc.get_account_data(&category_pda).await?;
        FunctionalCategoryData::try_deserialize(&mut category_data.as_ref())?
    };
    Ok(derive_ata(category.wallet, mint))
}
