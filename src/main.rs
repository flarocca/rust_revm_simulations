mod erc20;
mod helpers;
mod uniswap_v2;

use alloy_eips::{BlockId, BlockNumberOrTag};
use alloy_provider::Provider;
use alloy_provider::{network::Ethereum, ProviderBuilder, RootProvider};
use alloy_transport_http::Http;
use erc20::Erc20;
use reqwest::Client;
use revm::db::{AlloyDB, CacheDB};
use revm::primitives::{address, U256};
use std::str::FromStr;
use uniswap_v2::Router;

pub type AlloyCacheDB = CacheDB<AlloyDB<Http<Client>, Ethereum, RootProvider<Http<Client>>>>;

#[tokio::main]
async fn main() {
    let rpc_url = "https://eth-mainnet.g.alchemy.com/v2/Sg0Hh6Bcv4Dfj2OcU4_6VePVPED-8-MD"
        .parse()
        .unwrap();

    let client = ProviderBuilder::new().on_http(rpc_url);
    let block = client
        .get_block_by_number(BlockNumberOrTag::Latest, true)
        .await
        .unwrap()
        .unwrap();
    let database = AlloyDB::new(client, BlockId::latest()).unwrap();
    let mut cache_db = CacheDB::new(database);

    let usdt = address!("dac17f958d2ee523a2206206994597c13d831ec7");
    let weth = address!("c02aaa39b223fe8d0a0e5c4f27ead9083c756cc2");
    let amount_in = U256::from_str("1000000000000000000").unwrap();

    let caller = address!("af02B4b114322C214A6d95061c4f6299AB618aEc");
    let router = address!("7a250d5630B4cF539739dF2C5dAcb4c659F2488D");

    let token = Erc20::new(caller);
    let _ = token.set_balance(weth, caller, amount_in, &mut cache_db);
    let _ = token.approve(weth, router, amount_in, &mut cache_db);

    let router = Router::new(caller, router);
    let path = vec![weth, usdt];
    let result = router.swap_exact_tokens_for_tokens(
        amount_in,
        U256::ZERO,
        path,
        caller,
        U256::from(block.header.timestamp + 36),
        &mut cache_db,
    );

    println!("Result: {:#?}", result);
}
