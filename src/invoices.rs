use crate::supabase::SupabaseClient;
use crate::types::{Invoice, PaymentOption};
use serde_json::json;
use chrono::Utc;
use crate::payment::generate_uid;

pub async fn create_invoice(
    supabase: &SupabaseClient,
    amount: i64,
    currency: &str,
    account_id: i32,
    webhook_url: Option<String>,
    redirect_url: Option<String>,
    memo: Option<String>,
) -> Result<serde_json::Value, Box<dyn std::error::Error + Send + Sync>> {
    let now = Utc::now().to_rfc3339();
    let invoice_uid = format!("inv_{}", generate_uid());

    let mut data = json!({
        "uid": invoice_uid,
        "amount": amount,
        "currency": currency,
        "account_id": account_id as i64,
        "status": "unpaid",
        "createdAt": now,
        "updatedAt": now,
        "payment_options": []
    });

    // Add optional fields
    if let Some(url) = &webhook_url {
        data["webhook_url"] = json!(url);
    }
    if let Some(url) = &redirect_url {
        data["redirect_url"] = json!(url);
    }
    if let Some(text) = &memo {
        data["memo"] = json!(text);
    }

    // Create invoice in Supabase
    let response = supabase.create_invoice(
        amount,
        currency,
        account_id as i64,
        webhook_url,
        redirect_url,
        memo
    ).await?;

    Ok(response)
} 