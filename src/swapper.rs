use std::sync::Arc;

use solana_client::{nonblocking::rpc_client::RpcClient, rpc_config::RpcSendTransactionConfig};
use solana_sdk::{signature::{Keypair, Signer, Signature}, transaction::VersionedTransaction, message::VersionedMessage};
use anyhow::{Result, anyhow, Context};
use crate::swap_types::SwapResponse;

#[derive(Clone)]
pub struct Swapper {
    pub rpc: Arc<RpcClient>,
    pub keypair_bytes: [u8; 64]
}

impl Swapper {
    pub fn new(rpc: Arc<RpcClient>, keypair: Keypair) -> Swapper {
        Self {
            rpc,
            keypair_bytes: keypair.to_bytes(),
        }
    }
    pub async fn new_swap(self: &Arc<Self>, swap_response: SwapResponse, skip_preflight: bool, retries: usize) -> Result<Signature> {
        let kp = Keypair::from_bytes(&self.keypair_bytes)?;
        let v0_msg = swap_response.new_v0_transaction(&self.rpc, kp.pubkey(), Some(prio_fee(0.001)), Some(1_000_000)).await?;
        let v_tx = VersionedTransaction::try_new(
            VersionedMessage::V0(v0_msg),
            &vec![&kp]
        )?;
        match self.rpc.send_transaction_with_config(
            &v_tx,
            RpcSendTransactionConfig {
                skip_preflight,
                max_retries: Some(retries),
                ..Default::default()
            }
        ).await {
            Ok(sig) => {
                Ok(sig)
            }
            Err(err) => return Err(anyhow!("failed execute swap {err:#?}"))
        }
    }
    pub fn keypair(self: &Arc<Self>) -> Keypair {
        // if this fails something fucked up
        Keypair::from_bytes(&self.keypair_bytes).unwrap()
    }
}

pub fn prio_fee(input: f64) -> u64 {
    spl_token::ui_amount_to_amount(input, 9)
}