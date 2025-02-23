use anyhow::Result;
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use tokio_tungstenite::{connect_async, tungstenite::{Message, http::{Uri, Request, HeaderValue}}};
use tracing::{info, error};
use tokio::sync::oneshot;
use reqwest;
use crate::supabase::SupabaseClient;
use crate::confirmations;
use chrono::{DateTime, Utc};

#[derive(Debug, Serialize)]
struct SubscribeRequest {
    id: String,
    method: String,
    params: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct BlockNotification {
    hash: String,
    height: u32,
    #[serde(default)]
    timestamp: i64,
}

#[derive(Debug, Deserialize)]
struct TransactionInput {
    txid: String,
    sequence: u32,
    n: u32,
    addresses: Vec<String>,
    #[serde(rename = "isAddress")]
    is_address: bool,
    value: String,
}

#[derive(Debug, Deserialize)]
struct TransactionOutput {
    value: String,
    n: u32,
    hex: String,
    addresses: Vec<String>,
    #[serde(rename = "isAddress")]
    is_address: bool,
}

#[derive(Debug, Deserialize)]
struct TransactionNotification {
    txid: String,
    version: u32,
    vin: Vec<TransactionInput>,
    vout: Vec<TransactionOutput>,
    #[serde(rename = "blockHeight")]
    block_height: u32,
    confirmations: u32,
    #[serde(rename = "blockTime")]
    block_time: u64,
    size: u32,
    vsize: u32,
    value: String,
    #[serde(rename = "valueIn")]
    value_in: String,
    fees: String,
    hex: String,
}

#[derive(Debug, Deserialize)]
struct BlockbookMessage {
    id: Option<String>,
    data: Option<BlockbookData>,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum BlockbookData {
    Block(BlockNotification),
    Transaction(TransactionNotification),
    Subscription { subscribed: bool },
}

#[derive(Debug, Deserialize)]
struct BlockbookBlockResponse {
    hash: String,
    height: u32,
    time: i64,
    txs: Vec<BlockbookTransaction>,
}

#[derive(Debug, Deserialize)]
struct BlockbookTransaction {
    txid: String,
    // We can add other fields if needed later
}

pub struct BlockbookClient {
    ws_url: String,
    api_key: String,
    supabase: SupabaseClient,
}

pub struct BlockbookHandle {
    shutdown: oneshot::Sender<()>,
}

impl BlockbookClient {
    pub fn new(ws_url: String, api_key: String, supabase: SupabaseClient) -> Self {
        Self { ws_url, api_key, supabase }
    }

    pub async fn start_subscription(&self) -> Result<BlockbookHandle> {
        let url = format!("wss://{}/{}", self.ws_url, self.api_key);
        let url = url.parse::<Uri>()?;
        let (ws_stream, _) = connect_async(url).await?;
        let (mut write, mut read) = ws_stream.split();

        // Create shutdown channel
        let (shutdown_tx, shutdown_rx) = oneshot::channel();

        // Subscribe to new blocks
        let block_sub = SubscribeRequest {
            id: "1".to_string(),
            method: "subscribeNewBlock".to_string(),
            params: vec![],
        };
        write.send(Message::Text(serde_json::to_string(&block_sub)?)).await?;

        // Subscribe to new transactions
        /*let tx_sub = SubscribeRequest {
            id: "2".to_string(),
            method: "subscribeNewTransaction".to_string(),
            params: vec![],
        };
        write.send(Message::Text(serde_json::to_string(&tx_sub)?)).await?;*/

        info!("Subscribed to blocks and transactions from Blockbook");

        let ws_url = self.ws_url.clone();
        let api_key = self.api_key.clone();
        let supabase = self.supabase.clone();

        tokio::spawn(async move {
            tokio::select! {
                _ = shutdown_rx => {
                    info!("Shutting down Blockbook subscription");
                    let _ = write.close().await;
                }
                () = async {
                    while let Some(msg) = read.next().await {
                        match msg {
                            Ok(Message::Text(text)) => {
                                // Log raw message first
                                info!("Raw Blockbook message: {}", text);

                                match serde_json::from_str::<BlockbookMessage>(&text) {
                                    Ok(block_msg) => {
                                        if let Some(data) = block_msg.data {
                                            match data {
                                                BlockbookData::Block(block) => {
                                                    info!("New block: hash={} height={}", block.hash, block.height);
                                                    let client = BlockbookClient::new(ws_url.clone(), api_key.clone(), supabase.clone());
                                                    if let Err(e) = client.process_block(&block).await {
                                                        error!("Failed to process block {}: {}", block.hash, e);
                                                    }
                                                }
                                                BlockbookData::Transaction(tx) => {
                                                    info!(
                                                        "New transaction: txid={} value={} fees={} inputs={} outputs={}",
                                                        tx.txid,
                                                        tx.value,
                                                        tx.fees,
                                                        tx.vin.len(),
                                                        tx.vout.len()
                                                    );
                                                }
                                                BlockbookData::Subscription { subscribed } => {
                                                    info!("Subscription update: subscribed={}", subscribed);
                                                }
                                            }
                                        }
                                    }
                                    Err(e) => error!("Failed to parse blockbook message: {} (raw: {})", e, text),
                                }
                            }
                            Err(e) => error!("WebSocket error: {}", e),
                            _ => {}
                        }
                    }
                } => {}
            }
            info!("WebSocket connection closed");
        });

        Ok(BlockbookHandle {
            shutdown: shutdown_tx,
        })
    }

    async fn get_block_txids(&self, hash: &str) -> Result<Vec<String>> {
        let url = format!("https://{}/{}/api/v2/block/{}", self.ws_url, self.api_key, hash);
        let response = reqwest::Client::new()
            .get(&url)
            .header("api-key", &self.api_key)
            .send()
            .await?
            .json::<BlockbookBlockResponse>()
            .await?;

        // Extract just the txids from transactions
        Ok(response.txs.into_iter().map(|tx| tx.txid).collect())
    }

    async fn process_block(&self, block: &BlockNotification) -> Result<()> {
        info!("Processing block {} at height {}", block.hash, block.height);
        
        let txids = self.get_block_txids(&block.hash).await?;
        
        for txid in txids {
            if let Some(payment) = self.supabase.get_unconfirmed_payment_by_txid(&txid).await? {
                let confirmation = confirmations::Confirmation {
                    confirmation_hash: block.hash.clone(),
                    confirmation_height: block.height as i32,
                    confirmation_date: if block.timestamp > 0 {
                        DateTime::from_timestamp(block.timestamp, 0)
                            .unwrap_or_else(|| Utc::now())
                    } else {
                        Utc::now()
                    },
                    confirmations: Some(1),
                };

                match self.supabase.confirm_payment(payment, confirmation).await {
                    Ok(_) => info!("Confirmed payment for txid {}", txid),
                    Err(e) => error!("Failed to confirm payment for txid {}: {}", txid, e),
                }
            }
        }
        Ok(())
    }
}

impl BlockbookHandle {
    pub async fn shutdown(self) {
        let _ = self.shutdown.send(());
    }
} 