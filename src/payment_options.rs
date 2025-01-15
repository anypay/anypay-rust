use serde::{Deserialize, Serialize};
use anyhow::{Result, anyhow};
use crate::types::{Invoice, PaymentOption, Output, Account, Address};
use crate::payment::{
    convert, get_fee, get_new_address, to_satoshis, ConversionRequest, GetAddressRequest, ToSatoshisRequest
};
use crate::uri::{compute_invoice_uri, InvoiceUriParams};
use crate::supabase::SupabaseClient;
use futures::future::join_all;
use chrono::{Duration, Utc};

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

    let addresses = supabase.list_available_addresses(account).await.map_err(|e| anyhow!("Failed to list addresses: {}", e))?;
    tracing::info!("Listed available addresses: {:?}", addresses);

    let mut payment_options = Vec::new();

    // Process each address in parallel
    let option_futures = addresses.into_iter().map(|address_record| {
        let chain = address_record.chain.clone();
        let currency = address_record.currency.clone();
        let account = account.clone();
        let invoice = invoice.clone();
        let supabase = supabase.clone();

        async move {
            match build_payment_option(
                &account,
                &invoice,
                &address_record,
                &chain,
                &currency,
                &supabase,
            ).await {
                Ok(Some(option)) => Some(option),
                _ => None
            }
        }
    });

    // Wait for all payment options to be processed
    let results = join_all(option_futures).await;
    
    // Filter out None values and collect into payment_options
    for result in results {
        if let Some(option) = result {
            payment_options.push(option);
        }
    }

    // Create all payment options in the database
    if !payment_options.is_empty() {
        let inserted_options = supabase.create_payment_options(&payment_options).await.map_err(|e| anyhow!("Failed to create payment options: {}", e))?;
        return Ok(inserted_options);
    }

    Ok(Vec::new())
}

async fn build_payment_option(
    account: &Account,
    invoice: &Invoice,
    address_record: &Address,
    chain: &str,
    currency: &str,
    supabase: &SupabaseClient,
) -> Result<Option<PaymentOption>> {
    // Get coin info for precision
    let coin = supabase.get_coin(currency, chain).await.map_err(|e| anyhow!("Failed to get coin: {}", e))?.ok_or_else(|| anyhow!("Coin not found"))?;

    println!("coin: {:?}", coin);
    // Convert invoice amount to payment currency
    let account_denomination = account.denomination.as_deref().unwrap_or("USD");
    println!("account_denomination: {:?}", account_denomination);

    let conversion_request = crate::prices::ConversionRequest {
        quote_currency: account_denomination.to_string(),
        base_currency: currency.to_string(),
        quote_value: invoice.amount as f64,
    };

    println!("conversion_request: {:?}", conversion_request);

    let conversion = crate::prices::convert(
        conversion_request,
        supabase,
    ).await?;

    let amount = conversion.base_value;
    println!("amount: {:?}", amount);

    tracing::info!(
        "Converting {} {} to {} {}: {}",
        invoice.amount,
        account_denomination,
        amount,
        currency,
        amount
    );

    // Get payment address
    let mut address = get_new_address(GetAddressRequest {
        account: account.clone(),
        address: address_record.clone(),
        currency: currency.to_string(),
        chain: chain.to_string(),
    }).await?;

    // Clean up address if needed
    if address.contains(':') {
        address = address.split(':').nth(1).unwrap_or(&address).to_string();
    }

    // Convert to smallest unit (satoshis/wei/etc)
    let payment_amount = to_satoshis(ToSatoshisRequest {
        decimal: amount,
        currency: currency.to_string(),
        chain: chain.to_string(),
    }, supabase).await?;

    tracing::info!(
        "Converted {} {} to {} satoshis",
        amount,
        currency,
        payment_amount
    );

    // Calculate fee and outputs
    let fee = get_fee(currency, payment_amount).await?;
    let mut outputs = Vec::new();

    // Single output for all chains
    outputs.push(Output {
        address: address.clone(),
        amount: payment_amount,
    });

    // Compute payment URI
    let uri = compute_invoice_uri(&InvoiceUriParams {
        currency: currency.to_string(),
        uid: invoice.uid.clone(),
    });

    // Total amount is just the payment amount
    let total_amount = payment_amount;

    // Create payment option
    let now = Utc::now();
    let expires_at = now + Duration::minutes(15); // 15 minute expiry
    let payment_option = PaymentOption {
        invoice_uid: invoice.uid.clone(),
        currency: currency.to_string(),
        chain: chain.to_string(),
        amount: total_amount,
        address,
        outputs,
        uri,
        fee: fee.amount,
        created_at: now.to_rfc3339(),
        updated_at: now.to_rfc3339(),
        expires: expires_at.to_rfc3339(),
    };

    Ok(Some(payment_option))
} 

pub async fn refresh_payment_option(
    payment_option: &PaymentOption,
    invoice: &Invoice,
    account: &Account,
    supabase: &SupabaseClient,
) -> Result<PaymentOption> {
    // Get coin info for precision
    let coin = supabase.get_coin(&payment_option.currency, &payment_option.chain)
        .await.map_err(|e| anyhow!("Failed to get coin: {}", e))?
        .ok_or_else(|| anyhow!("Coin not found"))?;

    // Convert invoice amount to payment currency
    let account_denomination = account.denomination.as_deref().unwrap_or("USD");

    let conversion_request = crate::prices::ConversionRequest {
        quote_currency: account_denomination.to_string(),
        base_currency: payment_option.currency.to_string(),
        quote_value: invoice.amount as f64,
    };

    let conversion = crate::prices::convert(
        conversion_request,
        supabase,
    ).await?;

    let amount = conversion.base_value;

    // Convert to smallest unit (satoshis/wei/etc)
    let payment_amount = to_satoshis(ToSatoshisRequest {
        decimal: amount,
        currency: payment_option.currency.to_string(),
        chain: payment_option.chain.to_string(),
    }, supabase).await?;

    // Calculate fee
    let fee = get_fee(&payment_option.currency, payment_amount).await?;

    // Create single output with new amount
    let outputs = vec![Output {
        address: payment_option.address.clone(),
        amount: payment_amount,
    }];

    // Create updated payment option
    let now = Utc::now();
    let expires_at = now + Duration::minutes(15); // 15 minute expiry
    let updated = PaymentOption {
        invoice_uid: payment_option.invoice_uid.clone(),
        currency: payment_option.currency.clone(),
        chain: payment_option.chain.clone(),
        amount: payment_amount,
        address: payment_option.address.clone(),
        outputs,
        uri: payment_option.uri.clone(),
        fee: fee.amount,
        created_at: payment_option.created_at.clone(),
        updated_at: now.to_rfc3339(),
        expires: expires_at.to_rfc3339(),
    };

    Ok(updated)
}

pub async fn is_payment_option_expired(payment_option: &PaymentOption) -> bool {
    // Parse the expires string into a DateTime
    if let Ok(expires) = chrono::DateTime::parse_from_rfc3339(&payment_option.expires) {
        expires.with_timezone(&Utc) < Utc::now()
    } else {
        // If we can't parse the date, consider it expired
        true
    }
}

pub async fn update_expired_payment_options(
    invoice: &Invoice,
    payment_options: Vec<PaymentOption>,
    account: &Account,
    supabase: &SupabaseClient,
) -> Result<Vec<PaymentOption>> {
    let mut updated_options = Vec::new();
    tracing::info!("Updating expired payment options");

    for option in payment_options {
        if is_payment_option_expired(&option).await {
            tracing::info!("Payment option expired: {:?}", option);
            let refreshed = refresh_payment_option(&option, invoice, account, supabase).await?;
            updated_options.push(refreshed);
        } else {
            tracing::info!("Payment option not expired: {:?}", option);
            updated_options.push(option);
        }
    }

    // Update payment options in database
    if !updated_options.is_empty() {
        tracing::info!("Updating payment options in database");
        updated_options = supabase.create_payment_options(&updated_options)
            .await.map_err(|e| anyhow!("Failed to update payment options: {}", e))?;
    }

    Ok(updated_options)
} 
