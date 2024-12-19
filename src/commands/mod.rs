use async_trait::async_trait;
use clap::ArgMatches;
use eth_subscriptions::{
    subscribe_new_block_headers::SubscribeNewBlockHeaders,
    subscribe_new_pending_transactions::SubscribeNewPendingTransactions,
};
use general::compute_address::ComputeAddress;
use std::collections::HashMap;
use uniswap_v2::{
    swap_via_pool::SwapViaPool as UniswapV2SwapViaPool,
    swap_via_router::SwapViaRouter as UniswapV2SwapViaRouter,
};
use uniswap_v3::swap_via_pool::SwapViaPool as UniswapV3SwapViaPool;

pub mod eth_subscriptions;
pub mod general;
pub mod uniswap_v2;
pub mod uniswap_v3;

#[async_trait]
pub trait Command {
    async fn execute(&self, args: &ArgMatches);

    fn create(&self) -> clap::Command;

    fn name(&self) -> String;
}

pub fn get_commands() -> HashMap<String, Box<dyn Command>> {
    let mut result = HashMap::new();

    let commands: Vec<Box<dyn Command>> = vec![
        Box::new(SubscribeNewBlockHeaders),
        Box::new(SubscribeNewPendingTransactions),
        Box::new(ComputeAddress),
        Box::new(UniswapV2SwapViaRouter),
        Box::new(UniswapV2SwapViaPool),
        Box::new(UniswapV3SwapViaPool),
    ];

    for command in commands {
        result.insert(command.name(), command);
    }

    result
}
