use alloy_sol_types::{sol, SolCall, SolEvent, SolValue};
use anyhow::Result;
use revm::{
    primitives::{AccessList, Address, Bytes, Log, TxKind, U256},
    Evm,
};

use crate::commons::helpers::{extract_access_list, extract_gas_output_and_logs, AlloyCacheDB};

sol! {
    #[allow(missing_docs)]
    #[sol(rpc)]
    interface IRouter {

        #[derive(Debug, PartialEq, Eq)]
        function swapExactTokensForTokens(uint amountIn, uint amountOutMin, address[] calldata path,  address to, uint deadline) external returns (uint[] memory amounts);
    }
}

sol! {
    #[allow(missing_docs)]
    #[sol(rpc)]
    interface IPool {
        #[derive(Debug)]
        event Swap(address indexed sender, uint amount0In, uint amount1In, uint amount0Out, uint amount1Out, address indexed to);

        #[derive(Debug, PartialEq, Eq)]
        function token0() external view returns (address);

        #[derive(Debug, PartialEq, Eq)]
        function token1() external view returns (address);

        #[derive(Debug, PartialEq, Eq)]
        function getReserves() external view returns (uint112 reserve0, uint112 reserve1, uint32 blockTimestampLast);

        #[derive(Debug, PartialEq, Eq)]
        function swap(uint256 amount0Out, uint256 amount1Out, address to, bytes calldata data) external;

        #[derive(Debug, PartialEq, Eq)]
        function factory() external view returns (address);
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
    ) -> Result<(Vec<U256>, Vec<Log>, AccessList)> {
        let calldata = Bytes::from(
            IRouter::swapExactTokensForTokensCall::new((
                amount_in,
                amount_out_min,
                path,
                to,
                deadline,
            ))
            .abi_encode(),
        );

        let mut evm = Evm::builder()
            .with_db(&mut *database)
            .modify_tx_env(|tx| {
                tx.caller = self.caller;
                tx.transact_to = TxKind::Call(self.router);
                tx.data = calldata.clone();
                tx.value = U256::from(0);
            })
            .build();

        let result = evm.transact()?;
        let (output, logs, gas_used, gas_refunded) = extract_gas_output_and_logs(&result.result)?;
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
                tx.data = calldata;
                tx.value = U256::from(0);
                tx.access_list = access_list.0.clone();
            })
            .build();

        let result = evm.transact_commit()?;
        let (output, _, gas_used, gas_refunded) = extract_gas_output_and_logs(&result)?;
        let output = <Vec<U256>>::abi_decode(&output, false)?;

        println!("Router Swap - Gas used with access list: {:?}", gas_used);
        println!(
            "Router Swap - Gas refunded with access list: {:?}",
            gas_refunded
        );
        println!("Router Swap - Output: {:?}", output);

        Ok((output, logs, access_list))
    }
}

#[derive(Debug)]
pub struct Pool {
    caller: Address,
    pool: Address,
}

#[derive(Debug, Clone)]
pub struct Swap {
    pub pool: Address,
    pub amount0_in: U256,
    pub amount1_in: U256,
    pub amount0_out: U256,
    pub amount1_out: U256,
}

#[derive(Debug, Clone)]
pub struct PoolData {
    pub token_0: Address,
    pub token_1: Address,
}

impl Pool {
    pub fn new(caller: Address, pool: Address) -> Self {
        Self { caller, pool }
    }

    pub fn decode_swaps(logs: &[Log]) -> Result<Vec<Swap>> {
        let mut swaps = vec![];

        for log in logs.iter() {
            if !log.data.topics().is_empty() && log.data.topics()[0] == IPool::Swap::SIGNATURE_HASH
            {
                if let Ok(swap) = IPool::Swap::decode_log(log, true) {
                    swaps.push(Swap {
                        pool: log.address,
                        amount0_in: swap.amount0In,
                        amount1_in: swap.amount1In,
                        amount0_out: swap.amount0Out,
                        amount1_out: swap.amount1Out,
                    });
                }
            }
        }

        Ok(swaps)
    }

    pub fn swap(
        &self,
        amount0_out: U256,
        amount1_out: U256,
        to: Address,
        database: &mut AlloyCacheDB,
    ) -> Result<AccessList> {
        let calldata = Bytes::from(
            IPool::swapCall::new((amount0_out, amount1_out, to, Bytes::default())).abi_encode(),
        );
        let mut evm = Evm::builder()
            .with_db(&mut *database)
            .modify_tx_env(|tx| {
                tx.caller = self.caller;
                tx.transact_to = TxKind::Call(self.pool);
                tx.data = calldata.clone();
                tx.value = U256::from(0);
            })
            .build();

        let result = evm.transact()?;
        let (output, _, gas_used, gas_refunded) = extract_gas_output_and_logs(&result.result)?;
        let access_list = extract_access_list(&result);

        println!("Pool Swap - Gas used with no access list: {:?}", gas_used);
        println!(
            "Pool Swap - Gas refunded with no access list: {:?}",
            gas_refunded
        );
        println!("Pool Swap - Output: {:?}", output);

        drop(evm);

        let mut evm = Evm::builder()
            .with_db(database)
            .modify_tx_env(|tx| {
                tx.caller = self.caller;
                tx.transact_to = TxKind::Call(self.pool);
                tx.data = calldata;
                tx.value = U256::from(0);
                tx.access_list = access_list.0.clone();
            })
            .build();

        let result = evm.transact_commit()?;
        let (output, _, gas_used, gas_refunded) = extract_gas_output_and_logs(&result)?;

        println!("Pool Swap - Gas used with access list: {:?}", gas_used);
        println!(
            "Pool Swap - Gas refunded with access list: {:?}",
            gas_refunded
        );
        println!("Pool Swap - Output: {:?}", output);

        Ok(access_list)
    }

    pub fn get_reserves(&self, database: &mut AlloyCacheDB) -> Result<(U256, U256)> {
        let calldata = Bytes::from(IPool::getReservesCall::new(()).abi_encode());
        let mut evm = Evm::builder()
            .with_db(&mut *database)
            .modify_tx_env(|tx| {
                tx.caller = self.caller;
                tx.transact_to = TxKind::Call(self.pool);
                tx.data = calldata;
                tx.value = U256::from(0);
            })
            .build();

        let result = evm.transact()?;
        let (output, _, _, _) = extract_gas_output_and_logs(&result.result)?;
        let output = <(U256, U256, u128)>::abi_decode(&output, true)?;

        Ok((output.0, output.1))
    }

    pub fn get_pool_data(&self, database: &mut AlloyCacheDB) -> Result<PoolData> {
        let token_0 = self.get_token_0(database)?;
        let token_1 = self.get_token_1(database)?;

        Ok(PoolData { token_0, token_1 })
    }

    fn get_token_0(&self, database: &mut AlloyCacheDB) -> Result<Address> {
        let calldata = Bytes::from(IPool::token0Call::new(()).abi_encode());
        self.get_token(calldata, database)
    }

    fn get_token_1(&self, database: &mut AlloyCacheDB) -> Result<Address> {
        let calldata = Bytes::from(IPool::token1Call::new(()).abi_encode());
        self.get_token(calldata, database)
    }

    fn get_token(&self, calldata: Bytes, database: &mut AlloyCacheDB) -> Result<Address> {
        let mut evm = Evm::builder()
            .with_db(&mut *database)
            .modify_tx_env(|tx| {
                tx.caller = self.caller;
                tx.transact_to = TxKind::Call(self.pool);
                tx.data = calldata;
                tx.value = U256::from(0);
            })
            .build();

        let result = evm.transact()?;
        let (output, _, _, _) = extract_gas_output_and_logs(&result.result)?;
        let output = <Address>::abi_decode(&output, true)?;

        Ok(output)
    }
}
