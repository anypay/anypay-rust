mod event_dispatcher;
mod session;
mod types;
mod server;
mod supabase;
mod http;
mod xrpl;
mod amqp;
use xrpl::XRPLClient;
use clap::Parser;
use server::AnypayEventsServer;
use std::net::SocketAddr;
use axum::Server;
use std::sync::Arc;
use supabase::SupabaseClient;
use amqp::AmqpClient;
use dotenv::dotenv;



#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    // Get XRPL URL from env or CLI
    let xrpl_url = std::env::var("XRPL_WSS_URL").ok().unwrap();

    // Load .env file if it exists
    match dotenv() {
        Ok(_) => tracing::info!("Loaded .env file"),
        Err(e) => tracing::warn!("No .env file found: {}", e),
    }

    // Initialize AMQP if configured
    if let Ok(amqp_url) = std::env::var("AMQP_URL") {
        tracing::info!("AMQP URL found, attempting to connect...");
        match AmqpClient::new(&amqp_url).await {
            Ok(_) => tracing::info!("✅ Successfully connected to AMQP server"),
            Err(e) => tracing::error!("❌ Failed to connect to AMQP: {}", e),
        }
    } else {
        tracing::info!("No AMQP_URL found, skipping AMQP connection");
    }



    // Get Supabase credentials from environment
    let supabase_url = std::env::var("SUPABASE_URL")
        .expect("SUPABASE_URL must be set");
    let supabase_anon_key = std::env::var("SUPABASE_ANON_KEY")
        .expect("SUPABASE_ANON_KEY must be set");
    let supabase_service_role_key = std::env::var("SUPABASE_SERVICE_ROLE_KEY")
        .expect("SUPABASE_SERVICE_ROLE_KEY must be set");

    let supabase = Arc::new(SupabaseClient::new(
        &supabase_url,
        &supabase_anon_key,
        &supabase_service_role_key
    ));

    // Start WebSocket server
    let ws_server = AnypayEventsServer::new(
        "127.0.0.1:8080",
        &supabase_url,
        &supabase_anon_key,
        &supabase_service_role_key,
    );
    
    // Start HTTP server
    let http_server = http::HttpServer::new(supabase);
    let http_app = http_server.router();
    let http_addr = SocketAddr::from(([127, 0, 0, 1], 3000));

    tracing::info!("Starting WebSocket server on ws://127.0.0.1:8080");
    tracing::info!("Starting HTTP server on http://127.0.0.1:3000");
    let mut xrpl = XRPLClient::new();
    // Run all services concurrently
    
        tokio::join!(
            ws_server.run(),
            Server::bind(&http_addr)
                .serve(http_app.into_make_service()),
             async move { xrpl.run_with_url(&xrpl_url).await }    
        );


    Ok(())
}
