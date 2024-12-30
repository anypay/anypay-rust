use xrpl::asynch::clients::{
    AsyncWebSocketClient, SingleExecutorMutex, WebSocketOpen, XRPLAsyncWebsocketIO,
};
use xrpl::models::requests::subscribe::{StreamParameter, Subscribe};
use tracing::info;

pub struct XRPLClient {}

impl XRPLClient {
    pub fn new() -> Self {
        Self {}
    }

    pub async fn run_with_url(&mut self, url: &str) -> Result<(), Box<dyn std::error::Error>> {
        info!("Connecting to XRP Ledger at {}", url);
        let mut client: AsyncWebSocketClient<SingleExecutorMutex, WebSocketOpen> = 
            AsyncWebSocketClient::open(url.parse()?).await?;
        info!("✅ Connected to XRPL");

        let subscribe = Subscribe::new(
            None, None, None, None,
            Some(vec![StreamParameter::Ledger, StreamParameter::Transactions]),
            None, None, None,
        );

        client.xrpl_send(subscribe.into()).await?;
        info!("Subscribed to XRPL streams");

        loop {
            if let Some(msg) = client.xrpl_receive().await? {
                //info!("XRPL Event: {:#?}", msg);
            }
        }
    }
} 