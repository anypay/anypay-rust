use axum::{
    routing::{get, post, delete},
    Router,
    extract::{Path, Json},
    http::StatusCode,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use uuid::Uuid;
use std::sync::Arc;

use crate::{supabase::SupabaseClient, types::PaymentOption};
use crate::types::{Invoice, Price, PaymentRequest};

// Request/Response types matching swagger spec
#[derive(Deserialize)]
pub struct CreateInvoiceRequest {
    amount: i64,
    currency: String,
    account_id: i64,
    redirect_url: Option<String>,
    webhook_url: Option<String>,
    wordpress_site_url: Option<String>,
    memo: Option<String>,
    email: Option<String>,
    external_id: Option<String>,
    business_id: Option<String>,
    location_id: Option<String>,
    register_id: Option<String>,
    required_fee_rate: Option<String>,
}

#[derive(Serialize)]
pub struct InvoiceResponse {
    pub invoice: Invoice,
    pub payment_options: Vec<PaymentOption>,
}

#[derive(Serialize)]
pub struct PricesResponse {
    prices: Vec<Price>,
}

pub struct HttpServer {
    supabase: Arc<SupabaseClient>,
}

impl HttpServer {
    pub fn new(supabase: Arc<SupabaseClient>) -> Self {
        Self { supabase }
    }

    pub fn router(&self) -> Router {
        let supabase = self.supabase.clone();

        Router::new()
            // Prices endpoint
            .route("/api/v1/prices", get({
                let supabase = supabase.clone();
                move |_: ()| async move {
                    match supabase.list_prices().await {
                        Ok(prices) => Ok(Json(PricesResponse { prices })),
                        Err(e) => {
                            tracing::error!("Error listing prices: {}", e);
                            Err(StatusCode::INTERNAL_SERVER_ERROR)
                        }
                    }
                }
            }))

            // Invoice endpoints
            .route("/api/v1/invoices/:invoice_id", get({
                let supabase = supabase.clone();
                move |Path(invoice_id): Path<String>| async move {
                    match supabase.get_invoice(&invoice_id, true).await {
                        Ok(Some(invoice)) => Ok(Json(InvoiceResponse { invoice, payment_options: todo!() })),
                        Ok(None) => Err(StatusCode::NOT_FOUND),
                        Err(e) => {
                            tracing::error!("Error fetching invoice: {}", e);
                            Err(StatusCode::INTERNAL_SERVER_ERROR)
                        }
                    }
                }
            }))
            .route("/api/v1/invoices", post(move |Json(payload): Json<CreateInvoiceRequest>| async move {
                match supabase.create_invoice(
                    payload.amount, 
                    &payload.currency, 
                    payload.account_id,  // TODO: Get real account_id
                    payload.webhook_url,
                    payload.redirect_url,
                    payload.memo
                ).await {
                    Ok(response) => {
                        let data = response.as_object().unwrap();
                        Ok(Json(InvoiceResponse { 
                            invoice: serde_json::from_value(data["invoice"].clone()).unwrap(),
                            payment_options: serde_json::from_value(data["payment_options"].clone()).unwrap(),
                        }))
                    },
                    Err(e) => {
                        tracing::error!("Error creating invoice: {}", e);
                        Err(StatusCode::INTERNAL_SERVER_ERROR)
                    }
                }
            }))
            .route("/invoices/:uid", delete(move |Path(uid): Path<String>| async move {
                // TODO: Implement invoice cancellation
                StatusCode::NOT_IMPLEMENTED
            }))

            // Payment platform routes
            .route("/r", post(move |Json(payload): Json<PaymentRequest>| async move {
                // TODO: Implement payment request creation
                tracing::info!("Creating payment request: {:?}", payload);
                Ok::<Json<serde_json::Value>, StatusCode>(Json(json!({
                    "status": "success",
                    "uid": Uuid::new_v4().to_string()
                })))
            }))
            .route("/r/:uid", 
                post(move |
                    Path(uid): Path<String>,
                | async move {
                    // TODO: Process payment for existing request
                    tracing::info!("Processing payment for {}", uid);
                    Ok::<Json<serde_json::Value>, StatusCode>(Json(json!({
                        "status": "processing"
                    })))
                })
                .delete(move |Path(uid): Path<String>| async move {
                    // TODO: Cancel payment request
                    tracing::info!("Cancelling payment request {}", uid);
                    StatusCode::OK
                })
            )
    }
}

