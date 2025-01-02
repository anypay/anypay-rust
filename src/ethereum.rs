use alloy::providers::{Provider, ProviderBuilder, WsConnect};
use alloy::pubsub::PubSubFrontend;
use futures_util::StreamExt;
use std::sync::Arc;

pub struct EthereumClient {
    provider: Arc<dyn Provider<PubSubFrontend>>,
    chain: String,
}

impl EthereumClient {
    pub async fn new(chain: &str, ws_url: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let ws = WsConnect::new(ws_url);
        let provider = ProviderBuilder::new().on_ws(ws).await?;
        
        Ok(Self {
            provider: Arc::new(provider),
            chain: chain.to_string(),
        })
    }

    pub async fn subscribe_blocks(&self) -> Result<(), Box<dyn std::error::Error>> {
        let sub = self.provider.subscribe_blocks().await?;
        let mut stream = sub.into_stream();
        let chain = self.chain.clone();

        let handle = tokio::spawn(async move {
            println!("Awaiting block headers...");
            while let Some(block) = stream.next().await {
                tracing::debug!("Latest {} block number: {}", chain, block.header.number);
            }
        });

        // Keep the subscription alive
        tokio::spawn(async move {
            handle.await?;
            Ok::<_, Box<dyn std::error::Error + Send + Sync>>(())
        });

        Ok(())
    }
} 