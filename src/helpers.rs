use alloy_sol_types::decode_revert_reason;
use anyhow::{anyhow, Result};
use revm::primitives::{
    AccessList, AccessListItem, Bytes, ExecutionResult, Output, ResultAndState, B256,
};

pub fn get_revert_message(revert_message: &Bytes) -> String {
    match decode_revert_reason(revert_message) {
        None => revert_message.to_string(),
        Some(message) => message,
    }
}

pub fn extract_gas_and_output(result: &ExecutionResult) -> Result<(Bytes, u64, u64)> {
    match result {
        ExecutionResult::Success {
            output: Output::Call(value),
            gas_used,
            gas_refunded,
            ..
        } => Ok((value.clone(), *gas_used, *gas_refunded)),
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
