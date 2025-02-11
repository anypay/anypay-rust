use anyhow::{Result, anyhow};
use serde::{Deserialize, Serialize};
use shortid::next_short_64;
use crate::supabase::SupabaseClient;
use crate::types::{Account, Address};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Coin {
    pub currency: String,
    pub precision: i32,
    pub unavailable: bool,
    // Add other coin properties as needed
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversionRequest {
    pub currency: String,
    pub value: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetAddressRequest {
    pub account: Account,
    pub address: Address,
    pub currency: String,
    pub chain: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToSatoshisRequest {
    pub decimal: f64,
    pub currency: String,
    pub chain: String,
}

#[derive(Debug)]
pub struct ComputeUriRequest {
    pub currency: String,
    pub uid: String,
}

#[derive(Debug)]
pub struct Fee {
    pub amount: i64,
    pub address: String,
}

pub async fn convert(from: ConversionRequest, to_currency: &str, precision: Option<i32>) -> Result<f64> {
    // TODO: Implement price conversion using an external service
    // For now, using more realistic mock rates
    let rate = match (from.currency.as_str(), to_currency) {
        ("USD", "BTC") => 0.000025,  // ~$40,000 per BTC
        ("USD", "ETH") => 0.00045,   // ~$2,200 per ETH
        ("USD", "BSV") => 0.0015,    // ~$66 per BSV
        ("USD", "USDT") => 1.0,      // 1:1 for stablecoins
        ("USD", "USDC") => 1.0,      // 1:1 for stablecoins
        _ => return Err(anyhow!("Unsupported currency pair: {} to {}", from.currency, to_currency))
    };
    
    Ok(from.value * rate)
}

pub async fn get_new_address(req: GetAddressRequest) -> Result<String> {
    // For now, returning a mock address with a 64-bit shortid
    let id_bytes = next_short_64(0)?;
    let id = id_bytes.iter()
        .map(|val| format!("{:0>2x}", val))
        .collect::<String>();
    
    Ok(req.address.value)
}

pub async fn to_satoshis(req: ToSatoshisRequest, supabase: &SupabaseClient) -> Result<i64> {
    let coin = supabase.get_coin(&req.currency, &req.chain).await
        .map_err(|e| anyhow!("Failed to get coin: {}", e))?
        .ok_or_else(|| anyhow!("Coin not found"))?;

    // Get precision, defaulting to 8 for BTC/BSV, 18 for ETH, and 6 for stablecoins
    let precision = match req.chain.as_str() {
        "BTC" | "BSV" => 8,
        "ETH" => 18,
        _ => 6  // Default for stablecoins
    };

    let satoshis = (req.decimal * 10f64.powi(precision)) as i64;
    Ok(satoshis)
}

pub async fn get_fee(currency: &str, amount: i64) -> Result<Fee> {
    // Calculate fee based on currency
    let fee_rate = match currency {
        "BTC" | "BSV" => 0.0001,  // 0.01%
        "ETH" | "MATIC" => 0.001, // 0.1%
        _ => 0.001                // Default 0.1%
    };
    
    let fee_amount = (amount as f64 * fee_rate) as i64;
    Ok(Fee {
        amount: fee_amount,
        address: "fee_address_mock".to_string(),
    })
}

pub fn compute_invoice_uri(req: ComputeUriRequest) -> String {
    format!("anypay:{}:{}", req.currency, req.uid)
}

// Helper function to generate IDs
pub fn generate_uid() -> String {
    nanoid::nanoid!(12)  // 21 chars like in the JS version
} 