use anypay::types::{Message as WsMessage};
use clap::{Parser, Subcommand};
use futures_util::{SinkExt, StreamExt};
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};
use serde_json::Value;
use url::Url;
use std::error::Error;
use tracing::{error, warn};
use reqwest::header::{HeaderMap, HeaderValue, ACCEPT, CONTENT_TYPE, AUTHORIZATION};
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};

const DEFAULT_API_URL: &str = "https://api.anypayx.com";
const DEFAULT_WS_URL: &str = "wss://ws.anypayx.com";
const ENV_AUTH_TOKEN: &str = "ANYPAY_TOKEN";

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[arg(short, long, value_parser = clap::value_parser!(String), env("ANYPAY_TOKEN"), help = "Auth token (or set ANYPAY_TOKEN env var)")]
    auth_token: Option<String>,

    #[arg(long, default_value = DEFAULT_API_URL, help = "Base URL for API requests")]
    api_url: String,

    #[arg(long, default_value = DEFAULT_WS_URL, help = "WebSocket URL for monitoring")]
    ws_url: String,

    #[arg(long, help = "Output only JSON without any formatting or messages")]
    json: bool,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Create a new invoice
    CreateInvoice {
        #[arg(short, long)]
        amount: i64,
        
        #[arg(short, long)]
        currency: String,
        
        #[arg(short, long)]
        webhook_url: Option<String>,
        
        #[arg(short, long)]
        redirect_url: Option<String>,
        
        #[arg(short, long)]
        memo: Option<String>,
    },
    
    /// Submit a payment transaction for an invoice
    SubmitPayment {
        #[arg(long, help = "Invoice UID")]
        uid: String,

        #[arg(long, help = "Blockchain (e.g. ETH, BTC)")]
        chain: String,

        #[arg(long, help = "Currency/token (e.g. ETH, BTC, USDT)")]
        currency: String,

        #[arg(long, help = "Raw transaction hex")]
        txhex: String,
    },
    
    /// Request a payment with specific parameters or template
    RequestPayment {
        #[arg(long, help = "JSON template file for payment request")]
        template: Option<String>,

        #[arg(long, help = "Destination address")]
        address: Option<String>,

        #[arg(long, help = "Blockchain (e.g. ETH, BTC)")]
        chain: Option<String>,

        #[arg(long, help = "Coin/token symbol (e.g. USDT, ETH)")]
        coin: Option<String>,

        #[arg(long, help = "Currency for amount (e.g. USD)")]
        currency: Option<String>,

        #[arg(long, help = "Amount in specified currency")]
        amount: Option<f64>,

        #[arg(long, help = "Webhook URL for payment notifications")]
        webhook_url: Option<String>,

        #[arg(long, help = "Redirect URL after payment")]
        redirect_url: Option<String>,
    },
    
    /// Set a blockchain address for a specific chain and currency
    SetAddress {
        #[arg(long, help = "Blockchain address")]
        address: String,

        #[arg(long, help = "Blockchain (e.g. ETH, BTC)")]
        chain: String,

        #[arg(long, help = "Currency/token (e.g. ETH, BTC, USDT). Defaults to same as chain if not specified")]
        currency: Option<String>,
    },
    
    /// Get invoice details
    GetInvoice {
        #[arg(short, long)]
        uid: String,
    },
    
    /// Cancel an invoice
    CancelInvoice {
        #[arg(short, long)]
        uid: String,
    },
    
    /// Get current prices
    GetPrices,
    
    /// Monitor an invoice for updates
    MonitorInvoice {
        #[arg(short, long)]
        uid: String,
    },
}

async fn create_invoice(
    client: &reqwest::Client,
    amount: i64,
    currency: String,
    webhook_url: Option<String>,
    redirect_url: Option<String>,
    memo: Option<String>,
    api_url: &str,
) -> Result<Value, Box<dyn Error>> {
    let mut payload = serde_json::json!({
        "amount": amount,
        "currency": currency
    });

    if let Some(webhook) = webhook_url {
        payload["webhook_url"] = serde_json::json!(webhook);
    }
    if let Some(redirect) = redirect_url {
        payload["redirect_url"] = serde_json::json!(redirect);
    }
    if let Some(memo_text) = memo {
        payload["memo"] = serde_json::json!(memo_text);
    }

    let response = client
        .post(&format!("{}/api/v1/invoices", api_url))
        .json(&payload)
        .send()
        .await?;

    let status = response.status();
    let body = response.json::<Value>().await?;

    if !status.is_success() {
        return Err(format!("Failed to create invoice: {}", body).into());
    }

    Ok(body)
}

async fn get_invoice(client: &reqwest::Client, uid: &str, api_url: &str) -> Result<Value, Box<dyn Error>> {
    let mut headers = HeaderMap::new();
    headers.insert(ACCEPT, HeaderValue::from_static("application/payment-options"));

    let response = client
        .get(&format!("{}/i/{}", api_url, uid))
        .headers(headers)
        .send()
        .await?;

    let status = response.status();
    let body = response.json::<Value>().await?;

    if !status.is_success() {
        return Err(format!("Failed to get invoice: {}", body).into());
    }

    Ok(body)
}

async fn cancel_invoice(client: &reqwest::Client, uid: &str, api_url: &str) -> Result<Value, Box<dyn Error>> {
    let response = client
        .delete(&format!("{}/invoices/{}", api_url, uid))
        .send()
        .await?;

    let status = response.status();
    let body = response.json::<Value>().await?;

    if !status.is_success() {
        return Err(format!("Failed to cancel invoice: {}", body).into());
    }

    Ok(body)
}

async fn get_prices(client: &reqwest::Client, api_url: &str) -> Result<Value, Box<dyn Error>> {
    let response = client
        .get(&format!("{}/api/v1/prices", api_url))
        .send()
        .await?;

    let status = response.status();
    let body = response.json::<Value>().await?;

    if !status.is_success() {
        return Err(format!("Failed to get prices: {}", body).into());
    }

    Ok(body)
}

async fn handle_response(response: Result<Message, tokio_tungstenite::tungstenite::Error>) -> Result<Value, Box<dyn Error>> {
    let msg = response?;
    let text = msg.to_text()?;
    let value: Value = serde_json::from_str(text)?;
    
    if value["status"] == "error" {
        error!("Error from server: {}", value["message"]);
        return Err(value["message"].as_str()
            .unwrap_or("Unknown error")
            .to_string()
            .into());
    }
    
    Ok(value)
}

async fn request_payment(
    client: &reqwest::Client,
    template_path: Option<String>,
    address: Option<String>,
    chain: Option<String>,
    coin: Option<String>,
    currency: Option<String>,
    amount: Option<f64>,
    webhook_url: Option<String>,
    redirect_url: Option<String>,
    api_url: &str,
    auth_token: &str,
) -> Result<Value, Box<dyn Error>> {
    let payload = if let Some(path) = template_path {
        // Read template from file
        let template_str = std::fs::read_to_string(path)?;
        serde_json::from_str(&template_str)?
    } else {
        // Build template from parameters
        if let (Some(addr), Some(ch), Some(c), Some(curr), Some(amt)) = (
            &address, &chain, &coin, &currency, &amount
        ) {
            let template = serde_json::json!({
                "template": [{
                    "currency": c,
                    "to": [{
                        "address": addr,
                        "amount": amt,
                        "currency": curr
                    }]
                }],
                "options": {
                    "webhook": webhook_url,
                    "redirect": redirect_url
                }
            });
            // log template
            println!("Template: {}", template);
            template
        } else {
            return Err("Must provide either a template file or all of: address, chain, coin, currency, and amount".into());
        }
    };

    // Create Basic auth header with token as username and empty password
    let auth_value = format!("{}:", auth_token); // Note the colon for empty password
    let auth_header = format!("Basic {}", BASE64.encode(auth_value.as_bytes()));

    let response = client
        .post(&format!("{}/r", api_url))
        .header(AUTHORIZATION, auth_header)
        .json(&payload)
        .send()
        .await?;

    let status = response.status();
    let body = response.json::<Value>().await?;

    if !status.is_success() {
        return Err(format!("Failed to create payment request: {}", body).into());
    }

    Ok(body)
}

async fn submit_payment(
    client: &reqwest::Client,
    uid: &str,
    chain: &str,
    currency: &str,
    txhex: &str,
    api_url: &str,
) -> Result<Value, Box<dyn Error>> {
    let payload = serde_json::json!({
        "chain": chain,
        "currency": currency,
        "transactions": [{
            "tx": txhex
        }]
    });

    // log payload
    println!("Payload: {}", payload);

    let response = client
        .post(&format!("{}/r/{}", api_url, uid))
        .header(CONTENT_TYPE, "application/payment")
        .json(&payload)
        .send()
        .await?;

    let status = response.status();
    let body = response.json::<Value>().await?;

    if !status.is_success() {
        return Err(format!("Failed to submit payment: {}", body).into());
    }

    Ok(body)
}

async fn set_address(
    client: &reqwest::Client,
    address: &str,
    chain: &str,
    currency: &str,
    api_url: &str,
    auth_token: &str,
) -> Result<Value, Box<dyn Error>> {
    // Create the payload for setting an address
    let payload = serde_json::json!({
        "address": address,
        "chain": chain,
        "currency": currency
    });

    // Create auth header
    let auth_value = format!("{}:", auth_token); // Note the colon for empty password
    let auth_header = format!("Basic {}", BASE64.encode(auth_value.as_bytes()));

    // Send the request to set the address
    let response = client
        .post(&format!("{}/api/v1/addresses", api_url))
        .header(AUTHORIZATION, auth_header)
        .json(&payload)
        .send()
        .await?;

    let status = response.status();
    let body = response.json::<Value>().await?;

    if !status.is_success() {
        return Err(format!("Failed to set address: {}", body).into());
    }

    Ok(body)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // Initialize logging only if not in JSON mode
    let cli = Cli::parse();
    if !cli.json {
        tracing_subscriber::fmt::init();
    }
    
    // Create HTTP client
    let client = reqwest::Client::new();

    // Handle different commands
    match cli.command {
        Commands::RequestPayment { 
            template,
            address,
            chain,
            coin,
            currency,
            amount,
            webhook_url,
            redirect_url,
        } => {
            // Require auth token for request-payment
            let auth_token = cli.auth_token
                .ok_or_else(|| "Auth token is required for payment requests. Provide via --auth-token or ANYPAY_TOKEN env var")?;

            let response = request_payment(
                &client,
                template,
                address,
                chain,
                coin,
                currency,
                amount,
                webhook_url,
                redirect_url,
                &cli.api_url,
                &auth_token
            ).await?;
            if cli.json {
                println!("{}", serde_json::to_string(&response)?);
            } else {
                println!("Payment request created: {}", serde_json::to_string_pretty(&response)?);
            }
        },

        Commands::SubmitPayment { 
            uid,
            chain,
            currency,
            txhex,
        } => {
            // Require auth token for submit-payment
            let response = submit_payment(
                &client,
                &uid,
                &chain,
                &currency,
                &txhex,
                &cli.api_url,
            ).await?;
            if cli.json {
                println!("{}", serde_json::to_string(&response)?);
            } else {
                println!("Payment submitted: {}", serde_json::to_string_pretty(&response)?);
            }
        },

        // For other commands, auth token is optional but we'll warn if not present
        cmd => {
            let mut headers = HeaderMap::new();
            if let Some(token) = &cli.auth_token {
                // Create Basic auth header with token as username and empty password
                let auth_value = format!("{}:", token); // Note the colon for empty password
                let auth_header = format!("Basic {}", BASE64.encode(auth_value.as_bytes()));
                headers.insert(
                    AUTHORIZATION,
                    HeaderValue::from_str(&auth_header)?,
                );
            } else if !cli.json {
                warn!("No auth token provided. Some operations may fail. Set via --auth-token or ANYPAY_TOKEN env var");
            }

            let client = reqwest::Client::builder()
                .default_headers(headers)
                .build()?;

            match cmd {
                Commands::CreateInvoice { amount, currency, webhook_url, redirect_url, memo } => {
                    let response = create_invoice(&client, amount, currency, webhook_url, redirect_url, memo, &cli.api_url).await?;
                    if cli.json {
                        println!("{}", serde_json::to_string(&response)?);
                    } else {
                        println!("Created invoice: {}", serde_json::to_string_pretty(&response)?);
                    }
                },
                
                Commands::GetInvoice { uid } => {
                    let response = get_invoice(&client, &uid, &cli.api_url).await?;
                    if cli.json {
                        println!("{}", serde_json::to_string(&response)?);
                    } else {
                        println!("Invoice details: {}", serde_json::to_string_pretty(&response)?);
                    }
                },
                
                Commands::CancelInvoice { uid } => {
                    let response = cancel_invoice(&client, &uid, &cli.api_url).await?;
                    if cli.json {
                        println!("{}", serde_json::to_string(&response)?);
                    } else {
                        println!("Cancel result: {}", serde_json::to_string_pretty(&response)?);
                    }
                },
                
                Commands::GetPrices => {
                    let response = get_prices(&client, &cli.api_url).await?;
                    if cli.json {
                        println!("{}", serde_json::to_string(&response)?);
                    } else {
                        println!("Current prices: {}", serde_json::to_string_pretty(&response)?);
                    }
                },
                
                Commands::SetAddress { address, chain, currency } => {
                    // Require auth token for set-address
                    let auth_token = cli.auth_token
                        .ok_or_else(|| "Auth token is required for setting addresses. Provide via --auth-token or ANYPAY_TOKEN env var")?;
                        
                    // If currency is not specified, use the chain value as the default
                    let currency_value = currency.clone().unwrap_or_else(|| chain.clone());
                    
                    let response = set_address(&client, &address, &chain, &currency_value, &cli.api_url, &auth_token).await?;
                    if cli.json {
                        println!("{}", serde_json::to_string(&response)?);
                    } else {
                        println!("Address set: {}", serde_json::to_string_pretty(&response)?);
                    }
                },
                
                Commands::MonitorInvoice { uid } => {
                    // For monitoring, we still use WebSocket
                    let mut url = Url::parse(&cli.ws_url)?;
                    
                    if let Some(token) = cli.auth_token {
                        url.query_pairs_mut().append_pair("Authorization", &format!("Bearer {}", token));
                    }

                    let (ws_stream, _) = connect_async(url.as_str()).await?;
                    let (mut write, mut read) = ws_stream.split();
                    
                    let msg = WsMessage::Subscribe {
                        sub_type: "invoice".to_string(),
                        id: uid.clone(),
                    };
                    
                    write.send(Message::Text(serde_json::to_string(&msg)?)).await?;
                    if !cli.json {
                        println!("Monitoring invoice {}...", uid);
                    }
                    
                    while let Some(response) = read.next().await {
                        match handle_response(response).await {
                            Ok(value) => {
                                if cli.json {
                                    println!("{}", serde_json::to_string(&value)?);
                                } else {
                                    println!("Update received: {}", serde_json::to_string_pretty(&value)?);
                                }
                            },
                            Err(e) => {
                                if !cli.json {
                                    error!("Error processing update: {}", e);
                                }
                                return Err(e);
                            }
                        }
                    }
                },
                _ => unreachable!(),
            }
        }
    }
    
    Ok(())
} 