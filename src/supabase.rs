use serde::{Deserialize, Serialize};
use postgrest::Postgrest;
use serde_json;
use uuid::Uuid;
use chrono::Utc;

use crate::types::{CreateInvoiceRequest, Invoice};

pub struct SupabaseClient {
    client: Postgrest,
    anon_key: String,
    service_role_key: String,
}

impl SupabaseClient {
    pub fn new(url: &str, anon_key: &str, service_role_key: &str) -> Self {
        // Ensure URL ends with /rest/v1
        let api_url = if url.ends_with("/rest/v1") {
            url.to_string()
        } else {
            format!("{}/rest/v1", url.trim_end_matches('/'))
        };

        let client = Postgrest::new(&api_url)
            .insert_header("apikey", anon_key)
            .insert_header("Authorization", &format!("Bearer {}", service_role_key));

        SupabaseClient {
            client,
            anon_key: anon_key.to_string(),
            service_role_key: service_role_key.to_string(),
        }
    }

    pub async fn get_invoice(&self, invoice_id: &str, use_service_role: bool) -> Result<Option<Invoice>, Box<dyn std::error::Error>> {
        let auth_key = if use_service_role {
            &self.service_role_key
        } else {
            &self.anon_key
        };

        let response = self.client
            .from("invoices")
            .select("*")
            .eq("uid", invoice_id)
            .auth(auth_key)
            .execute()
            .await?;

        let response_text = response.text().await?;
        println!("Response: {}", response_text);

        let invoices: Vec<Invoice> = serde_json::from_str(&response_text)?;
        // return only the first invoice
        if let Some(invoice) = invoices.into_iter().next() {
            Ok(Some(invoice))
        } else {
            Ok(None)
        }
    }

    pub async fn create_invoice(
        &self,
        amount: i64,
        currency: &str,
        account_id: i64,
    ) -> Result<Invoice, Box<dyn std::error::Error>> {
        let now = Utc::now().to_rfc3339();
        let new_invoice = CreateInvoiceRequest {
            amount,
            currency: currency.to_string(),
            account_id,
            status: "unpaid".to_string(),
            uid: Uuid::new_v4().to_string(),
            created_at: now.clone(),
            updated_at: now,
        };

        // Log the request payload
        tracing::info!("Creating invoice with payload: {:?}", new_invoice);

        let request_body = serde_json::to_string(&new_invoice)?;
        tracing::debug!("Request body: {}", request_body);
        println!("Request body: {}", request_body);

        let response = match self.client
            .from("invoices")
            .insert(&request_body)
            .auth(&self.service_role_key)
            .execute()
            .await
        {
            Ok(resp) => resp,
            Err(e) => {
                tracing::error!("Failed to execute create invoice request: {}", e);
                return Err(e.into());
            }
        };

        let response_text = response.text().await?;
        tracing::info!("Create invoice response: {}", response_text);

        match serde_json::from_str::<Vec<Invoice>>(&response_text) {
            Ok(mut invoices) => {
                invoices.pop().ok_or_else(|| {
                    tracing::error!("No invoice returned in successful response");
                    "No invoice returned".into()
                })
            }
            Err(e) => {
                tracing::error!("Failed to parse invoice response: {}", e);
                tracing::error!("Response text was: {}", response_text);
                Err(e.into())
            }
        }
    }
}