use crate::supabase::SupabaseClient;
use serde_json::Value;

pub async fn create_invoice(
    supabase: &SupabaseClient,
    amount: i64,
    currency: &str,
    account_id: i32,
    webhook_url: Option<String>,
    redirect_url: Option<String>,
    memo: Option<String>,
) -> Result<Value, Box<dyn std::error::Error>> {
    // Create invoice in Supabase
    let invoice = supabase.create_invoice(
        amount,
        currency,
        account_id.into(),
        webhook_url,
        redirect_url,
        memo
    ).await.unwrap();



    Ok(invoice)
} 