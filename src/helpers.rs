use alloy_sol_types::decode_revert_reason;
use anyhow::{anyhow, Result};
use revm::primitives::{
    AccessList, AccessListItem, Address, Bytes, ExecutionResult, Log, Output, ResultAndState, B256,
    U256,
};

use crate::AlloyCacheDB;

pub fn get_revert_message(revert_message: &Bytes) -> String {
    match decode_revert_reason(revert_message) {
        None => revert_message.to_string(),
        Some(message) => message,
    }
}

pub fn extract_gas_output_and_logs(
    result: &ExecutionResult,
) -> Result<(Bytes, Vec<Log>, u64, u64)> {
    match result {
        ExecutionResult::Success {
            output: Output::Call(value),
            gas_used,
            gas_refunded,
            logs,
            ..
        } => Ok((value.clone(), logs.clone(), *gas_used, *gas_refunded)),
        ExecutionResult::Revert { output, .. } => {
            Err(anyhow!("Reverted: {:?}", get_revert_message(output)))
        }
        result => Err(anyhow!("Execution failed: {result:?}")),
    }
}

pub fn extract_access_list(result: &ResultAndState) -> AccessList {
    let storages = result
        .state
        .iter()
        .filter(|(_, account)| !account.storage.is_empty())
        .map(|(&address, account)| AccessListItem {
            address,
            storage_keys: account
                .storage
                .keys()
                .map(|k| B256::from(k.to_be_bytes()))
                .collect(),
        })
        .collect::<Vec<_>>();

    AccessList::from(storages)
}

pub fn set_eth_balance(account: Address, amount: U256, database: &mut AlloyCacheDB) {
    let account = database.load_account(account).unwrap();
    account.info.balance = amount;
}
