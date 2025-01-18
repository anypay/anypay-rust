use std::net::SocketAddr;
use std::sync::Arc;
use axum::Server;
use tracing::info;
use anyhow::Result;
use crate::server::AnypayEventsServer;
use crate::supabase::SupabaseClient;
use crate::http::HttpServer;
use crate::amqp::AmqpClient;
use crate::xrpl::XRPLClient;
use crate::ethereum::EthereumClient;

pub struct AnypayServer {
    ws_server: AnypayEventsServer,
    http_server: HttpServer,
    xrpl_client: Option<XRPLClient>,
    eth_client: Option<EthereumClient>,
    polygon_client: Option<EthereumClient>,
    avax_client: Option<EthereumClient>,
    bnb_client: Option<EthereumClient>,
    http_port: u16,
    xrpl_url: Option<String>,
}

impl AnypayServer {
    pub async fn new(
        host: &str,
        port: u16,
        http_port: u16,
        supabase_url: &str,
        supabase_anon_key: &str,
        supabase_service_role_key: &str,
        amqp_url: Option<String>,
        xrpl_wss_url: Option<String>,
        eth_wss_url: Option<String>,
        polygon_wss_url: Option<String>,
        avax_wss_url: Option<String>,
        bnb_wss_url: Option<String>,
    ) -> Result<(Self)> {
        // Initialize Supabase client
        let supabase = Arc::new(SupabaseClient::new(
            supabase_url,
            supabase_anon_key,
            supabase_service_role_key
        ));

        // Initialize AMQP if configured
        if let Some(amqp_url) = &amqp_url {
            info!("Connecting to AMQP...");
            AmqpClient::new(amqp_url).await?;
            info!("✅ AMQP Connected");
        }

        // Initial price load and start updater
        supabase.refresh_prices().await?;
        SupabaseClient::start_price_updater(supabase.clone());

        // Initialize WebSocket server
        let ws_addr = format!("{}:{}", host, port);
        let ws_server = AnypayEventsServer::new(
            &ws_addr,
            supabase_url,
            supabase_anon_key,
            supabase_service_role_key,
        );

        // Initialize HTTP server
        let http_server = HttpServer::new(supabase);

        // Initialize blockchain clients
        let eth_client = if let Some(ws_url) = eth_wss_url {
            info!("Connecting to Ethereum node...");
            match EthereumClient::new("ETH", &ws_url).await {
                Ok(client) => {
                    info!("Connected to Ethereum node");
                    client.subscribe_blocks().await?;
                    Some(client)
                }
                Err(e) => {
                    tracing::error!("Failed to connect to Ethereum node: {}", e);
                    None
                }
            }
        } else {
            None
        };

        let polygon_client = if let Some(ws_url) = polygon_wss_url {
            info!("Connecting to Polygon node...");
            match EthereumClient::new("POLYGON", &ws_url).await {
                Ok(client) => {
                    info!("Connected to Polygon node");
                    client.subscribe_blocks().await?;
                    Some(client)
                }
                Err(e) => {
                    tracing::error!("Failed to connect to Polygon node: {}", e);
                    None
                }
            }
        } else {
            None
        };

        let avax_client = if let Some(ws_url) = avax_wss_url {
            info!("Connecting to Avalanche node...");
            match EthereumClient::new("AVAX", &ws_url).await {
                Ok(client) => {
                    info!("Connected to Avalanche node");
                    client.subscribe_blocks().await?;
                    Some(client)
                }
                Err(e) => {
                    tracing::error!("Failed to connect to Avalanche node: {}", e);
                    None
                }
            }
        } else {
            None
        };

        let bnb_client = if let Some(ws_url) = bnb_wss_url {
            info!("Connecting to Binance Smart Chain node...");
            match EthereumClient::new("BNB", &ws_url).await {
                Ok(client) => {
                    info!("Connected to BSC node");
                    client.subscribe_blocks().await?;
                    Some(client)
                }
                Err(e) => {
                    tracing::error!("Failed to connect to BSC node: {}", e);
                    None
                }
            }
        } else {
            None
        };

        let xrpl_client = xrpl_wss_url.as_ref().map(|_| XRPLClient::new());

        Ok(Self {
            ws_server,
            http_server,
            xrpl_client,
            eth_client,
            polygon_client,
            avax_client,
            bnb_client,
            http_port,
            xrpl_url: xrpl_wss_url,
        })
    }

    pub async fn run(self) -> Result<()> {
        let http_app = self.http_server.router();
        let http_addr = SocketAddr::from(([127, 0, 0, 1], self.http_port));

        info!("Starting WebSocket server...");
        info!("Starting HTTP server on http://127.0.0.1:{}", self.http_port);

        match self.xrpl_client {
            Some(mut xrpl) => {
                if let Some(url) = self.xrpl_url {
                    tokio::join!(
                        self.ws_server.run(),
                        Server::bind(&http_addr).serve(http_app.into_make_service()),
                        async move { xrpl.run_with_url(&url).await }
                    );
                }
            }
            None => {
                tokio::join!(
                    self.ws_server.run(),
                    Server::bind(&http_addr).serve(http_app.into_make_service())
                );
            }
        }

        Ok(())
    }
}
