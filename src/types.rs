use serde::{Deserialize, Serialize};

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
        account_id: i64,
    },
    #[serde(rename = "list_prices")]
    ListPrices,
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
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Invoice {
    pub id: i64,    
    pub uid: String,
    pub amount: i64,
    pub currency: String,
    pub status: String,
    #[serde(rename = "createdAt")]
    pub created_at: String,
    #[serde(rename = "updatedAt")]
    pub updated_at: String,
    pub account_id: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub address: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hash: Option<String>,
    #[serde(default)]
    pub complete: bool,
    // Add other optional fields as needed...
}

#[derive(Debug, Serialize, Deserialize)]
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