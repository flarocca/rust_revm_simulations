use alloy_sol_types::{sol, SolCall, SolValue};
use anyhow::{anyhow, Result};
use revm::{
    primitives::{keccak256, AccessList, Address, Bytes, TxKind, U256},
    Evm,
};

use crate::commons::helpers::{extract_access_list, extract_gas_output_and_logs};

use super::helpers::AlloyCacheDB;

sol! {
    #[allow(missing_docs)]
    #[sol(rpc)]
    interface IErc20 {
        #[derive(Debug)]
        event Transfer(address indexed from, address indexed to, uint256 value);

        #[derive(Debug)]
        event Approval(address indexed owner, address indexed spender, uint256 value);

        #[derive(Debug)]
        function balanceOf(address account) external returns (uint256);

        #[derive(Debug)]
        function allowance(address owner, address spender) public view virtual returns (uint256);

        #[derive(Debug)]
        function name() public returns (string);

        #[derive(Debug)]
        function symbol() public returns (string);

        #[derive(Debug)]
        function decimals() public returns (uint8);

        #[derive(Debug)]
        function transfer(address destination, uint value) public returns (bool);

        #[derive(Debug)]
        function transferFrom(address src, address dst, uint wad) public returns (bool);

        #[derive(Debug)]
        function approve(address spender, uint wad) public returns (bool);
    }
}

pub struct Erc20 {
    caller: Address,
    token: Address,
}

impl Erc20 {
    pub fn new(caller: Address, token: Address) -> Self {
        Self { caller, token }
    }

    pub fn balance_of(
        &self,
        account: Address,
        database: &mut AlloyCacheDB,
    ) -> Result<(U256, AccessList)> {
        let calldata = Bytes::from(IErc20::balanceOfCall::new((account,)).abi_encode());
        let mut evm = Evm::builder()
            .with_db(database)
            .modify_tx_env(|tx| {
                tx.caller = self.caller;
                tx.transact_to = TxKind::Call(self.token);
                tx.data = calldata;
                tx.value = U256::from(0);
            })
            .build();

        let result = evm.transact()?;
        let (output, _, _, _) = extract_gas_output_and_logs(&result.result)?;
        let access_list = extract_access_list(&result);
        let output = <U256>::abi_decode(&output, false)?;

        Ok((output, access_list))
    }

    #[allow(dead_code)]
    pub fn allowance(
        &self,
        owner: Address,
        spender: Address,
        database: &mut AlloyCacheDB,
    ) -> Result<(U256, AccessList)> {
        let calldata = Bytes::from(IErc20::allowanceCall::new((owner, spender)).abi_encode());
        let mut evm = Evm::builder()
            .with_db(database)
            .modify_tx_env(|tx| {
                tx.caller = self.caller;
                tx.transact_to = TxKind::Call(self.token);
                tx.data = calldata;
                tx.value = U256::from(0);
            })
            .build();

        let result = evm.transact()?;
        let (output, _, _, _) = extract_gas_output_and_logs(&result.result)?;
        let output = <U256>::abi_decode(&output, false)?;
        let access_list = extract_access_list(&result);

        Ok((output, access_list))
    }

    pub fn approve(
        &self,
        spender: Address,
        amount: U256,
        database: &mut AlloyCacheDB,
    ) -> Result<(bool, AccessList)> {
        let calldata = Bytes::from(IErc20::approveCall::new((spender, amount)).abi_encode());
        let mut evm = Evm::builder()
            .with_db(&mut *database)
            .modify_tx_env(|tx| {
                tx.caller = self.caller;
                tx.transact_to = TxKind::Call(self.token);
                tx.data = calldata.clone();
                tx.value = U256::from(0);
            })
            .build();

        // Checkpoint:
        // First execution ensures that the transaction is valid and also allows us to get the
        // storage slots that were touched. At this point it is important not to commit the
        // changes to the database.
        let result = evm.transact()?;
        let (output, _, gas_used, gas_refunded) = extract_gas_output_and_logs(&result.result)?;
        let output = <bool>::abi_decode(&output, false)?;

        // FIX: These access list must be incorrectly calculated as the consume more gas than the
        // original tx
        let access_list = extract_access_list(&result);

        drop(evm);

        println!(
            "ERC20 Approve - Gas used with no access list: {:?}",
            gas_used
        );
        println!(
            "ERC20 Approve - Gas refunded with no access list: {:?}",
            gas_refunded
        );
        println!("ERC20 Approve - Output: {:?}", output);

        let mut evm = Evm::builder()
            .with_db(database)
            .modify_tx_env(|tx| {
                tx.caller = self.caller;
                tx.transact_to = TxKind::Call(self.token);
                tx.data = calldata;
                tx.value = U256::from(0);
                tx.access_list = access_list.0.clone();
            })
            .build();

        // Checkpoint:
        // We execute the same transaction but this time including the access list and committing
        // the changes to the database. It is important to commit the changes or the swap via the
        // router will fail as it will try to perform a `transfer_from`.
        //
        // TODO: We could add an extra flag to indicate whether to commit or not.
        // TODO: We could include an extra validation so ensure that the output on the first
        // transaction matches the output on the second transaction.
        let result = evm.transact_commit()?;
        let (output, _, gas_used, gas_refunded) = extract_gas_output_and_logs(&result)?;
        let output = <bool>::abi_decode(&output, false)?;

        println!("ERC20 Approve - Gas used with access list: {:?}", gas_used);
        println!(
            "ERC20 Approve - Gas refunded with access list: {:?}",
            gas_refunded
        );
        println!("ERC20 Approve - Output: {:?}", output);

        Ok((output, access_list))
    }

    pub fn transfer(
        &self,
        destination: Address,
        amount: U256,
        database: &mut AlloyCacheDB,
    ) -> Result<(bool, AccessList)> {
        let calldata = Bytes::from(IErc20::transferCall::new((destination, amount)).abi_encode());
        let mut evm = Evm::builder()
            .with_db(&mut *database)
            .modify_tx_env(|tx| {
                tx.caller = self.caller;
                tx.transact_to = TxKind::Call(self.token);
                tx.data = calldata.clone();
                tx.value = U256::from(0);
            })
            .build();

        // Checkpoint:
        // First execution ensures that the transaction is valid and also allows us to get the
        // storage slots that were touched. At this point it is important not to commit the
        // changes to the database.
        let result = evm.transact()?;
        let (output, _, gas_used, gas_refunded) = extract_gas_output_and_logs(&result.result)?;
        let output = <bool>::abi_decode(&output, false)?;

        // FIX: These access list must be incorrectly calculated as the consume more gas than the
        // original tx
        let access_list = extract_access_list(&result);

        drop(evm);

        println!(
            "ERC20 Transfer - Gas used with no access list: {:?}",
            gas_used
        );
        println!(
            "ERC20 Transfer - Gas refunded with no access list: {:?}",
            gas_refunded
        );
        println!("ERC20 Transfer - Output: {:?}", output);

        let mut evm = Evm::builder()
            .with_db(database)
            .modify_tx_env(|tx| {
                tx.caller = self.caller;
                tx.transact_to = TxKind::Call(self.token);
                tx.data = calldata;
                tx.value = U256::from(0);
                tx.access_list = access_list.0.clone();
            })
            .build();

        // Checkpoint:
        // We execute the same transaction but this time including the access list and committing
        // the changes to the database. It is important to commit the changes or the swap via the
        // router will fail as it will try to perform a `transfer_from`.
        //
        // TODO: We could add an extra flag to indicate whether to commit or not.
        // TODO: We could include an extra validation so ensure that the output on the first
        // transaction matches the output on the second transaction.
        let result = evm.transact_commit()?;
        let (output, _, gas_used, gas_refunded) = extract_gas_output_and_logs(&result)?;
        let output = <bool>::abi_decode(&output, false)?;

        println!("ERC20 Transfer - Gas used with access list: {:?}", gas_used);
        println!(
            "ERC20 Transfer - Gas refunded with access list: {:?}",
            gas_refunded
        );
        println!("ERC20 Transfer - Output: {:?}", output);

        Ok((output, access_list))
    }

    pub fn set_balance(
        &self,
        account: Address,
        amount: U256,
        database: &mut AlloyCacheDB,
    ) -> Result<()> {
        let slot = self.get_balance_slot(account, database)?;
        let _ = database.insert_account_storage(self.token, slot, amount);

        Ok(())
    }

    fn get_balance_slot(&self, account: Address, database: &mut AlloyCacheDB) -> Result<U256> {
        let (_, touched_storage) = self.balance_of(account, database)?;
        let touched_storage = touched_storage
            .iter()
            .find(|&storage| storage.address == self.token)
            .unwrap();

        for i in 0..50 {
            let slot = keccak256((account, i).abi_encode());

            if touched_storage.storage_keys.iter().any(|key| key == &slot) {
                return Ok(U256::from_be_bytes(slot.into()));
            };
        }

        Err(anyhow!("Storage slot not found"))
    }
}
