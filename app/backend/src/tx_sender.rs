use std::time::Duration;

use anchor_client::{
    solana_client::{nonblocking::rpc_client::RpcClient, rpc_config::RpcSendTransactionConfig},
    solana_sdk::{
        commitment_config::CommitmentConfig,
        compute_budget::ComputeBudgetInstruction,
        instruction::Instruction,
        message::{v0::Message, VersionedMessage},
        pubkey::Pubkey,
        signature::Signature,
        signer::Signer,
        transaction::VersionedTransaction,
    },
};
use tokio::time::sleep;

pub const RPC_URI: &str = "https://devnet.helius-rpc.com/?api-key=REDACTED";

pub async fn build_versioned_tx_from_ixs(
    ixs: &[Instruction],
    fee_payer: &Pubkey,
) -> VersionedTransaction {
    let rpc_client = RpcClient::new(RPC_URI.to_string());
    let budget_instruction = ComputeBudgetInstruction::set_compute_unit_limit(1_000_000);

    let blockhash = rpc_client.get_latest_blockhash().await.unwrap();
    let msg = VersionedMessage::V0(
        Message::try_compile(
            fee_payer,
            &[&[budget_instruction], ixs].concat(),
            &[],
            blockhash,
        )
        .unwrap(),
    );
    VersionedTransaction {
        signatures: vec![Signature::default()],
        message: msg,
    }
}

pub fn add_signature(tx: &mut VersionedTransaction, signer: impl Signer) {
    let msg_bytes = tx.message.serialize();
    tx.signatures[0] = signer.sign_message(&msg_bytes);
}

pub async fn send_and_confirm_transaction(
    tx: &VersionedTransaction,
) -> anyhow::Result<anchor_client::solana_sdk::signature::Signature> {
    let rpc_client = RpcClient::new_with_timeout_and_commitment(
        RPC_URI.to_string(),
        Duration::from_secs(30),
        CommitmentConfig::confirmed(),
    );

    let send_cfg = RpcSendTransactionConfig {
        skip_preflight: false,
        max_retries: Some(5),
        ..Default::default()
    };

    let mut last_err = None;
    for attempt in 0..5 {
        match rpc_client.send_transaction_with_config(tx, send_cfg).await {
            Ok(sig) => {
                for _ in 0..50 {
                    if rpc_client.confirm_transaction(&sig).await? {
                        return Ok(sig);
                    }
                    sleep(Duration::from_millis(500)).await;
                }

                return Err(anyhow::anyhow!(
                    "transaction sent but not confirmed: {}",
                    sig
                ));
            }
            Err(err) => {
                last_err = Some(err);
                let backoff = Duration::from_millis(500 * (attempt + 1) as u64);
                sleep(backoff).await;
            }
        }
    }

    Err(last_err
        .map(|e| anyhow::anyhow!(e))
        .unwrap_or_else(|| anyhow::anyhow!("transaction failed")))
}
