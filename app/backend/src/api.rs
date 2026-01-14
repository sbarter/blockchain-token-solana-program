use std::{str::FromStr, sync::Arc};

use anchor_client::{
    anchor_lang::{solana_program::example_mocks::solana_sdk::system_program, AccountDeserialize},
    solana_sdk::{
        commitment_config::CommitmentConfig, pubkey::Pubkey, signature::Keypair, signer::Signer,
    },
    Client, Cluster, Program,
};
use master_token_program::states::InvestorCategoryData;

use crate::{
    helpers::{
        derive_ata, derive_category_pda, derive_investor_pda, get_master_pda,
        ASSOCIATED_TOKEN_PROGRAM_ID, TOKEN_2022_PROGRAM_ID,
    },
    tx_sender::{self, RPC_URI},
};

pub struct MasterTokenProgram {
    payer: Arc<Keypair>,
}

impl MasterTokenProgram {
    pub fn new(payer: Keypair) -> Self {
        Self {
            payer: Arc::new(payer),
        }
    }

    pub async fn get_coder(&self) -> Program<Arc<Keypair>> {
        let cluster = Cluster::from_str(RPC_URI).unwrap();
        let provider = Client::new_with_options(
            cluster,
            Arc::clone(&self.payer),
            CommitmentConfig::confirmed(),
        );
        provider.program(master_token_program::ID).unwrap()
    }

    pub async fn transfer_category_vestings(&self, mint: Pubkey) -> anyhow::Result<()> {
        let coder = self.get_coder().await;
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
            marketing_ata: derive_ata(derive_category_pda("marketing", mint), mint),
            reserve_cat: derive_category_pda("reserve", mint),
            reserve_ata: derive_ata(derive_category_pda("reserve", mint), mint),
            liquidity_cat: derive_category_pda("liquidity", mint),
            liquidity_ata: derive_ata(derive_category_pda("liquidity", mint), mint),
            mint,
            token_program: TOKEN_2022_PROGRAM_ID,
            associated_token_program: ASSOCIATED_TOKEN_PROGRAM_ID,
            system_program: system_program::ID,
        };
        let ix = coder.request().accounts(accounts).instructions()?.remove(0);

        let tx = tx_sender::build_versioned_tx_from_ixs(&[ix], &self.payer.pubkey()).await;
        tx_sender::send_and_confirm_transaction(&tx).await?;
        Ok(())
    }

    pub async fn investor_claim_tokens(
        &self,
        category_seed: &str,
        investor_id: u16,
        mint: Pubkey,
    ) -> anyhow::Result<()> {
        let coder = self.get_coder().await;
        let accounts = master_token_program::accounts::InvestorClaimTokens {
            investor_pda: derive_investor_pda(category_seed, investor_id, mint),
            investor_ata: derive_ata(derive_investor_pda(category_seed, investor_id, mint), mint),
            category: derive_category_pda(category_seed, mint),
            category_ata: derive_ata(derive_category_pda(category_seed, mint), mint),
            mint,
            token_program: TOKEN_2022_PROGRAM_ID,
            associated_token_program: ASSOCIATED_TOKEN_PROGRAM_ID,
            system_program: system_program::ID,
        };
        let ix = coder
            .request()
            .accounts(accounts)
            .args(master_token_program::instruction::InvestorClaimTokens {
                category_seed: category_seed.to_string(),
                investor_index: investor_id,
            })
            .instructions()?
            .remove(0);

        let tx = tx_sender::build_versioned_tx_from_ixs(&[ix], &self.payer.pubkey()).await;
        tx_sender::send_and_confirm_transaction(&tx).await?;
        Ok(())
    }

    pub async fn claim_for_all_investors(&self, mint: Pubkey) -> anyhow::Result<()> {
        let coder = self.get_coder().await;
        const CATEGORY_SEEDS: &[&str] = &["preseed", "seed", "institutional", "vgp", "founders"];

        for seed in CATEGORY_SEEDS.iter() {
            let category_data = coder
                .rpc()
                .get_account_data(&derive_category_pda(seed, mint))
                .unwrap();
            let category =
                InvestorCategoryData::try_deserialize(&mut category_data.as_ref()).unwrap();
            for investor_id in 1..=category.investor_count {
                self.investor_claim_tokens(seed, investor_id, mint)
                    .await
                    .unwrap();
            }
        }

        Ok(())
    }
}
