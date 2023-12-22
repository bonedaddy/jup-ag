use crate::address_tables::load_address_lookup_table;
use crate::quote_types::QuoteResponse;
use anyhow::anyhow;
use base64::engine::general_purpose::STANDARD as b64;
use base64::Engine;
use serde::{Deserialize, Serialize};
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::compute_budget::ComputeBudgetInstruction;
use solana_sdk::instruction::{AccountMeta, Instruction};
use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;
use std::sync::Arc;
pub const SWAP_BASE: &str = "https://quote-api.jup.ag/v6/swap-instructions";

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SwapRequest {
    pub user_public_key: String,
    pub wrap_and_unwrap_sol: bool,
    pub use_shared_accounts: bool,
    pub fee_account: Option<String>,
    pub compute_unit_price_micro_lamports: i64,
    pub as_legacy_transaction: bool,
    pub use_token_ledger: bool,
    pub destination_token_account: Option<String>,
    pub quote_response: QuoteResponse,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SwapResponse {
    pub token_ledger_instruction: serde_json::Value,
    pub compute_budget_instructions: Vec<ComputeBudgetIx>,
    pub setup_instructions: Vec<SetupInstruction>,
    pub swap_instruction: SwapInstruction,
    //pub cleanup_instruction: CleanupInstruction,
    pub address_lookup_table_addresses: Vec<String>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ComputeBudgetIx {
    pub program_id: String,
    pub accounts: Vec<serde_json::Value>,
    pub data: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SetupInstruction {
    pub program_id: String,
    pub accounts: Vec<Account>,
    pub data: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Account {
    pub pubkey: String,
    pub is_signer: bool,
    pub is_writable: bool,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SwapInstruction {
    pub program_id: String,
    pub accounts: Vec<Account>,
    pub data: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CleanupInstruction {
    pub program_id: String,
    pub accounts: Vec<Account>,
    pub data: String,
}

impl SwapResponse {
    pub fn address_lookup_tables(&self) -> Vec<Pubkey> {
        self.address_lookup_table_addresses
            .iter()
            .filter_map(|addr| Pubkey::from_str(addr).ok())
            .collect::<Vec<_>>()
    }
    pub async fn new_v0_transaction(
        &self,
        rpc: &Arc<RpcClient>,
        payer: Pubkey,
        prio_fee: Option<u64>,
        cu_limit: Option<u32>,
    ) -> anyhow::Result<solana_sdk::message::v0::Message> {
        let num_instructions = usize::from(prio_fee.is_some())
            + usize::from(cu_limit.is_some())
            + self.setup_instructions.len()
            + 1; // 1 = swap tx

        let mut instructions = Vec::with_capacity(num_instructions);

        if let Some(prio_fee) = prio_fee {
            instructions.push(ComputeBudgetInstruction::set_compute_unit_price(prio_fee));
        }

        if let Some(cu_limit) = cu_limit {
            instructions.push(ComputeBudgetInstruction::set_compute_unit_limit(cu_limit));
        }
        let setup_ixs: Vec<Instruction> = self
            .setup_instructions
            .iter()
            .filter_map(|ix| ix.to_instruction().ok())
            .collect();
        instructions.extend_from_slice(&setup_ixs);
        instructions.push(self.swap_instruction.to_instruction()?);
        // omit cleanup
        let luts = self.address_lookup_tables();
        let luts = load_address_lookup_table(rpc, &luts).await?;
        let msg = solana_sdk::message::v0::Message::try_compile(
            &payer,
            &instructions,
            &luts,
            rpc.get_latest_blockhash().await?,
        )?;
        Ok(msg)
    }
}

impl SetupInstruction {
    pub fn to_instruction(&self) -> anyhow::Result<Instruction> {
        let ix_data = b64.decode(&self.data)?;
        let expected_size = self.accounts.len();
        let accounts: Vec<AccountMeta> = self
            .accounts
            .iter()
            .filter_map(|acct| {
                if acct.is_writable {
                    Some(AccountMeta::new(acct.pubkey.parse().ok()?, acct.is_signer))
                } else {
                    Some(AccountMeta::new_readonly(
                        acct.pubkey.parse().ok()?,
                        acct.is_signer,
                    ))
                }
            })
            .collect();
        if accounts.len() != expected_size {
            return Err(anyhow!("account parse failed"));
        }
        Ok(Instruction {
            program_id: self.program_id.parse()?,
            accounts,
            data: ix_data,
        })
    }
}

impl SwapInstruction {
    pub fn to_instruction(&self) -> anyhow::Result<Instruction> {
        let ix_data = b64.decode(&self.data)?;
        let expected_size = self.accounts.len();
        let accounts: Vec<AccountMeta> = self
            .accounts
            .iter()
            .filter_map(|acct| {
                if acct.is_writable {
                    Some(AccountMeta::new(acct.pubkey.parse().ok()?, acct.is_signer))
                } else {
                    Some(AccountMeta::new_readonly(
                        acct.pubkey.parse().ok()?,
                        acct.is_signer,
                    ))
                }
            })
            .collect();
        if accounts.len() != expected_size {
            return Err(anyhow!("account parse failed"));
        }
        Ok(Instruction {
            program_id: self.program_id.parse()?,
            accounts,
            data: ix_data,
        })
    }
}

impl CleanupInstruction {
    pub fn to_instruction(&self) -> anyhow::Result<Instruction> {
        let ix_data = b64.decode(&self.data)?;
        let expected_size = self.accounts.len();
        let accounts: Vec<AccountMeta> = self
            .accounts
            .iter()
            .filter_map(|acct| {
                if acct.is_writable {
                    Some(AccountMeta::new(acct.pubkey.parse().ok()?, acct.is_signer))
                } else {
                    Some(AccountMeta::new_readonly(
                        acct.pubkey.parse().ok()?,
                        acct.is_signer,
                    ))
                }
            })
            .collect();
        if accounts.len() != expected_size {
            return Err(anyhow!("account parse failed"));
        }
        Ok(Instruction {
            program_id: self.program_id.parse()?,
            accounts,
            data: ix_data,
        })
    }
}
