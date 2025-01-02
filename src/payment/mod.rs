use anyhow::Result;
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use bigdecimal::BigDecimal;
use std::collections::HashMap;
use shortid::next_short_64;
use crate::{supabase::SupabaseClient};
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
    // For now, returning a mock conversion
    Ok(from.value * 0.00004) // Mock BTC/USD rate
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
    let coin = supabase.get_coin(&req.currency, &req.chain).await.unwrap();

    if coin.is_none() {
        return Err(anyhow::anyhow!("Coin not found"));
    } else {
        let satoshis = (req.decimal * 10f64.powi(coin.unwrap().precision.unwrap_or(0))) as i64;
        return Ok(satoshis);
    }    
}

pub async fn get_fee(currency: &str, amount: i64) -> Result<Fee> {
    // TODO: Implement proper fee calculation
    Ok(Fee {
        amount: amount / 100, // Mock 1% fee
        address: "fee_address_mock".to_string(),
    })
}

pub fn compute_invoice_uri(req: ComputeUriRequest) -> String {
    format!("anypay:{}:{}", req.currency, req.uid)
}

// Helper function to generate short IDs
pub fn generate_uid() -> String {
    let id_bytes = next_short_64(0).unwrap();
    id_bytes.iter()
        .map(|val| format!("{:0>2x}", val))
        .collect::<String>()
} 