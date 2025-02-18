use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;
use tracing::{info, error, debug};
use crate::supabase::SupabaseClient;
use anyhow::anyhow;
// Core types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Confirmation {
    pub confirmation_hash: String,
    pub confirmation_height: i32,
    pub confirmation_date: DateTime<Utc>,
    pub confirmations: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Payment {
    pub id: i32,
    pub txid: String,
    pub chain: String,
    pub currency: String,
    pub status: String,
    pub invoice_uid: String,
    pub confirmation_hash: Option<String>,
    pub confirmation_height: Option<i32>,
    pub confirmation_date: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Invoice {
    pub id: i32,
    pub uid: String,
    pub status: String,
    pub account_id: Option<String>,
    pub app_id: Option<String>,
    pub hash: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct PaymentConfirmedEvent {
    pub topic: String,
    pub payload: PaymentConfirmedPayload,
}

#[derive(Debug, Clone, Serialize)]
pub struct PaymentConfirmedPayload {
    pub account_id: Option<String>,
    pub app_id: Option<String>,
    pub payment: PaymentInfo,
    pub invoice: InvoiceInfo,
    pub confirmation: ConfirmationInfo,
}

#[derive(Debug, Clone, Serialize)]
pub struct PaymentInfo {
    pub chain: String,
    pub currency: String,
    pub txid: String,
    pub status: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct InvoiceInfo {
    pub uid: String,
    pub status: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ConfirmationInfo {
    pub hash: String,
    pub height: i32,
}

#[derive(Debug, Clone)]
pub struct BlockNotification {
    pub hash: String,
    pub height: u32,
    pub timestamp: i64,
    pub txids: Vec<String>,
}

pub struct ConfirmationService {
    supabase: SupabaseClient,
    block_tx: broadcast::Sender<BlockNotification>,
}

impl ConfirmationService {
    pub fn new(supabase: SupabaseClient, block_tx: broadcast::Sender<BlockNotification>) -> Self {
        Self { supabase, block_tx }
    }

    pub async fn confirm_payment(&self, payment: Payment, confirmation: Confirmation) -> Result<Payment> {
        info!("Confirming payment {}", payment.id);

        if payment.confirmation_hash.is_some() {
            info!("Payment {} already confirmed", payment.id);
            return Ok(payment);
        }

        // Update payment record
        debug!("Updating payment record {}", payment.id);
        let updated_payment = self.supabase.update_payment(
            payment.id,
            &confirmation.confirmation_hash,
            confirmation.confirmation_height,
            &confirmation.confirmation_date,
        ).await?;

        // Get associated invoice
        let (invoice, _) = self.supabase.get_invoice(&payment.invoice_uid, true).await?.ok_or_else(|| anyhow!("Invoice not found"))?;
        
        debug!("Found associated invoice {}", invoice.id);
        // Update invoice status
        self.supabase.update_invoice_status(&invoice.uid, "paid").await?;

        // Publish confirmation event
        let event = PaymentConfirmedEvent {
            topic: "payment.confirmed".to_string(),
            payload: PaymentConfirmedPayload {
                account_id: Some(invoice.account_id.to_string()),
                app_id: None,
                payment: PaymentInfo {
                    chain: payment.chain,
                    currency: payment.currency,
                    txid: payment.txid,
                    status: updated_payment.status.clone(),
                },
                invoice: InvoiceInfo {
                    uid: invoice.uid,
                    status: "paid".to_string(),
                },
                confirmation: ConfirmationInfo {
                    hash: confirmation.confirmation_hash,
                    height: confirmation.confirmation_height,
                },
            },
        };

        // TODO: Implement webhook sending
        // await create_and_send_webhook("payment.confirmed", event);

        Ok(updated_payment)
    }

    pub async fn get_confirmation_for_txid(&self, txid: &str) -> Result<Option<Payment>> {
        info!("Getting confirmation for txid {}", txid);

        let payment = match self.supabase.get_payment_by_txid(txid).await? {
            Some(p) => p,
            None => {
                debug!("No payment found for txid {}", txid);
                return Ok(None);
            }
        };

        // TODO: Implement plugin system for getting confirmations
        // let confirmation = get_confirmation(txid, &payment.chain, &payment.currency).await?;

        // For now, return None to indicate no confirmation yet
        Ok(None)
    }

    pub async fn list_unconfirmed_payments(&self, chain: &str, currency: &str) -> Result<Vec<Payment>> {
        info!("Listing unconfirmed payments for {}/{}", chain, currency);
        
        self.supabase.get_unconfirmed_payments(chain, currency).await
    }

    pub async fn start_confirmation_monitoring(&self) {
        info!("Starting confirmation monitoring process");

        let mut block_rx = self.block_tx.subscribe();

        tokio::spawn(async move {
            while let Ok(block) = block_rx.recv().await {
                debug!("Processing new block: {}", block.hash);
                
                // TODO: 
                // 1. Fetch full block details including txids
                // 2. Check for matching unconfirmed payments
                // 3. Confirm any found payments
            }
        });
    }

    pub async fn process_block(&self, block: BlockNotification) -> Result<()> {
        debug!("Processing block {} at height {}", block.hash, block.height);
        
        // Check each transaction in block against unconfirmed payments
        for txid in &block.txids {
            if let Some(payment) = self.supabase.get_unconfirmed_payment_by_txid(txid).await? {
                let confirmation = Confirmation {
                    confirmation_hash: block.hash.clone(),
                    confirmation_height: block.height as i32,
                    confirmation_date: DateTime::from_timestamp(block.timestamp, 0)
                        .unwrap_or_else(|| Utc::now()),
                    confirmations: Some(1),
                };

                match self.confirm_payment(payment, confirmation).await {
                    Ok(_) => info!("Confirmed payment for txid {}", txid),
                    Err(e) => error!("Failed to confirm payment for txid {}: {}", txid, e),
                }
            }
        }
        Ok(())
    }
} 