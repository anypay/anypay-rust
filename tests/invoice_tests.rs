use anypay_websockets::{
    supabase::SupabaseClient,
    types::{Account, Invoice, PaymentOption},
    payment_options::create_payment_options,
    payment_options::update_expired_payment_options,
};
use std::env;
use dotenv::dotenv;

fn setup_supabase() -> SupabaseClient {
    dotenv().ok();
    let url = env::var("SUPABASE_URL").expect("SUPABASE_URL must be set");
    let anon_key = env::var("SUPABASE_ANON_KEY").expect("SUPABASE_ANON_KEY must be set");
    let service_role_key = env::var("SUPABASE_SERVICE_ROLE_KEY").expect("SUPABASE_SERVICE_ROLE_KEY must be set");
    
    SupabaseClient::new(&url, &anon_key, &service_role_key)
}

fn create_test_invoice() -> Invoice {
    Invoice {
        id: 1,
        uid: format!("inv_{}", uuid::Uuid::new_v4()),
        amount: 100000, // $1000.00
        currency: "USD".to_string(),
        status: "unpaid".to_string(),
        account_id: 1,
        complete: Some(false),
        webhook_url: Some("https://example.com/webhook".to_string()),
        redirect_url: Some("https://example.com/return".to_string()),
        memo: Some("Test invoice".to_string()),
        uri: format!("pay:?r=https://api.anypayx.com/r/{}", uuid::Uuid::new_v4()),
        createdAt: chrono::Utc::now().to_rfc3339(),
        updatedAt: chrono::Utc::now().to_rfc3339(),
    }
}

fn create_test_account() -> Account {
    Account {
        id: 1,
        denomination: Some("USD".to_string()),
    }
}

#[tokio::test]
async fn test_create_payment_options() {
    let supabase = setup_supabase();
    let invoice = create_test_invoice();
    let account = create_test_account();
    
    let payment_options = create_payment_options(&account, &invoice, &supabase)
        .await
        .expect("Failed to create payment options");
    
    assert!(!payment_options.is_empty(), "Should have created at least one payment option");
    
    for option in &payment_options {
        verify_payment_option(option, &invoice);
    }
}

fn verify_payment_option(option: &PaymentOption, invoice: &Invoice) {
    // Basic fields
    assert!(!option.invoice_uid.is_empty(), "Payment option should have invoice_uid");
    assert!(!option.currency.is_empty(), "Payment option should have currency");
    assert!(!option.chain.is_empty(), "Payment option should have chain");
    assert!(!option.address.is_empty(), "Payment option should have address");
    assert!(!option.uri.is_empty(), "Payment option should have URI");
    
    // Amount and fee
    assert!(option.amount > 0, "Payment option amount should be > 0");
    assert!(option.fee >= 0, "Payment option fee should be >= 0");
    
    // Outputs
    assert!(!option.outputs.is_empty(), "Payment option should have outputs");
    let total_output_amount: i64 = option.outputs.iter().map(|o| o.amount).sum();
    assert_eq!(total_output_amount, option.amount, "Sum of outputs should equal payment option amount");
    
    // Timestamps
    assert!(!option.created_at.is_empty(), "Payment option should have created_at");
    assert!(!option.updated_at.is_empty(), "Payment option should have updated_at");
    assert!(!option.expires.is_empty(), "Payment option should have expires");
    
    // Verify each output
    for output in &option.outputs {
        assert!(output.amount > 0, "Output amount should be > 0");
        assert!(!output.address.is_empty(), "Output should have address");
    }
    
    // Verify single output for all chains
    assert_eq!(option.outputs.len(), 1, "Should have exactly 1 output");
    assert_eq!(option.outputs[0].amount, option.amount, "Output amount should equal total amount");
}

#[tokio::test]
async fn test_payment_option_expiry() {
    let supabase = setup_supabase();
    let invoice = create_test_invoice();
    let account = create_test_account();
    
    // Create initial payment options
    let initial_options = create_payment_options(&account, &invoice, &supabase)
        .await
        .expect("Failed to create initial payment options");
    
    assert!(!initial_options.is_empty(), "Should have created initial payment options");
    
    // Store initial amounts for comparison
    let initial_amounts: std::collections::HashMap<String, i64> = initial_options
        .iter()
        .map(|opt| (format!("{}:{}", opt.currency, opt.chain), opt.amount))
        .collect();
    
    // Refresh payment options
    let refreshed_options = update_expired_payment_options(
        &invoice,
        initial_options,
        &account,
        &supabase
    ).await.expect("Failed to refresh payment options");
    
    assert_eq!(
        refreshed_options.len(),
        initial_amounts.len(),
        "Should have same number of payment options after refresh"
    );
    
    // Verify refreshed options
    for option in &refreshed_options {
        verify_payment_option(option, &invoice);
        
        // Check if amount changed due to price updates
        let key = format!("{}:{}", option.currency, option.chain);
        if let Some(&initial_amount) = initial_amounts.get(&key) {
            if option.amount != initial_amount {
                println!("Amount changed for {}:", key);
                println!("  Initial: {}", initial_amount);
                println!("  Updated: {}", option.amount);
            }
        }
    }
} 