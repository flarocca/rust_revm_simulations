use alloy_sol_types::{sol, SolCall, SolEvent, SolValue};
use anyhow::Result;
use revm::{
    primitives::{AccessList, Address, Bytes, Log, TxKind, I256, U256},
    Evm,
};

use crate::commons::helpers::{extract_access_list, extract_gas_output_and_logs, AlloyCacheDB};

sol! {
    #[allow(missing_docs)]
    #[sol(rpc)]
    interface IPool {
        #[derive(Debug)]
        event Swap(address indexed sender, address indexed recipient, int256 amount0, int256 amount1, uint160 sqrtPriceX96, uint128 liquidity, int24 tick);

        #[derive(Debug, PartialEq, Eq)]
        function swap(address poolAddress, address recipient, address tokenIn, address tokenOut, bool zeroForOne, uint256 amountIn) external returns ( uint256 tokenInBalanceBefore, uint256 tokenInBalanceAfter, uint256 tokenOutBalanceBefore, uint256 tokenOutBalanceAfter);

        #[derive(Debug, PartialEq, Eq)]
        function getPoolData(address pool) external view returns (address token00, address token01, address factory, uint24 fee);
    }
}

#[derive(Debug)]
pub struct Pool {
    caller: Address,
    pool: Address,
    simulator: Address,
}

#[derive(Debug, Clone)]
pub struct Swap {
    pub pool: Address,
    pub amount_0: I256,
    pub amount_1: I256,
}

#[derive(Debug, Clone)]
pub struct PoolData {
    pub factory: Address,
    pub token_0: Address,
    pub token_1: Address,
    pub fee: u128,
}

impl Pool {
    pub fn new(caller: Address, pool: Address, simulator: Address) -> Self {
        Self {
            caller,
            pool,
            simulator,
        }
    }

    pub fn decode_swaps(logs: &[Log]) -> Result<Vec<Swap>> {
        let mut swaps = vec![];

        for log in logs.iter() {
            if !log.data.topics().is_empty() && log.data.topics()[0] == IPool::Swap::SIGNATURE_HASH
            {
                if let Ok(swap) = IPool::Swap::decode_log(log, true) {
                    swaps.push(Swap {
                        pool: log.address,
                        amount_0: swap.amount0,
                        amount_1: swap.amount1,
                    });
                }
            }
        }

        Ok(swaps)
    }

    pub fn swap(
        &self,
        token_in: Address,
        token_out: Address,
        zero_for_one: bool,
        amount_in: U256,
        to: Address,
        database: &mut AlloyCacheDB,
    ) -> Result<AccessList> {
        let calldata = Bytes::from(
            IPool::swapCall::new((self.pool, to, token_in, token_out, zero_for_one, amount_in))
                .abi_encode(),
        );
        let mut evm = Evm::builder()
            .with_db(&mut *database)
            .modify_tx_env(|tx| {
                tx.caller = self.caller;
                tx.transact_to = TxKind::Call(self.simulator);
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
                tx.transact_to = TxKind::Call(self.simulator);
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

    pub fn get_pool_data(&self, database: &mut AlloyCacheDB) -> Result<PoolData> {
        let calldata = Bytes::from(IPool::getPoolDataCall::new((self.pool,)).abi_encode());
        let mut evm = Evm::builder()
            .with_db(&mut *database)
            .modify_tx_env(|tx| {
                tx.caller = self.caller;
                tx.transact_to = TxKind::Call(self.simulator);
                tx.data = calldata;
                tx.value = U256::from(0);
            })
            .build();

        let result = evm.transact()?;
        let (output, _, _, _) = extract_gas_output_and_logs(&result.result)?;
        let (token_0, token_1, factory, fee) =
            <(Address, Address, Address, u128)>::abi_decode(&output, true)?;

        Ok(PoolData {
            factory,
            token_0,
            token_1,
            fee,
        })
    }
}
