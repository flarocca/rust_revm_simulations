use alloy_eips::{BlockId, BlockNumberOrTag};
use alloy_provider::Provider;
use alloy_provider::{ProviderBuilder, RootProvider};
use alloy_rpc_types::Block;
use alloy_transport_http::Http;
use anyhow::Result;
use clap::ArgMatches;
use reqwest::Client;
use revm::db::{AlloyDB, CacheDB};
use revm::primitives::{address, Address, U256};
use std::str::FromStr;

use crate::commands::uniswap_v2::contracts::Pool;
use crate::commons::erc20::Erc20;
use crate::commons::helpers::{set_eth_balance, AlloyCacheDB};

#[derive(Debug, Clone)]
pub(crate) struct SwapViaPoolConfig {
    pool: Address,
    token_in: Address,
    amount: U256,
    caller: Address,
}

impl SwapViaPoolConfig {
    pub fn from_args(caller: Address, args: &ArgMatches) -> Self {
        let pool = args
            .get_one::<String>("pool")
            .expect("Pool address is required");
        let pool = Address::from_str(pool).expect("Invalid pool in address");

        let token_in = args
            .get_one::<String>("token-in")
            .expect("Token in is required");
        let token_in = Address::from_str(token_in).expect("Invalid token in address");

        let amount = args
            .get_one::<String>("amount")
            .expect("Amount is required");
        let amount = U256::from_str(amount).expect("Invalid amount");

        Self {
            pool,
            token_in,
            amount,
            caller,
        }
    }
}

pub async fn execute(rpc_url: &str, args: &ArgMatches) {
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

    let swap_configuration = SwapViaPoolConfig::from_args(caller, args);

    // Step 1: Based on the discovery made with the router, we know simulate the swap hitting
    // the poool straight away.
    let result =
        simulate(&block, &client, &swap_configuration).expect("Error running simulation for pool");

    println!("Swap Via Pool - Result: \n{:#?}", result);

    // Step 3: Build the final transaction and send it to builders.
    // Step 4: Monitor the chain until we find our transaction in a block
    // WIP
}

fn simulate(
    block: &Block,
    client: &RootProvider<Http<Client>>,
    swap_configuration: &SwapViaPoolConfig,
) -> Result<(U256, U256)> {
    let block_id = BlockId::Number(BlockNumberOrTag::Number(block.header.number));
    let mut database = CacheDB::new(AlloyDB::new(client, block_id).unwrap());

    let pool = Pool::new(swap_configuration.caller, swap_configuration.pool);
    let pool_data = pool.get_pool_data(&mut database)?;

    let token_out = if pool_data.token_0 == swap_configuration.token_in {
        pool_data.token_1
    } else {
        pool_data.token_0
    };

    let token_in = Erc20::new(swap_configuration.caller, swap_configuration.token_in);
    let token_out = Erc20::new(swap_configuration.caller, token_out);

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

    let (amount0_out, amount1_out) = get_output_amounts(swap_configuration, &mut database)?;

    // Mandatory step, before executing the swap on the pool we need to transfer the correct input
    // token amount to the pool. Without this step, the swap will fail with a `ISUFFICIENT
    // LIQUIDITY` error.
    let _ = token_in.transfer(
        swap_configuration.pool,
        swap_configuration.amount,
        &mut database,
    )?;

    let _ = pool.swap(
        amount0_out,
        amount1_out,
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

    let amount_out = if pool_data.token_0 == swap_configuration.token_in {
        amount1_out
    } else {
        amount0_out
    };

    assert!(
        balance_in_before - swap_configuration.amount == balance_in_after,
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
    Ok((swap_configuration.amount, amount_out))
}

fn get_output_amounts(
    swap_configuration: &SwapViaPoolConfig,
    database: &mut AlloyCacheDB,
) -> Result<(U256, U256)> {
    // In UniswapV2 protocol, the fee is harcoded to be 0.3%. Since pools are deployed via a
    // well-known factory (0x5C69bEe701ef814a2B6a3EDD4B1652CB9cc5aA6f) also known as Uniswap V2 Deployer,
    // we can safely assume the fee is always 0.3%.
    // Ideally, we should always validate the factory that deployed the pool, not only to ensure
    // the fee is correct, but also to be sure the pool is not a malicious one.
    //
    // For more information check the contract code at: https://github.com/Uniswap/v2-periphery/blob/0335e8f7e1bd1e8d8329fd300aea2ef2f36dd19f/contracts/libraries/UniswapV2Library.sol#L43
    let fee = U256::from(3);
    let percentage = U256::from(1000);

    let pool = Pool::new(swap_configuration.caller, swap_configuration.pool);

    let (reserve_0, reserve_1) = pool.get_reserves(database)?;
    let pool_data = pool.get_pool_data(database)?;

    let (reserve_input, reserver_output) = if pool_data.token_0 == swap_configuration.token_in {
        (reserve_0, reserve_1)
    } else {
        (reserve_1, reserve_0)
    };

    let amount_in_with_fee = swap_configuration
        .amount
        .saturating_mul(percentage.saturating_sub(fee)); // Amoun * (1000 - fee)
    let numerator = amount_in_with_fee.saturating_mul(reserver_output);
    let denominator = reserve_input
        .saturating_mul(percentage)
        .saturating_add(amount_in_with_fee);

    let amount_out = numerator.checked_div(denominator).unwrap();

    if pool_data.token_0 == swap_configuration.token_in {
        Ok((U256::ZERO, amount_out))
    } else {
        Ok((amount_out, U256::ZERO))
    }
}
