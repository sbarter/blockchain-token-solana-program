use std::{str::FromStr, sync::Arc};

use anchor_client::{
    anchor_lang::AccountDeserialize,
    solana_sdk::{
        commitment_config::CommitmentConfig, pubkey::Pubkey, signature::Keypair, signer::Signer,
        system_program,
    },
    Client, Cluster, Program,
};
use master_token_program::states::{Investor, InvestorCategoryData};

use crate::{
    helpers::{
        derive_ata, derive_category_pda, derive_investor_pda, find_functional_category_ata,
        get_master_pda, ASSOCIATED_TOKEN_PROGRAM_ID, TOKEN_2022_PROGRAM_ID,
    },
    tx_sender::{self, RPC_URI},
};

pub struct MasterTokenProgram {
    payer: Arc<Keypair>,
    coder: Program<Arc<Keypair>>,
}

impl MasterTokenProgram {
    pub fn new(payer: Keypair) -> Self {
        let payer = Arc::new(payer);
        let coder = {
            let cluster = Cluster::from_str(RPC_URI).unwrap();
            let provider = Client::new_with_options(
                cluster,
                Arc::clone(&payer),
                CommitmentConfig::confirmed(),
            );
            provider.program(master_token_program::ID).unwrap()
        };
        Self { payer, coder }
    }

    pub async fn transfer_category_vestings(&self, mint: Pubkey) -> anyhow::Result<()> {
        let accounts = master_token_program::accounts::TransferCategoryVestings {
            master_pda: get_master_pda(),
            master_ata: derive_ata(get_master_pda(), mint),
            pre_seed_cat: derive_category_pda("preseed", mint),
            pre_seed_ata: derive_ata(derive_category_pda("preseed", mint), mint),
            seed_cat: derive_category_pda("seed", mint),
            seed_ata: derive_ata(derive_category_pda("seed", mint), mint),
            institutional_cat: derive_category_pda("institutional", mint),
            institutional_ata: derive_ata(derive_category_pda("institutional", mint), mint),
            vgp_cat: derive_category_pda("vgp", mint),
            vgp_ata: derive_ata(derive_category_pda("vgp", mint), mint),
            founders_cat: derive_category_pda("founders", mint),
            founders_ata: derive_ata(derive_category_pda("founders", mint), mint),
            marketing_cat: derive_category_pda("marketing", mint),
            marketing_ata: find_functional_category_ata(
                derive_category_pda("marketing", mint),
                mint,
            )
            .await?,
            reserve_cat: derive_category_pda("reserve", mint),
            reserve_ata: find_functional_category_ata(derive_category_pda("reserve", mint), mint)
                .await?,
            liquidity_cat: derive_category_pda("liquidity", mint),
            liquidity_ata: find_functional_category_ata(
                derive_category_pda("liquidity", mint),
                mint,
            )
            .await?,
            mint,
            token_program: TOKEN_2022_PROGRAM_ID,
            associated_token_program: ASSOCIATED_TOKEN_PROGRAM_ID,
            system_program: system_program::ID,
        };
        let ix = self
            .coder
            .request()
            .accounts(accounts)
            .args(master_token_program::instruction::TransferCategoryVestings)
            .instructions()?
            .remove(0);

        println!("Transferring tokens to categories...");
        let mut tx = tx_sender::build_versioned_tx_from_ixs(&[ix], &self.payer.pubkey()).await;
        tx_sender::add_signature(&mut tx, &self.payer);
        tx_sender::send_and_confirm_transaction(&tx).await?;
        println!("Transferred tokens to categories.");
        Ok(())
    }

    pub async fn investor_claim_tokens(
        &self,
        category_seed: &str,
        investor_id: u16,
        mint: Pubkey,
    ) -> anyhow::Result<()> {
        let investor_pda = derive_investor_pda(category_seed, investor_id, mint);
        let investor = {
            let investor_data = self.coder.rpc().get_account_data(&investor_pda)?;
            Investor::try_deserialize(&mut investor_data.as_ref())?
        };
        let accounts = master_token_program::accounts::InvestorClaimTokens {
            investor_pda,
            investor_ata: derive_ata(investor.wallet, mint),
            category: derive_category_pda(category_seed, mint),
            category_ata: derive_ata(derive_category_pda(category_seed, mint), mint),
            mint,
            token_program: TOKEN_2022_PROGRAM_ID,
            associated_token_program: ASSOCIATED_TOKEN_PROGRAM_ID,
            system_program: system_program::ID,
        };
        let ix = self
            .coder
            .request()
            .accounts(accounts)
            .args(master_token_program::instruction::InvestorClaimTokens {
                category_seed: category_seed.to_string(),
                investor_index: investor_id,
            })
            .instructions()?
            .remove(0);

        let mut tx = tx_sender::build_versioned_tx_from_ixs(&[ix], &self.payer.pubkey()).await;
        tx_sender::add_signature(&mut tx, &self.payer);
        tx_sender::send_and_confirm_transaction(&tx).await?;
        println!("Transferred tokens to investor: {category_seed}:{investor_id}");
        Ok(())
    }

    pub async fn claim_for_all_investors(&self, mint: Pubkey) -> anyhow::Result<()> {
        const CATEGORY_SEEDS: &[&str] = &["preseed", "seed", "institutional", "vgp", "founders"];

        for seed in CATEGORY_SEEDS.iter() {
            let category_data = self
                .coder
                .rpc()
                .get_account_data(&derive_category_pda(seed, mint))?;
            let category = InvestorCategoryData::try_deserialize(&mut category_data.as_ref())?;
            println!(
                "Found {} investors for {seed} category.",
                category.investor_count
            );
            for investor_id in 1..=category.investor_count {
                println!("Trying to claim for investor #{investor_id}...");
                self.investor_claim_tokens(seed, investor_id, mint).await?;
            }
        }

        Ok(())
    }
}
