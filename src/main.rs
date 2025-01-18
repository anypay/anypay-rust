mod event_dispatcher;
mod session;
mod types;
mod server;
mod supabase;
mod http;
mod xrpl;
mod amqp;
mod payment_options;
mod payment;
mod prices;
mod config;
mod invoices;
mod ethereum;
mod uri;
use std::sync::Arc;
use std::net::SocketAddr;

use dotenv::dotenv;
use server::AnypayEventsServer;
use axum::Server;
use supabase::SupabaseClient;
use amqp::AmqpClient;
use xrpl::XRPLClient;
use config::Config;
use ethereum::EthereumClient;
use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();

    // Initialize logging
    tracing_subscriber::fmt::init();

    // Load configuration
    let config = Config::from_env()?;

    // Initialize services
    let supabase = Arc::new(SupabaseClient::new(
        &config.supabase_url,
        &config.supabase_anon_key,
        &config.supabase_service_role_key
    ));

    // Initialize AMQP if configured
    if let Some(amqp_url) = &config.amqp_url {
        tracing::info!("Connecting to AMQP...");
        let _amqp = AmqpClient::new(amqp_url).await?;
        tracing::info!("✅ AMQP Connected");
    }

    // Initial price load
    supabase.refresh_prices().await.unwrap();
    
    // Start price updater
    SupabaseClient::start_price_updater(supabase.clone());

    // Initialize servers
    let ws_addr = format!("{}:{}", config.websocket_host, config.websocket_port);
    let ws_server = AnypayEventsServer::new(
        &ws_addr,
        &config.supabase_url,
        &config.supabase_anon_key,
        &config.supabase_service_role_key,
    );
    
    let http_server = http::HttpServer::new(supabase);
    let http_app = http_server.router();
    let http_addr = SocketAddr::from(([127, 0, 0, 1], config.http_port));

    tracing::info!("Starting WebSocket server on ws://{}", ws_addr);
    tracing::info!("Starting HTTP server on http://127.0.0.1:{}", config.http_port);


    let eth_client = if let Ok(ws_url) = std::env::var("ETH_WSS_URL") {
        tracing::info!("Connecting to Ethereum node...");
        match EthereumClient::new("ETH", &ws_url).await {
            Ok(client) => {
                tracing::info!("Connected to Ethereum node");
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

    let polygon_client = if let Ok(ws_url) = std::env::var("POLYGON_WSS_URL") {
        tracing::info!("Connecting to Ethereum node...");
        match EthereumClient::new("POLYGON", &ws_url).await {
            Ok(client) => {
                tracing::info!("Connected to Polygon node");
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

    let avalanche_client = if let Ok(ws_url) = std::env::var("AVAX_WSS_URL") {
        tracing::info!("Connecting to Ethereum node...");
        match EthereumClient::new("AVAX", &ws_url).await {
            Ok(client) => {
                tracing::info!("Connected to Avalanche node");
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


    let bnb_client = if let Ok(ws_url) = std::env::var("BNB_WSS_URL") {
        tracing::info!("Connecting to Ethereum node...");
        match EthereumClient::new("BNB", &ws_url).await {
            Ok(client) => {
                tracing::info!("Connected to Binance Smart Chain node");
                client.subscribe_blocks().await?;
                Some(client)
            }
            Err(e) => {
                tracing::error!("Failed to connect to Binance Smart Chain node: {}", e);
                None
            }
        }
    } else {
        None
    };

    // Run services
    match &config.xrpl_wss_url {
        Some(xrpl_url) => {
            let mut xrpl = XRPLClient::new();
            tokio::join!(
                ws_server.run(),
                Server::bind(&http_addr).serve(http_app.into_make_service()),
                async move { xrpl.run_with_url(xrpl_url).await }
            );
        }
        None => {
            tokio::join!(
                ws_server.run(),
                Server::bind(&http_addr).serve(http_app.into_make_service())
            );
        }
    }


    Ok(())
}
