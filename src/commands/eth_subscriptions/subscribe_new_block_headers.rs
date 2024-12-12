use alloy_rpc_types::Block;
use clap::ArgMatches;
use futures::StreamExt;
use revm::primitives::alloy_primitives::TxHash;

use crate::commons::eth_ws_subscriber::{self, EthWsSubscriptionRequest};

pub async fn execute(args: &ArgMatches) {
    let ws_url = args
        .get_one::<String>("ws-url")
        .expect("WS URL is required");
    let subscription_request = EthWsSubscriptionRequest::new_heads(1);
    let mut subscription =
        eth_ws_subscriber::subscribe::<Block<TxHash>>(ws_url.to_owned(), subscription_request)
            .await
            .expect("Failed to subscribe");

    while let Some(block) = subscription.next().await {
        println!("Received block: {:#?}", block.header);
    }
}
