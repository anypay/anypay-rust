use serde::{Deserialize, Serialize};
use postgrest::Postgrest;
use serde_json;

#[derive(Debug, Serialize, Deserialize)]
pub struct Invoice {
    pub id: String,
    pub amount: i64,
    pub currency: String,
    pub status: String,
    // Add other fields as needed
}

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
}