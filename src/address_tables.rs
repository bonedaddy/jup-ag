use anyhow::Result;
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::address_lookup_table::state::LOOKUP_TABLE_META_SIZE;
use solana_sdk::address_lookup_table_account::AddressLookupTableAccount;
use solana_sdk::instruction::InstructionError;
use solana_sdk::{
    address_lookup_table::state::{AddressLookupTable, LookupTableMeta, ProgramState},
    pubkey::Pubkey,
};

pub async fn load_address_lookup_table<'a>(
    rpc: &RpcClient,
    tables: &[Pubkey],
) -> Result<Vec<AddressLookupTableAccount>> {
    let accounts = rpc.get_multiple_accounts(tables).await?;
    let accounts = accounts
        .into_iter()
        .enumerate()
        .filter_map(|(idx, acct)| {
            Some((LookupTable::deserialize(&acct?.data).ok()?, tables[idx]))
        })
        .map((|(lut, key)| {
            AddressLookupTableAccount { key: key, addresses: lut.addresses }
        }))
        .collect::<Vec<_>>();
    Ok(accounts)
}
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct LookupTable {
    pub meta: LookupTableMeta,
    pub addresses: Vec<Pubkey>,
}
impl LookupTable {
    /// Efficiently deserialize an address table without allocating
    /// for stored addresses.
    pub fn deserialize(data: &[u8]) -> Result<LookupTable> {
        let program_state: ProgramState =
            bincode::deserialize(data).map_err(|_| InstructionError::InvalidAccountData)?;

        let meta = match program_state {
            ProgramState::LookupTable(meta) => Ok(meta),
            ProgramState::Uninitialized => Err(InstructionError::UninitializedAccount),
        }?;

        let raw_addresses_data = data.get(LOOKUP_TABLE_META_SIZE..).ok_or({
            // Should be impossible because table accounts must
            // always be LOOKUP_TABLE_META_SIZE in length
            InstructionError::InvalidAccountData
        })?;
        let addresses: &[Pubkey] = bytemuck::try_cast_slice(raw_addresses_data).map_err(|_| {
            // Should be impossible because raw address data
            // should be aligned and sized in multiples of 32 bytes
            InstructionError::InvalidAccountData
        })?;

        Ok(LookupTable {
            meta,
            addresses: addresses.to_vec(),
        })
    }
}
