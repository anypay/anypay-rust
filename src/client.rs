use anyhow::{Result, anyhow};
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, CONTENT_TYPE};
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use serde::{Deserialize, Serialize};
use bitcoin::Transaction;

const DEFAULT_API_URL: &str = "https://api.anypayx.com";

#[derive(Debug, Deserialize)]
pub struct Invoice {
    pub uid: String,
    pub status: String,
    pub currency: String,
    pub amount: f64,
    pub merchant_name: String,
    pub payment_options: Option<PaymentOptions>,
}

#[derive(Debug, Deserialize)]
pub struct PaymentOptions {
    pub payment_options: Vec<PaymentOption>,
}

#[derive(Debug, Deserialize)]
pub struct PaymentOption {
    pub chain: String,
    pub currency: String,
    pub instructions: Vec<PaymentInstruction>,
}

#[derive(Debug, Deserialize)]
pub struct PaymentInstruction {
    pub outputs: Vec<Output>,
}

#[derive(Debug, Deserialize)]
pub struct Output {
    pub address: String,
    pub amount: f64,
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
pub struct PriceResponse {
    pub price: f64,
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
            .get(&format!("{}/i/{}", self.api_url, uid))
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

    pub async fn get_utxos(&self, address: &str) -> Result<Vec<Utxo>> {
        let response = self.client
            .get(&format!("{}/api/v1/utxos/{}", self.api_url, address))
            .send()
            .await?;

        if !response.status().is_success() {
            let error = response.text().await?;
            return Err(anyhow!("Failed to fetch UTXOs: {}", error));
        }

        let utxos = response.json::<Vec<Utxo>>().await?;
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

    pub async fn get_btc_price(&self) -> Result<f64> {
        let response = self.client
            .get("https://api.anypayx.com/api/v1/prices/BTC/USD")
            .send()
            .await?;

        if !response.status().is_success() {
            let error = response.text().await?;
            return Err(anyhow!("Failed to fetch BTC price: {}", error));
        }

        let price = response.json::<PriceResponse>().await?;
        Ok(price.price)
    }
} 