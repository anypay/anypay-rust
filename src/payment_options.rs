use serde::{Deserialize, Serialize};
use anyhow::Result;
use crate::types::{Invoice, PaymentOption, Output, Account, Address};
use crate::payment::{
    convert, get_fee, get_new_address, to_satoshis, ConversionRequest, GetAddressRequest, ToSatoshisRequest
};
use crate::uri::{compute_invoice_uri, InvoiceUriParams};

use crate::supabase::SupabaseClient;


#[derive(Debug, Serialize, Deserialize)]
pub struct Fee {
    pub amount: i64,
    pub address: String,
}

pub async fn create_payment_options(
    account: &Account,
    invoice: &Invoice,
    supabase: &SupabaseClient,
) -> Result<Vec<PaymentOption>> {
    tracing::info!("Creating payment options for invoice: {:?}", invoice);

    let addresses = supabase.list_available_addresses(account).await.unwrap();
    tracing::info!("Listed available addresses: {:?}", addresses);

    let mut payment_options = Vec::new();

    // Build all payment options first
    for address_record in addresses {
        let chain = address_record.chain.clone();
        let currency = address_record.currency.clone();

        if let Ok(Some(option)) = build_payment_option(
            account,
            invoice,
            &address_record,
            &chain,
            &currency,
            supabase,
        ).await {
            payment_options.push(option);
        }
    }

    // Batch insert all payment options
    if !payment_options.is_empty() {
        let inserted_options = supabase.create_payment_options(&payment_options).await.unwrap();
        return Ok(inserted_options);
    }

    Ok(Vec::new())
}

// Helper function to build a payment option without inserting
async fn build_payment_option(
    account: &Account,
    invoice: &Invoice,
    address_record: &Address,
    chain: &str,
    currency: &str,
    supabase: &SupabaseClient,
) -> Result<Option<PaymentOption>> {
    let coin = supabase.get_coin(currency, chain).await.unwrap().unwrap();

    // Convert invoice amount to the payment option's currency
    let amount = convert(
        ConversionRequest {
            value: invoice.amount as f64,
            currency: invoice.currency.to_string(),
        },
        currency,
        Some(coin.precision.unwrap_or(8)),
    ).await?;

    tracing::info!("Converted amount {} {} to {} {}", 
        invoice.amount, 
        invoice.currency, 
        amount, 
        currency
    );

    let payment_amount = to_satoshis(ToSatoshisRequest {
        decimal: amount,
        currency: currency.to_string(),
        chain: chain.to_string(),
    }, supabase).await?;

    tracing::info!("Converted to satoshis: {}", payment_amount);

    let mut address = get_new_address(GetAddressRequest {
        account: account.clone(),
        address: address_record.clone(),
        currency: currency.to_string(),
        chain: chain.to_string(),
    }).await?;

    if address.contains(':') {
        address = address.split(':').nth(1).unwrap_or(&address).to_string();
    }

    let fee = get_fee(currency, payment_amount).await?;
    let mut outputs = Vec::new();

    if !["MATIC", "ETH", "AVAX"].contains(&chain) {
        let adjusted_amount = payment_amount - fee.amount;
        
        outputs.push(Output {
            address: address.clone(),
            amount: adjusted_amount,
        });
        
        outputs.push(Output {
            address: fee.address.clone(),
            amount: fee.amount,
        });
    } else {
        outputs.push(Output {
            address: address.clone(),
            amount: payment_amount,
        });
    }

    let uri = compute_invoice_uri(&InvoiceUriParams {
        currency: currency.to_string(),
        uid: invoice.id.to_string(),
    });

    let total_amount: i64 = outputs.iter().map(|output| output.amount).sum();

    let payment_option = PaymentOption {
        invoice_uid: invoice.uid.to_string(),
        currency: currency.to_string(),
        chain: chain.to_string(),
        amount: total_amount,
        address,
        outputs,
        uri,
        fee: fee.amount,
        createdAt: chrono::Utc::now(),
        updatedAt: chrono::Utc::now(),
    };

    Ok(Some(payment_option))
} 
