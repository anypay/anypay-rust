use serde::Deserialize;
use anyhow::{Result, anyhow};

#[derive(Debug, Deserialize)]
pub struct Config {
    pub supabase_url: String,
    pub supabase_anon_key: String,
    pub supabase_service_role_key: String,
    pub amqp_url: Option<String>,
    pub xrpl_wss_url: Option<String>,
    pub websocket_host: String,
    pub websocket_port: u16,
    pub http_host: String,
    pub http_port: u16,
}

impl Config {
    pub fn from_env() -> Result<Self> {
        dotenv::dotenv().ok();

        Ok(Config {
            supabase_url: std::env::var("SUPABASE_URL")
                .map_err(|_| anyhow!("SUPABASE_URL not set"))?,
            supabase_anon_key: std::env::var("SUPABASE_ANON_KEY")
                .map_err(|_| anyhow!("SUPABASE_ANON_KEY not set"))?,
            supabase_service_role_key: std::env::var("SUPABASE_SERVICE_ROLE_KEY")
                .map_err(|_| anyhow!("SUPABASE_SERVICE_ROLE_KEY not set"))?,
            amqp_url: std::env::var("AMQP_URL").ok(),
            xrpl_wss_url: std::env::var("XRPL_WSS_URL").ok(),
            websocket_host: std::env::var("WEBSOCKET_HOST")
                .unwrap_or_else(|_| "127.0.0.1".to_string()),
            websocket_port: std::env::var("WEBSOCKET_PORT")
                .unwrap_or_else(|_| "8080".to_string())
                .parse()
                .map_err(|e| anyhow!("Invalid WEBSOCKET_PORT: {}", e))?,
            http_host: std::env::var("HTTP_HOST")
                .unwrap_or_else(|_| "127.0.0.1".to_string()),
            http_port: std::env::var("HTTP_PORT")
                .unwrap_or_else(|_| "3000".to_string())
                .parse()
                .map_err(|e| anyhow!("Invalid HTTP_PORT: {}", e))?,
        })
    }
} 