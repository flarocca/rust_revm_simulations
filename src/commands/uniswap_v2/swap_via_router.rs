use alloy_eips::{BlockId, BlockNumberOrTag};
use alloy_provider::Provider;
use alloy_provider::{network::Ethereum, ProviderBuilder, RootProvider};
use alloy_rpc_types::Block;
use alloy_transport_http::Http;
use anyhow::{anyhow, Result};
use clap::ArgMatches;
use reqwest::Client;
use revm::db::{AlloyDB, CacheDB};
use revm::primitives::{address, Address, Log, U256};
use std::str::FromStr;

use crate::commands::uniswap_v2::contracts::Pool;
use crate::commons::erc20::Erc20;
use crate::commons::helpers::set_eth_balance;

use super::contracts::Router;

#[derive(Debug, Clone)]
struct SwapViaRouterConfig {
    token_in: Address,
    token_out: Address,
    amount: U256,
    caller: Address,
}

impl SwapViaRouterConfig {
    pub fn from_args(caller: Address, args: &ArgMatches) -> Self {
        let token_in = args
            .get_one::<String>("token-in")
            .expect("Token in is required");
        let token_in = Address::from_str(token_in).expect("Invalid token in address");

        let token_out = args
            .get_one::<String>("token-out")
            .expect("Token out is required");
        let token_out = Address::from_str(token_out).expect("Invalid token out address");

        let amount = args
            .get_one::<String>("amount")
            .expect("Amount is required");
        let amount = U256::from_str(amount).expect("Invalid amount");

        Self {
            token_in,
            token_out,
            amount,
            caller,
        }
    }
}

pub async fn execute(args: &ArgMatches) {
    let rpc_url = args
        .get_one::<String>("rpc-url")
        .expect("RPC URL is required");
    let rpc_url = rpc_url.parse().unwrap();

    let client = ProviderBuilder::new().on_http(rpc_url);
    let block = client
        .get_block_by_number(BlockNumberOrTag::Latest, true)
        .await
        .unwrap()
        .unwrap();

    // The caller must be the public address that will sign the transactions,
    // which implies this wallet must be funded. For the purposes of this example
    // we are using a random address that is funded when simulating.
    let caller = address!("FF3cF7b8582571095A2B05268A4E1BafBDAD060D");

    let swap_configuration = SwapViaRouterConfig::from_args(caller, args);

    // Step 1: Simulate the swap via the router and extract the Swap events that contains
    // the pool address and the amount of tokens swapped.
    let logs = simulate_with_router(&block, &client, &swap_configuration)
        .expect("Error running simulation for router");

    // Step 2: Based on the discovery made with the router, we know simulate the swap hitting
    // the poool straiught away.
    let (_amount_in, _amount_out) = simulate_with_pool(&block, &client, logs, &swap_configuration)
        .expect("Error running simulation for pool");

    // Step 3: Build the final transaction and send it to builders.
    // Step 4: Monitor the chain until we find our transaction in a block
    // WIP
}

fn simulate_with_router(
    block: &Block,
    client: &RootProvider<Http<Client>>,
    swap_configuration: &SwapViaRouterConfig,
) -> Result<Vec<Log>> {
    let block_id = BlockId::Number(BlockNumberOrTag::Number(block.header.number));
    let database: AlloyDB<Http<Client>, Ethereum, &RootProvider<Http<Client>>> =
        AlloyDB::new(client, block_id).unwrap();
    let mut database = CacheDB::new(database);

    // This is the address of UniswapV2 Router at Ethereum mainnet. If testing in a
    // different chain, please change this address accordingly.
    let router = address!("7a250d5630B4cF539739dF2C5dAcb4c659F2488D");

    let token = Erc20::new(swap_configuration.caller, swap_configuration.token_in);

    // Optional step for convenience. Extreme caution is advised when using this method
    // as this might lead to a simulation that does not match the real state of the blockchain.
    set_eth_balance(
        swap_configuration.caller,
        swap_configuration.amount,
        &mut database,
    );
    let _ = token.set_balance(
        swap_configuration.caller,
        swap_configuration.amount,
        &mut database,
    );

    // Mandatory step, the router performs a `transfer_from` call pon the Erc20 on behalf of the
    // caller. Without this step, the swap will fail with a `TRANSFER_FROM_FAILED` error.
    let _ = token.approve(router, swap_configuration.amount, &mut database);

    let router = Router::new(swap_configuration.caller, router);

    let path = vec![swap_configuration.token_in, swap_configuration.token_out];

    // The amount_out_min indicates the router the minimum amount of output tokens expected.
    // The router will fail the swap if the output amount is smaller than this. For our purposes
    // we set it to zero as we are only running a simulation.
    let amount_out_min = U256::ZERO;

    // The deadline, expressed in timestamp, indicates the number of blocks allowed to process the
    // swap. Since there is a new block produced each 12 seconds, we are specifying that the swap
    // is only valid for the next 3 blocks.
    let deadline = U256::from(block.header.timestamp + 36);

    let result = router.swap_exact_tokens_for_tokens(
        swap_configuration.amount,
        amount_out_min,
        path,
        swap_configuration.caller,
        deadline,
        &mut database,
    )?;

    Ok(result.1)
}

fn simulate_with_pool(
    block: &Block,
    client: &RootProvider<Http<Client>>,
    logs: Vec<Log>,
    swap_configuration: &SwapViaRouterConfig,
) -> Result<(U256, U256)> {
    let block_id = BlockId::Number(BlockNumberOrTag::Number(block.header.number));
    let mut database = CacheDB::new(AlloyDB::new(client, block_id).unwrap());

    let swap_logs = Pool::decode_swaps(&logs)?;

    // For the purposes of this example we are only moving forward when there is a single swap.
    // However, there can be cases where the router performed multiple swaps to get the output.
    // This can happen when there is no direct path between two pairs for example.
    if swap_logs.len() != 1 {
        return Err(anyhow!("Only one swap is supported"));
    }

    let swap_event = swap_logs[0].clone();

    let pool = Pool::new(swap_configuration.caller, swap_event.pool);

    let token_in = Erc20::new(swap_configuration.caller, swap_configuration.token_in);
    let token_out = Erc20::new(swap_configuration.caller, swap_configuration.token_out);

    // Optional step for convenience. Extreme caution is advised when using this method
    // as this might lead to a simulation that does not match the real state of the blockchain.
    set_eth_balance(
        swap_configuration.caller,
        swap_configuration.amount,
        &mut database,
    );
    let _ = token_in.set_balance(
        swap_configuration.caller,
        swap_configuration.amount,
        &mut database,
    );

    // Save the balances before perdorming the swap so that we can validate that the swap
    // was successful.
    let balance_in_before = token_in
        .balance_of(swap_configuration.caller, &mut database)?
        .0;
    let balance_out_before = token_out
        .balance_of(swap_configuration.caller, &mut database)?
        .0;

    // Pools have `token0` and `token1`, we are not analyzing the pool, which means we don't
    // know which token we are swapping for. An alternative to get that information without
    // analyzing the pool is to check the Swap event and see which input amount (`amount0_in` or
    // `amount1_in`) is not zero.
    let amount_in = if swap_event.amount0_in.is_zero() {
        swap_event.amount1_in
    } else {
        swap_event.amount0_in
    };

    // Mandatory step, before executing the swap on the pool we need to transfer the correct input
    // token amount to the pool. Without this step, the swap will fail with a `IIA` (Insufficient
    // Input Amount) error.
    let _ = token_in.transfer(swap_event.pool, amount_in, &mut database)?;

    let _ = pool.swap(
        swap_event.amount0_out,
        swap_event.amount1_out,
        swap_configuration.caller,
        &mut database,
    )?;

    // In order to ensure the swap was successful, we need to check the balances of both tokens
    // before and after the swap.
    let balance_in_after = token_in
        .balance_of(swap_configuration.caller, &mut database)?
        .0;
    let balance_out_after = token_out
        .balance_of(swap_configuration.caller, &mut database)?
        .0;

    let amount_out = if swap_event.amount0_out.is_zero() {
        swap_event.amount1_out
    } else {
        swap_event.amount0_out
    };

    assert!(
        balance_in_before - amount_in == balance_in_after,
        "The balance of token in does not match the expected output"
    );
    assert!(
        balance_out_before + amount_out == balance_out_after,
        "The balance of token out does not match the expected output"
    );

    // TODO: For now we are just returning the input and output amounts, which is fine and works.
    // However, that means we will have to build the transaction again to send it to builders.
    // An alternative approach would be to build the final transaction, simulate it and if ok
    // return the transaction ready to be sent.
    Ok((amount_in, amount_out))
}
