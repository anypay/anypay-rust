use clap::Parser;
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;
use anypay::anypay_server::AnypayServer;
use anyhow::Result;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Host address to bind to
    #[arg(long, env = "HOST", default_value = "0.0.0.0")]
    host: String,

    /// Port to listen on
    #[arg(long, env = "PORT", default_value = "8080")]
    port: u16,

    /// HTTP port to listen on
    #[arg(long, env = "HTTP_PORT", default_value = "3000")]
    http_port: u16,

    /// Supabase URL
    #[arg(long, env = "SUPABASE_URL")]
    supabase_url: String,

    /// Supabase anon key
    #[arg(long, env = "SUPABASE_ANON_KEY")]
    supabase_anon_key: String,

    /// Supabase service role key
    #[arg(long, env = "SUPABASE_SERVICE_ROLE_KEY")]
    supabase_service_role_key: String,

    /// AMQP URL for message queue
    #[arg(long, env = "AMQP_URL")]
    amqp_url: Option<String>,

    /// XRPL WebSocket URL
    #[arg(long, env = "XRPL_WSS_URL")]
    xrpl_wss_url: Option<String>,

    /// Ethereum WebSocket URL
    #[arg(long, env = "ETH_WSS_URL")]
    eth_wss_url: Option<String>,

    /// Polygon WebSocket URL
    #[arg(long, env = "POLYGON_WSS_URL")]
    polygon_wss_url: Option<String>,

    /// Avalanche WebSocket URL
    #[arg(long, env = "AVAX_WSS_URL")]
    avax_wss_url: Option<String>,

    /// Binance Smart Chain WebSocket URL
    #[arg(long, env = "BNB_WSS_URL")]
    bnb_wss_url: Option<String>,

    /// Enable debug logging
    #[arg(long)]
    debug: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Parse command line arguments
    let args = Args::parse();

    // Setup logging
    let log_level = if args.debug { Level::DEBUG } else { Level::INFO };
    let subscriber = FmtSubscriber::builder()
        .with_max_level(log_level)
        .with_target(false)
        .with_thread_ids(false)
        .with_file(true)
        .with_line_number(true)
        .compact()
        .init();

    info!("Starting Anypay server...");

    // Initialize and run server
    let server = AnypayServer::new(
        &args.host,
        args.port,
        args.http_port,
        &args.supabase_url,
        &args.supabase_anon_key,
        &args.supabase_service_role_key,
        args.amqp_url,
        args.xrpl_wss_url,
        args.eth_wss_url,
        args.polygon_wss_url,
        args.avax_wss_url,
        args.bnb_wss_url,
    ).await?;

    server.run().await?;

    Ok(())
} 