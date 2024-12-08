use crate::{
    helpers::{extract_access_list, extract_gas_and_output},
    AlloyCacheDB,
};
use alloy_sol_types::{sol, SolCall, SolValue};
use anyhow::Result;
use revm::{
    primitives::{AccessList, Address, TxKind, U256},
    Evm,
};

sol! {
    #[allow(missing_docs)]
    #[sol(rpc)]
    interface IRouter {

        #[derive(Debug, PartialEq, Eq)]
        function swapExactTokensForTokens(uint amountIn, uint amountOutMin, address[] calldata path,  address to, uint deadline) external returns (uint[] memory amounts);
    }
}

#[derive(Debug)]
pub struct Router {
    caller: Address,
    router: Address,
}

impl Router {
    pub fn new(caller: Address, router: Address) -> Self {
        Self { caller, router }
    }

    pub fn swap_exact_tokens_for_tokens(
        &self,
        amount_in: U256,
        amount_out_min: U256,
        path: Vec<Address>,
        to: Address,
        deadline: U256,
        database: &mut AlloyCacheDB,
    ) -> Result<(Vec<U256>, AccessList)> {
        let call = IRouter::swapExactTokensForTokensCall::new((
            amount_in,
            amount_out_min,
            path,
            to,
            deadline,
        ));
        let mut evm = Evm::builder()
            .with_db(&mut *database)
            .modify_tx_env(|tx| {
                tx.caller = self.caller;
                tx.transact_to = TxKind::Call(self.router);
                tx.data = call.abi_encode().into();
                tx.value = U256::from(0);
            })
            .build();

        let result = evm.transact()?;
        let (output, gas_used, gas_refunded) = extract_gas_and_output(&result.result)?;
        let output = <Vec<U256>>::abi_decode(&output, false)?;
        let access_list = extract_access_list(&result);

        println!("Router Swap - Gas used with no access list: {:?}", gas_used);
        println!(
            "Router Swap - Gas refunded with no access list: {:?}",
            gas_refunded
        );
        println!("Router Swap - Output: {:?}", output);

        drop(evm);

        let mut evm = Evm::builder()
            .with_db(database)
            .modify_tx_env(|tx| {
                tx.caller = self.caller;
                tx.transact_to = TxKind::Call(self.router);
                tx.data = call.abi_encode().into();
                tx.value = U256::from(0);
                tx.access_list = access_list.0.clone();
            })
            .build();

        let result = evm.transact_commit()?;
        let (output, gas_used, gas_refunded) = extract_gas_and_output(&result)?;
        let output = <Vec<U256>>::abi_decode(&output, false)?;

        println!("Router Swap - Gas used with access list: {:?}", gas_used);
        println!(
            "Router Swap - Gas refunded with access list: {:?}",
            gas_refunded
        );
        println!("Router Swap - Output: {:?}", output);

        Ok((output, access_list))
    }
}
