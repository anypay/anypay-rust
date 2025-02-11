use anyhow::{Result, anyhow};
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, CONTENT_TYPE, ACCEPT};
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use serde::{Deserialize, Serialize};
use bitcoin::Transaction;

const DEFAULT_API_URL: &str = "https://api.anypayx.com";
const MEMPOOL_API_URL: &str = "https://mempool.space/api";

#[derive(Debug, Deserialize)]
pub struct Invoice {
    pub uid: String,
    pub status: String,
    pub currency: String,
    pub amount: f64,
    pub uri: String,
    #[serde(rename = "createdAt")]
    pub created_at: String,
    #[serde(rename = "expiresAt")]
    pub expires_at: Option<String>,
    pub payment_options: Vec<PaymentOption>,
    pub notes: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct PaymentOptions {
    pub payment_options: Vec<PaymentOption>,
}

#[derive(Debug, Deserialize)]
pub struct PaymentOption {
    pub time: String,
    pub expires: String,
    pub memo: String,
    #[serde(rename = "paymentUrl")]
    pub payment_url: String,
    #[serde(rename = "paymentId")]
    pub payment_id: String,
    pub chain: String,
    pub currency: String,
    pub network: String,
    pub instructions: Vec<PaymentInstruction>,
}

#[derive(Debug, Deserialize)]
pub struct PaymentInstruction {
    #[serde(rename = "type")]
    pub instruction_type: String,
    #[serde(rename = "requiredFeeRate")]
    pub required_fee_rate: u32,
    pub outputs: Vec<Output>,
}

#[derive(Debug, Deserialize)]
pub struct Output {
    pub address: String,
    pub amount: u64,
}

#[derive(Debug, Deserialize)]
pub struct PriceResponse {
    pub price: f64,
}

#[derive(Debug, Deserialize)]
struct MempoolUtxoStatus {
    confirmed: bool,
    block_height: Option<u32>,
    block_time: Option<u64>,
}

#[derive(Debug, Deserialize)]
struct MempoolUtxo {
    txid: String,
    vout: u32,
    value: u64,
    status: MempoolUtxoStatus,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Utxo {
    pub txid: String,
    pub vout: u32,
    pub amount: f64,
    pub confirmations: u32,
    pub script_pub_key: String,
}

#[derive(Debug, Deserialize)]
pub struct Price {
    pub currency: String,
    pub base_currency: String,
    pub value: String,
    #[serde(rename = "updatedAt")]
    pub updated_at: String,
    pub source: String,
}

#[derive(Debug, Deserialize)]
pub struct PriceList {
    pub prices: Vec<Price>,
}

#[derive(Debug, Deserialize)]
struct ConversionResponse {
    conversion: Conversion,
}

#[derive(Debug, Deserialize)]
struct Conversion {
    output: ConversionOutput,
}

#[derive(Debug, Deserialize)]
struct ConversionOutput {
    value: f64,
}

pub struct AnypayClient {
    client: reqwest::Client,
    api_url: String,
}

impl AnypayClient {
    pub fn new(api_key: &str) -> Self {
        let mut headers = HeaderMap::new();
        let auth_value = format!("{}:", api_key); // Basic auth with empty password
        let auth_header = format!("Basic {}", BASE64.encode(auth_value.as_bytes()));
        headers.insert(
            AUTHORIZATION,
            HeaderValue::from_str(&auth_header).expect("Invalid authorization header value"),
        );

        let client = reqwest::Client::builder()
            .default_headers(headers)
            .build()
            .expect("Failed to create HTTP client");

        Self {
            client,
            api_url: DEFAULT_API_URL.to_string(),
        }
    }

    pub async fn get_invoice(&self, uid: &str) -> Result<Invoice> {
        let response = self.client
            .get(&format!("{}/api/v1/invoices/{}", self.api_url, uid))
            .send()
            .await?;

        if !response.status().is_success() {
            let error = response.text().await?;
            return Err(anyhow!("Failed to fetch invoice: {}", error));
        }

        let data = response.json::<serde_json::Value>().await?;
        let invoice = data.get("invoice")
            .ok_or_else(|| anyhow!("Invalid response format: missing invoice field"))?;
        
        serde_json::from_value(invoice.clone())
            .map_err(|e| anyhow!("Failed to parse invoice: {}", e))
    }

    pub async fn get_payment_option(&self, uid: &str, chain: &str, currency: &str) -> Result<Invoice> {
        let payload = serde_json::json!({
            "chain": chain,
            "currency": currency
        });

        let response = self.client
            .post(&format!("{}/i/{}", self.api_url, uid))
            .header("content-type", "application/payment-request")
            .header("x-currency", currency)
            .header("x-chain", chain)
            .json(&payload)
            .send()
            .await?;

        if !response.status().is_success() {
            let error = response.text().await?;
            return Err(anyhow!("Failed to fetch payment options: {}", error));
        }

        let data = response.json::<serde_json::Value>().await?;
        let invoice = data.get("invoice")
            .ok_or_else(|| anyhow!("Invalid response format: missing invoice field"))?;
        
        serde_json::from_value(invoice.clone())
            .map_err(|e| anyhow!("Failed to parse invoice with payment options: {}", e))
    }

    pub async fn get_utxos(&self, address: &str) -> Result<Vec<Utxo>> {
        let response = reqwest::Client::new()
            .get(&format!("{}/address/{}/utxo", MEMPOOL_API_URL, address))
            .send()
            .await?;

        if !response.status().is_success() {
            let error = response.text().await?;
            return Err(anyhow!("Failed to fetch UTXOs from mempool.space: {}", error));
        }

        let mempool_utxos = response.json::<Vec<MempoolUtxo>>().await?;
        
        // Get the current block height for calculating confirmations
        let tip_response = reqwest::Client::new()
            .get(&format!("{}/blocks/tip/height", MEMPOOL_API_URL))
            .send()
            .await?;

        let current_height = if tip_response.status().is_success() {
            tip_response.text().await?.parse::<u32>().unwrap_or(0)
        } else {
            0
        };

        // Convert mempool UTXOs to our format
        let utxos = mempool_utxos.into_iter()
            .map(|u| {
                let confirmations = if u.status.confirmed {
                    u.status.block_height
                        .map(|height| current_height.saturating_sub(height) + 1)
                        .unwrap_or(0)
                } else {
                    0
                };

                Utxo {
                    txid: u.txid,
                    vout: u.vout,
                    amount: u.value as f64 / 100_000_000.0, // Convert satoshis to BTC
                    confirmations,
                    script_pub_key: String::new(), // Mempool API doesn't provide scriptPubKey
                }
            })
            .collect();

        Ok(utxos)
    }

    pub async fn submit_payment(&self, invoice_uid: &str, chain: &str, currency: &str, tx_hex: &str) -> Result<()> {
        let payload = serde_json::json!({
            "chain": chain,
            "currency": currency,
            "transactions": [{
                "tx": tx_hex
            }]
        });

        let response = self.client
            .post(&format!("{}/r/{}", self.api_url, invoice_uid))
            .header(CONTENT_TYPE, "application/payment")
            .json(&payload)
            .send()
            .await?;

        if !response.status().is_success() {
            let error = response.text().await?;
            return Err(anyhow!("Failed to submit payment: {}", error));
        }

        Ok(())
    }

    pub async fn get_prices(&self) -> Result<PriceList> {
        let response = self.client
            .get(&format!("{}/api/v1/prices", self.api_url))
            .send()
            .await?;

        if !response.status().is_success() {
            let error = response.text().await?;
            return Err(anyhow!("Failed to fetch prices: {}", error));
        }

        let prices = response.json::<PriceList>().await?;
        Ok(prices)
    }

    pub async fn get_btc_price(&self) -> Result<f64> {
        let prices = self.get_prices().await?;
        let btc_price = prices.prices.iter()
            .find(|p| p.currency == "BTC" && p.base_currency == "USD")
            .ok_or_else(|| anyhow!("BTC price not found"))?;
        
        btc_price.value.parse::<f64>()
            .map_err(|e| anyhow!("Failed to parse BTC price: {}", e))
    }

    pub async fn get_price(&self, currency: &str) -> Result<f64> {
        let response = self.client
            .get(&format!("{}/convert/1-{}/to-USD", self.api_url, currency))
            .send()
            .await?;

        if !response.status().is_success() {
            let error = response.text().await?;
            return Err(anyhow!("Failed to fetch price: {}", error));
        }

        let conversion = response.json::<ConversionResponse>().await?;
        Ok(conversion.conversion.output.value)
    }
} 