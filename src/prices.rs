use std::sync::RwLock;
use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};
use anyhow::Result;
use crate::supabase::SupabaseClient;
use bigdecimal::BigDecimal;
use std::str::FromStr;
use std::ops::{Mul, Div};

const MAX_DECIMALS: i32 = 8;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Amount {
    pub currency: String,
    pub value: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversionRequest {
    pub quote_currency: String,
    pub base_currency: String,
    pub quote_value: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversionResult {
    pub quote_currency: String,
    pub base_currency: String,
    pub quote_value: f64,
    pub base_value: f64,
    pub timestamp: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Price {
    pub currency: String,
    pub base_currency: String,
    pub value: f64,
    pub source: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Conversion {
    pub quote_currency: String,
    pub base_currency: String,
    pub quote_value: f64,
    pub base_value: f64,
    pub timestamp: String,
    pub source: String,
}

pub async fn convert(
    req: ConversionRequest,
    supabase: &SupabaseClient,
) -> Result<ConversionResult> {

    // Try to find direct price
    let price = supabase.find_price(
        &req.base_currency,
        &req.quote_currency
    ).await.unwrap();

    if let Some(price) = price {
        let base_value = BigDecimal::from_str(&req.quote_value.to_string())?
            .mul(BigDecimal::from_str(&price.value.to_string())?)
            .with_scale(MAX_DECIMALS.into())
            .to_string()
            .parse::<f64>()?;

        return Ok(ConversionResult {
            quote_currency: req.quote_currency,
            base_currency: req.base_currency,
            quote_value: req.quote_value,
            base_value,
            timestamp: chrono::Utc::now().to_rfc3339(),
        });
    }

    // Try inverse price
    let inverse = supabase.find_price(
        &req.quote_currency,
        &req.base_currency
    ).await.unwrap();

    if let Some(inverse) = inverse {
        let price = BigDecimal::from_str("1")?
            .div(BigDecimal::from_str(&inverse.value.to_string())?);
            
        let base_value = price
            .mul(BigDecimal::from_str(&req.quote_value.to_string())?)
            .with_scale(MAX_DECIMALS.into())
            .to_string()
            .parse::<f64>()?;

        return Ok(ConversionResult {
            quote_currency: req.quote_currency,
            base_currency: req.base_currency,
            quote_value: req.quote_value,
            base_value,
            timestamp: chrono::Utc::now().to_rfc3339(),
        });
    }

    anyhow::bail!(
        "No price for {} to {}", 
        req.quote_currency, 
        req.base_currency
    )
}

pub async fn create_conversion(
    req: ConversionRequest,
    supabase: &SupabaseClient,
) -> Result<Conversion> {
    let result = convert(req, supabase).await?;
    
    Ok(Conversion {
        quote_currency: result.quote_currency,
        base_currency: result.base_currency,
        quote_value: result.quote_value,
        base_value: result.base_value,
        timestamp: result.timestamp,
        source: "anypay".to_string(), // Or get this from the price record
    })
} 