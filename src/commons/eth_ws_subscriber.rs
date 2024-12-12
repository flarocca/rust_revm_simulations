use anyhow::Result;
use futures::{SinkExt, Stream, StreamExt};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{future, pin::Pin};
use tokio_tungstenite::connect_async;
use tungstenite::Message;

pub type Subscription<T> = Pin<Box<dyn Stream<Item = T> + Send>>;

#[derive(Debug, Deserialize)]
enum EthWsResponse<T> {
    Connected,
    Message(T),
    Empty(),
}

#[derive(Default, Serialize)]
pub struct EthWsSubscriptionRequest {
    jsonrpc: String,
    id: u32,
    method: String,
    params: Vec<String>,
}

impl EthWsSubscriptionRequest {
    pub fn new_heads(id: u32) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id,
            method: "eth_subscribe".to_string(),
            params: vec!["newHeads".to_string()],
        }
    }

    pub fn new_pending_transactions(id: u32) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id,
            method: "eth_subscribe".to_string(),
            params: vec!["newPendingTransactions".to_string()],
        }
    }
}

pub async fn subscribe<T>(
    url: String,
    subscription_request: EthWsSubscriptionRequest,
) -> Result<Subscription<T>>
where
    T: for<'a> Deserialize<'a> + Serialize + Send + 'static,
{
    let (stream, _) = connect_async(url).await.expect("Failed to connect");
    let (mut writer, reader) = stream.split();

    let reader = reader
        .map(|result| match result {
            Ok(message) => match message {
                Message::Text(text) => handle_message(text),
                Message::Close(text) => {
                    println!("Connection closed. Reason: {:?}", text);
                    None
                }
                Message::Ping(_) | Message::Pong(_) => None,
                _ => panic!("Unexpected message: {:?}", message),
            },
            Err(e) => {
                println!("Error reading stream. Reason: {:?}", e);
                None
            }
        })
        .filter_map(future::ready);

    writer
        .send(Message::Text(serde_json::to_string(&subscription_request).unwrap()).clone())
        .await?;

    Ok(Box::pin(reader))
}

fn handle_message<T>(json_str: String) -> Option<T>
where
    T: for<'a> Deserialize<'a> + Serialize + Send + 'static,
{
    let value: Value = serde_json::from_str(&json_str).unwrap();

    let value = value.as_object().unwrap();
    if value.contains_key("params") && value.contains_key("method") {
        let params = value["params"].as_object().unwrap();
        if params.contains_key("result") {
            let result = serde_json::from_value::<T>(params["result"].clone()).unwrap();

            return Some(result);
        }
    }

    None
}
