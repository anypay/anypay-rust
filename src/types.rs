use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};


#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "action")]
pub enum Message {
    #[serde(rename = "subscribe")]
    Subscribe {
        #[serde(rename = "type")]
        sub_type: String,
        id: String,
    },
    #[serde(rename = "unsubscribe")]
    Unsubscribe {
        #[serde(rename = "type")]
        sub_type: String,
        id: String,
    },
    #[serde(rename = "fetch_invoice")]
    FetchInvoice {
        id: String,
    },
    #[serde(rename = "create_invoice")]
    CreateInvoice {        
        amount: i64,
        currency: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        webhook_url: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        redirect_url: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        memo: Option<String>,
    },
    #[serde(rename = "list_prices")]
    ListPrices,
    #[serde(rename = "convert_price")]
    ConvertPrice {
        quote_currency: String,
        base_currency: String,
        #[serde(deserialize_with = "deserialize_number_from_string")]
        quote_value: f64,
    },
    #[serde(rename = "cancel_invoice")]
    CancelInvoice {
        uid: String,
    },
    #[serde(rename = "ping")]
    Ping,
}

fn deserialize_number_from_string<'de, D>(deserializer: D) -> Result<f64, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::de::Error;
    
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum StringOrFloat {
        String(String),
        Float(f64),
        Integer(i64),
    }

    match StringOrFloat::deserialize(deserializer)? {
        StringOrFloat::String(s) => s.parse::<f64>().map_err(Error::custom),
        StringOrFloat::Float(f) => Ok(f),
        StringOrFloat::Integer(i) => Ok(i as f64),
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Response {
    pub status: String,
    pub message: String,
}

#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub struct Subscription {
    pub sub_type: String,
    pub id: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateInvoiceRequest {
    pub amount: i64,
    pub currency: String,
    pub account_id: i64,
    pub status: String,
    pub uid: String,
    #[serde(rename = "createdAt")]
    pub created_at: String,  // ISO 8601 timestamp
    #[serde(rename = "updatedAt")]
    pub updated_at: String,
    pub webhook_url: Option<String>,
    pub redirect_url: Option<String>,
    pub memo: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Invoice {
    pub id: i64,
    pub uid: String,
    pub amount: i64,
    pub currency: String,
    pub status: String,
    pub account_id: i64,
    pub complete: Option<bool>,
    pub webhook_url: Option<String>,
    pub redirect_url: Option<String>,
    pub memo: Option<String>,
    pub uri: String,
    pub createdAt: String,
    pub updatedAt: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Price {
    pub id: i64,
    pub currency: String,
    pub value: f64,
    #[serde(rename = "createdAt")]
    pub created_at: String,
    #[serde(rename = "updatedAt")]
    pub updated_at: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PaymentOutput {
    pub address: Option<String>,
    pub script: Option<String>,
    pub amount: f64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PaymentTemplate {
    pub chain: Option<String>,
    pub currency: String,
    #[serde(rename = "to")]
    pub outputs: Vec<PaymentOutput>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PaymentOptions {
    pub webhook: Option<String>,
    pub redirect: Option<String>,
    pub secret: Option<String>,
    pub metadata: Option<serde_json::Value>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PaymentRequest {
    pub template: Vec<PaymentTemplate>,
    pub options: Option<PaymentOptions>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PaymentOption {
    pub invoice_uid: String,
    pub currency: String,
    pub chain: String,
    pub amount: i64,
    pub address: String,
    pub outputs: Vec<Output>,
    pub uri: String,
    pub fee: i64,
    #[serde(rename = "createdAt")]
    pub created_at: String,
    #[serde(rename = "updatedAt")]
    pub updated_at: String,
    pub expires: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Output {
    pub address: String,
    pub amount: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Account {
    pub id: i64,
    pub denomination: Option<String>,
    // ... other fields ...
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Address {
    pub chain: String,
    pub currency: String,
    pub value: String,
    // ... other fields
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Coin {
    pub id: i64,
    pub currency: String,
    pub chain: String,
    #[serde(default)]
    pub precision: Option<i32>,
    #[serde(default)]
    pub unavailable: bool,
    #[serde(rename = "uri_template")]
    pub uri_template: Option<String>,
    #[serde(rename = "createdAt")]
    pub created_at: String,
    #[serde(rename = "updatedAt")]
    pub updated_at: String,
    #[serde(default)]
    pub supported: bool,
    pub required_fee_rate: Option<i64>,
    pub color: Option<String>,
}