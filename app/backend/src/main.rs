use std::{env::home_dir, time::Duration};

use anchor_client::solana_sdk::{pubkey::Pubkey, signature::Keypair, signer::EncodableKey};
use tokio::time;

use crate::api::MasterTokenProgram;

pub mod api;
pub mod helpers;
pub mod tx_sender;

const MINT: Pubkey = Pubkey::from_str_const("DDuFoNdPcjo7i1KPpjRGW2qwjg2KixPgdgPEu4tDbdGs");

#[tokio::main]
async fn main() {
    println!("Program ID: {}", master_token_program::ID);
    println!("Master PDA: {}", helpers::get_master_pda());
    println!(
        "Master ATA: {}",
        helpers::derive_ata(helpers::get_master_pda(), MINT)
    );
    let keypair =
        Keypair::read_from_file(home_dir().unwrap().join(".config/solana/id.json")).unwrap();
    let program = MasterTokenProgram::new(keypair);
    for _ in 0..10 {
        program.transfer_category_vestings(MINT).await.unwrap();
        // program.claim_for_all_investors(MINT).await.unwrap();
        time::sleep(Duration::from_secs(10)).await;
    }
}
