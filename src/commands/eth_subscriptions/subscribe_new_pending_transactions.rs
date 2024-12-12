use alloy_eips::{BlockId, BlockNumberOrTag};
use alloy_provider::Provider;
use alloy_provider::{ProviderBuilder, RootProvider};
use alloy_rpc_types::Block;
use alloy_rpc_types::Transaction;
use alloy_transport_http::Http;
use anyhow::Result;
use clap::ArgMatches;
use futures::StreamExt;
use reqwest::Client;
use revm::db::{AlloyDB, CacheDB};
use revm::primitives::alloy_primitives::TxHash;
use revm::primitives::{TxKind, U256};
use revm::Evm;

use crate::commons::eth_ws_subscriber::{self, EthWsSubscriptionRequest};

pub async fn execute(args: &ArgMatches) {
    let ws_url = args
        .get_one::<String>("ws-url")
        .expect("WS URL is required");
    let rpc_url = ws_url.clone().replace("wss", "https");
    let subscription_request = EthWsSubscriptionRequest::new_pending_transactions(1);
    let mut subscription =
        eth_ws_subscriber::subscribe::<TxHash>(ws_url.to_owned(), subscription_request)
            .await
            .expect("Failed to subscribe");

    let client = Client::new();
    let mut transactions = Vec::new();

    while let Some(transaction) = subscription.next().await {
        let response = client
            .post(rpc_url.clone())
            .json(&serde_json::json!({
                "jsonrpc": "2.0",
                "method": "eth_getTransactionByHash",
                "params": [transaction.to_string()],
                "id": 1,
            }))
            .send()
            .await
            .expect("Failed to send transaction to server");

        let response = response
            .json::<serde_json::Value>()
            .await
            .expect("Failed to parse response");

        let json_response = response.as_object().unwrap();
        if json_response.contains_key("result") {
            let result =
                serde_json::from_value::<Transaction>(json_response["result"].clone()).unwrap();

            transactions.push(result.clone());

            if transactions.len() == 10 {
                break;
            }
        }
    }

    simulate_transactions(transactions, rpc_url).await.unwrap();
}

async fn simulate_transactions(transactions: Vec<Transaction>, rpc_url: String) -> Result<()> {
    let rpc_url = rpc_url.parse().unwrap();
    let client = ProviderBuilder::new().on_http(rpc_url);

    let block = client
        .get_block_by_number(BlockNumberOrTag::Latest, true)
        .await
        .unwrap()
        .unwrap();

    let mut database = CacheDB::new(AlloyDB::new(client, block.header.number.into()).unwrap());

    for transaction in transactions {
        let mut evm = Evm::builder()
            .with_db(&mut database)
            .modify_tx_env(|tx| {
                tx.caller = transaction.from;
                if let Some(to) = transaction.to {
                    tx.transact_to = TxKind::Call(to);
                } else {
                    tx.transact_to = TxKind::Create;
                }
                tx.data = transaction.input;
                tx.value = transaction.value;
                tx.gas_limit = transaction.gas;
                tx.gas_price = U256::from(transaction.gas_price.unwrap_or_default());
                tx.nonce = Some(transaction.nonce);
            })
            .modify_block_env(|block_env| {
                block_env.number = U256::from(block.header.number + 1);
            })
            .build();

        println!(
            "Executing transaction\nFrom: {:?}\nHash: {:?}\nNonce: {:?}",
            transaction.from, transaction.hash, transaction.nonce
        );

        match evm.transact_commit() {
            Ok(result) => match result {
                revm::primitives::ExecutionResult::Success { reason, .. } => println!(
                    "Transaction {:?} executed successfully with result: {:?}",
                    transaction.hash, reason
                ),
                revm::primitives::ExecutionResult::Revert { .. } => {
                    println!("Transaction {:?} reverted", transaction.hash)
                }
                revm::primitives::ExecutionResult::Halt { reason, .. } => {
                    println!(
                        "Transaction {:?} halted with reason: {:?}",
                        transaction.hash, reason
                    )
                }
            },
            Err(e) => println!(
                "Error executing transaction {:?}: {:?}",
                transaction.hash, e
            ),
        };
    }
    Ok(())
}
