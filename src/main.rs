mod event_dispatcher;
mod session;
mod types;
mod server;
mod supabase;

use server::AnypayEventsServer;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    // Get Supabase credentials from environment
    let supabase_url = std::env::var("SUPABASE_URL")
        .expect("SUPABASE_URL must be set");
    let supabase_anon_key = std::env::var("SUPABASE_ANON_KEY")
        .expect("SUPABASE_ANON_KEY must be set");
    let supabase_service_role_key = std::env::var("SUPABASE_SERVICE_ROLE_KEY")
        .expect("SUPABASE_SERVICE_ROLE_KEY must be set");

    // Create and run the server
    let server = AnypayEventsServer::new(
        "127.0.0.1:8080",
        &supabase_url,
        &supabase_anon_key,
        &supabase_service_role_key
    );
    server.run().await?;

    Ok(())
}
