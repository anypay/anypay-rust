use serde::{Deserialize, Serialize};
use postgrest::Postgrest;
use serde_json::{self, json, Value};
use uuid::Uuid;
use chrono::Utc;
use std::sync::RwLock;
use lazy_static::lazy_static;
use std::collections::HashMap;
use tokio::time::{interval, Duration};
use std::sync::Arc;
use anyhow::{Result, anyhow};

use crate::{payment::ConversionRequest, payment_options::create_payment_options, types::{Account, Address, Coin, CreateInvoiceRequest, Invoice, PaymentOption, Price}};

lazy_static! {
    static ref COIN_CACHE: RwLock<Option<HashMap<String, Coin>>> = RwLock::new(None);
    static ref PRICE_CACHE: RwLock<HashMap<String, Price>> = RwLock::new(HashMap::new());
}

#[derive(Clone)]
pub struct SupabaseClient {
    client: Arc<Postgrest>,
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

        let client = Arc::new(Postgrest::new(&api_url)
            .insert_header("apikey", anon_key)
            .insert_header("Authorization", &format!("Bearer {}", service_role_key)));

        SupabaseClient {
            client,
            anon_key: anon_key.to_string(),
            service_role_key: service_role_key.to_string(),
        }
    }

    pub async fn get_invoice(&self, invoice_id: &str, use_service_role: bool) -> Result<Option<(Invoice, Vec<PaymentOption>)>> {
        let auth_key = if use_service_role {
            &self.service_role_key
        } else {
            &self.anon_key
        };

        tracing::info!("Fetching invoice with id: {}", invoice_id);

        // Get invoice
        let response = self.client.as_ref()
            .from("invoices")
            .select("*")
            .eq("uid", invoice_id)
            .auth(self.service_role_key.to_string())
            .execute()
            .await
            .map_err(|e| anyhow!("Failed to fetch invoice: {}", e))?;

        tracing::info!("Invoice response: {:?}", response);

        let response_text = response.text().await
            .map_err(|e| anyhow!("Failed to read response: {}", e))?;
        tracing::info!("Invoice response text: {:?}", response_text);
        let invoices: Vec<Invoice> = serde_json::from_str(&response_text)
            .map_err(|e| anyhow!("Failed to parse invoice: {}", e))?;

        tracing::info!("Invoices: {:?}", invoices);
        
        if let Some(invoice) = invoices.into_iter().next() {
            // Get payment options
            let response = self.client.as_ref()
                .from("payment_options")
                .select("*")
                .eq("invoice_uid", invoice_id)
                .auth(auth_key)
                .execute()
                .await
                .map_err(|e| anyhow!("Failed to fetch payment options: {}", e))?;

            let response_text = response.text().await
                .map_err(|e| anyhow!("Failed to read response: {}", e))?;
            let payment_options: Vec<PaymentOption> = serde_json::from_str(&response_text)
                .map_err(|e| anyhow!("Failed to parse payment options: {}", e))?;

            // Get account for refreshing payment options
            let account = self.get_account(invoice.account_id).await?;
            tracing::info!("Account: {:?}", account);

            // Check for expired payment options and refresh them
            let updated_options = crate::payment_options::update_expired_payment_options(
                &invoice,
                payment_options,
                &account,
                self
            ).await.unwrap_or_else(|_| Vec::new()); // Return empty vec if refresh fails

            Ok(Some((invoice, updated_options)))
        } else {
            Ok(None)
        }
    }

    pub async fn create_invoice(
        &self,
        amount: i64,
        currency: &str,
        account_id: i64,
        webhook_url: Option<String>,
        redirect_url: Option<String>,
        memo: Option<String>,
    ) -> Result<serde_json::Value> {
        let uid = format!("inv_{}", crate::payment::generate_uid());
        let new_invoice = serde_json::json!([{
            "amount": amount,
            "currency": currency,
            "account_id": account_id,
            "status": "unpaid",
            "uid": uid.clone(),
            "webhook_url": webhook_url,
            "redirect_url": redirect_url,
            "memo": memo,
            "uri": format!("pay:?r=https://api.anypayx.com/r/{}", crate::payment::generate_uid()),
            "createdAt": Utc::now().to_rfc3339(),
            "updatedAt": Utc::now().to_rfc3339(),
        }]);

        tracing::info!("New invoice: {}", new_invoice);

        let response = self.client.as_ref()
            .from("invoices")
            .insert(&serde_json::to_string(&new_invoice).map_err(|e| anyhow!("Failed to serialize invoice: {}", e))?)
            .auth(&self.service_role_key)
            .execute()
            .await
            .map_err(|e| anyhow!("Failed to create invoice: {}", e))?;

        let response_text = response.text()
            .await
            .map_err(|e| anyhow!("Failed to get response text: {}", e))?;
        tracing::info!("Create invoice response: {}", response_text);

        let invoices: Vec<Invoice> = serde_json::from_str(&response_text)
            .map_err(|e| anyhow!("Failed to parse invoice response: {}", e))?;
        let invoice = invoices.into_iter().next()
            .ok_or_else(|| anyhow!("No invoice created"))?;
        
        // Get account and create payment options
        let account = self.get_account(account_id)
            .await
            .map_err(|e| anyhow!("Failed to get account: {}", e))?;
        let payment_options = create_payment_options(&account, &invoice, self)
            .await
            .map_err(|e| anyhow!("Failed to create payment options: {}", e))?;

        Ok(json!({
            "invoice": invoice,
            "payment_options": payment_options
        }))
    }

    pub async fn list_prices(&self) -> Result<Vec<Price>> {
        let response = self.client.as_ref()
            .from("prices")
            .select("*")
            .auth(&self.service_role_key)
            .execute()
            .await
            .map_err(|e| anyhow!("Failed to fetch prices: {}", e))?;

        let text = response.text().await
            .map_err(|e| anyhow!("Failed to read response: {}", e))?;

        let prices = serde_json::from_str::<Vec<Price>>(&text)
            .map_err(|e| anyhow!("Failed to parse prices: {}", e))?;
        Ok(prices)
    }

    pub async fn get_account(&self, account_id: i64) -> Result<Account> {
        let response = self.client.as_ref()
            .from("accounts")
            .select("*")
            .eq("id", account_id.to_string())
            .auth(&self.service_role_key)
            .execute()
            .await
            .map_err(|e| anyhow!("Failed to fetch account: {}", e))?;

        let text = response.text().await
            .map_err(|e| anyhow!("Failed to read response: {}", e))?;

        let accounts: Vec<Account> = serde_json::from_str(&text)
            .map_err(|e| anyhow!("Failed to parse account: {}", e))?;
        accounts.into_iter().next()
            .ok_or_else(|| anyhow!("Account not found"))
    }

    pub async fn list_available_addresses(&self, account: &Account) -> Result<Vec<Address>> {
        let response_text = self.client.as_ref()
            .from("addresses")
            .select("*")
            .eq("account_id", account.id.to_string())
            .execute()
            .await?
            .text()
            .await?;

        let addresses: Vec<Address> = serde_json::from_str(&response_text)?;

        let mut available = Vec::new();
        for addr in addresses {
            let coin = self.get_coin(&addr.currency, &addr.chain).await.unwrap();
            if coin.is_none() {
            } else {
                if !coin.unwrap().unavailable {
                    available.push(addr);
                }
            }

        }

        Ok(available)
    }

    async fn ensure_coins_loaded(&self) -> Result<()> {
        // Check if cache is already loaded
        if COIN_CACHE.read().unwrap().is_some() {
            return Ok(());
        }

        // Load coins if cache is empty
        let response = self.client.as_ref()
            .from("coins")
            .select("*")
            .auth(&self.service_role_key)
            .execute()
            .await?;

        let response_text = response.text().await?;
        tracing::info!("Loading coins from DB: {}", response_text);
        let coins: Vec<Coin> = serde_json::from_str(&response_text)?;
        
        let mut coin_map = HashMap::new();
        for coin in coins {
            coin_map.insert(format!("{}:{}", coin.currency, coin.chain), coin);
        }
        
        let mut cache = COIN_CACHE.write().unwrap();
        *cache = Some(coin_map);
        
        Ok(())
    }

    pub async fn get_coins(&self) -> Result<HashMap<String, Coin>> {
        let response = self.client.as_ref()
            .from("coins")
            .select("*")
            .auth(&self.service_role_key)
            .execute()
            .await?;

        let response_text = response.text().await?;
        let coins: Vec<Coin> = serde_json::from_str(&response_text)?;
        
        // Convert to HashMap
        let mut coin_map = HashMap::new();
        for coin in coins {
            coin_map.insert(coin.currency.clone(), coin);
        }
        
        Ok(coin_map)
    }

    pub async fn get_coin(&self, currency: &str, chain: &str) -> Result<Option<Coin>> {
        self.ensure_coins_loaded().await?;
        
        Ok(COIN_CACHE.read().unwrap()
            .as_ref()
            .and_then(|map| map.get(&format!("{}:{}", currency, chain))
            .cloned()))
    }

    pub async fn refresh_coins(&self) -> Result<()> {
        // Force reload coins
        let mut cache = COIN_CACHE.write().unwrap();
        *cache = None;
        drop(cache);
        
        self.ensure_coins_loaded().await
    }

    pub async fn create_payment_options(&self, options: &[PaymentOption]) -> Result<Vec<PaymentOption>> {
        let response = self.client.as_ref()
            .from("payment_options")
            .insert(&serde_json::to_string(&serde_json::json!(options))?)
            .auth(&self.service_role_key)
            .execute()
            .await?;

        let response_text = response.text().await?;
        tracing::info!("Create payment options response: {}", response_text);
        let inserted: Vec<PaymentOption> = serde_json::from_str(&response_text)?;
        
        Ok(inserted)
    }

    pub async fn start_price_updater(supabase: Arc<Self>) {
        let mut interval = interval(Duration::from_secs(60)); // Every minute

        tokio::spawn(async move {
            loop {
                interval.tick().await;
                if let Err(e) = supabase.refresh_prices().await {
                    tracing::error!("Failed to refresh prices: {}", e);
                }
            }
        });
    }

    pub async fn refresh_prices(&self) -> Result<()> {
        let response = self.client.as_ref()
            .from("prices")
            .select("*")
            .auth(&self.service_role_key)
            .execute()
            .await?;

        let response_text = response.text().await?;
        let prices: Vec<Price> = serde_json::from_str(&response_text)?;

        // Update cache
        let mut cache = PRICE_CACHE.write().unwrap();
        for price in prices {
            cache.insert(price.currency.clone(), price);
        }

        tracing::info!("Updated price cache with {} prices", cache.len());
        Ok(())
    }

    pub fn get_cached_price(&self, currency: &str) -> Option<Price> {
        PRICE_CACHE.read()
            .unwrap()
            .get(currency)
            .cloned()
    }

    pub async fn find_price(&self, base_currency: &str, currency: &str) -> Result<Option<Price>> {
        let response = self.client.as_ref()
            .from("prices")
            .select("*")
            .eq("base_currency", base_currency)
            .eq("currency", currency)
            .auth(&self.service_role_key)
            .execute()
            .await?;

        let response_text = response.text().await?;
        let prices: Vec<Price> = serde_json::from_str(&response_text)?;
        
        Ok(prices.into_iter().next())
    }

    pub async fn update_invoice_status(&self, uid: &str, status: &str) -> Result<()> {
        self.client.as_ref()
            .from("invoices")
            .update(&serde_json::to_string(&json!({
                "status": status
            }))?)
            .eq("uid", uid)
            .execute()
            .await?;
        Ok(())
    }

    pub async fn validate_api_key(&self, api_key: &str) -> Result<Option<i32>> {
        println!("api_key: {:?}", api_key);
        let response = self.client.as_ref()
            .from("access_tokens")
            .select("account_id")
            .eq("uid", api_key)
            .single()
            .execute()
            .await?;

        println!("response: {:?}", response);
            
        let response_text = response.text().await?;
        let data: Value = serde_json::from_str(&response_text)?;
        
        Ok(data.get("account_id").and_then(|v| v.as_i64()).map(|id| id as i32))
    }

    pub async fn cancel_invoice(&self, uid: &str, account_id: i32) -> Result<()> {
        // First fetch invoice to check ownership
        println!("Cancelling invoice: {:?}", uid);
        let (invoice, _) = self.get_invoice(uid, true).await?
            .ok_or(anyhow!("Invoice not found"))?;

        // Verify ownership
        if invoice.account_id as i32 != account_id {
            return Err(anyhow!("Unauthorized to cancel this invoice"));
        }

        // Update status to cancelled
        self.update_invoice_status(uid, "cancelled").await?;
        
        Ok(())
    }
}


pub async fn convert(
    req: ConversionRequest,
    to_currency: &str,
    precision: Option<i32>,
    supabase: &SupabaseClient,
) -> Result<f64> {
    let from_price = supabase.get_cached_price(&req.currency)
        .ok_or_else(|| anyhow!("Price not found for {}", req.currency))?;
    
    let to_price = supabase.get_cached_price(to_currency)
        .ok_or_else(|| anyhow!("Price not found for {}", to_currency))?;

    // Convert through USD
    let usd_value = req.value * from_price.value;
    let converted = usd_value / to_price.value;

    tracing::info!(
        "Converting {} {} (USD rate: {}) to {} (USD rate: {}) = {}",
        req.value,
        req.currency,
        from_price.value,
        to_currency,
        to_price.value,
        converted
    );

    // Apply precision if specified
    /*let result = if let Some(p) = precision {
        let factor = 10f64.powi(p);
        (converted * factor).round() / factor
    } else {
        converted
    };

    Ok(result)*/
    Ok(converted)
}