mod event_dispatcher;
mod session;
mod types;
mod server;

use server::AnypayEventsServer;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    // Create and run the server
    let server = AnypayEventsServer::new("127.0.0.1:8080");
    server.run().await?;

    Ok(())
}
