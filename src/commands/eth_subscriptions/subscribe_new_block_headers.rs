use alloy_rpc_types::Block;
use async_trait::async_trait;
use clap::{Arg, ArgAction, ArgMatches};
use futures::StreamExt;
use revm::primitives::alloy_primitives::TxHash;

use crate::{
    commands::Command,
    commons::eth_ws_subscriber::{self, EthWsSubscriptionRequest},
};

pub struct SubscribeNewBlockHeaders;

#[async_trait]
impl Command for SubscribeNewBlockHeaders {
    fn create(&self) -> clap::Command {
        clap::Command::new("subscribe-new-block-headers")
            .about("Subscribe to new block headers")
            .long_flag("subscribe-new-block-headers")
            .arg(
                Arg::new("ws-url")
                    .long("ws-url")
                    .required(true)
                    .action(ArgAction::Set)
                    .help("The WS URL to subscribe to"),
            )
    }

    fn name(&self) -> String {
        "subscribe-new-block-headers".to_owned()
    }

    async fn execute(&self, args: &ArgMatches) {
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
}
