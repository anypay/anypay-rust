use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::net::{TcpListener, TcpStream};
use tokio_tungstenite::{
    accept_hdr_async,
    tungstenite::handshake::server::{Request, Response, ErrorResponse},
};
use futures::{StreamExt, SinkExt};
use uuid::Uuid;
use serde_json::json;

use crate::event_dispatcher::EventDispatcher;
use crate::payment_options::create_payment_options;
use crate::session::Session;
use crate::types::Message;
use crate::supabase::SupabaseClient;
use crate::prices::{ConversionRequest, convert};
use crate::invoices;

pub struct AnypayEventsServer {
    event_dispatcher: Arc<EventDispatcher>,
    sessions: Arc<RwLock<HashMap<Uuid, Session>>>,
    addr: String,
    supabase: Arc<SupabaseClient>,
}

impl AnypayEventsServer {
    pub fn new(addr: &str, supabase_url: &str, supabase_anon_key: &str, supabase_service_role_key: &str) -> Self {
        AnypayEventsServer {
            event_dispatcher: Arc::new(EventDispatcher::new()),
            sessions: Arc::new(RwLock::new(HashMap::new())),
            addr: addr.to_string(),
            supabase: Arc::new(SupabaseClient::new(supabase_url, supabase_anon_key, supabase_service_role_key)),
        }
    }

    pub async fn run(&self) -> Result<(), Box<dyn std::error::Error>> {
        let listener = TcpListener::bind(&self.addr).await?;
        tracing::info!("WebSocket server listening on: {}", self.addr);

        while let Ok((stream, addr)) = listener.accept().await {
            tracing::info!("New connection from: {}", addr);
            
            let event_dispatcher = self.event_dispatcher.clone();
            let sessions = self.sessions.clone();
            let supabase = self.supabase.clone();
            
            tokio::spawn(async move {
                if let Err(e) = Self::handle_connection(stream, event_dispatcher, sessions, supabase).await {
                    tracing::error!("Error handling connection: {}", e);
                }
            });
        }

        Ok(())
    }

    async fn handle_message(
        message: Message,
        session: &Session,
        event_dispatcher: &Arc<EventDispatcher>,
        supabase: &Arc<SupabaseClient>,
    ) -> serde_json::Value {
        println!("message in handle message: {:?}", message);
        match message {
            Message::Subscribe { sub_type, id } => {
                event_dispatcher.subscribe(session.clone(), &sub_type, &id).await;
                json!({
                    "status": "success",
                    "message": format!("Subscribed to {} {}", sub_type, id)
                })
            }
            Message::Unsubscribe { sub_type, id } => {
                event_dispatcher.unsubscribe(session.clone(), &sub_type, &id).await;
                json!({
                    "status": "success",
                    "message": format!("Unsubscribed from {} {}", sub_type, id)
                })
            }
            Message::FetchInvoice { id } => {
                tracing::info!("Fetching invoice with id: {}", id);
                match supabase.get_invoice(&id, true).await {
                    Ok(Some(invoice)) => json!({
                        "status": "success",
                        "data": {
                            "invoice": invoice.0,
                            "payment_options": invoice.1
                        }
                    }),
                    Ok(None) => json!({
                        "status": "error",
                        "message": "Invoice not found"
                    }),
                    Err(e) => json!({
                        "status": "error",
                        "message": format!("Error fetching invoice: {}", e)
                    }),
                }
            }
            Message::CreateInvoice { amount, currency, webhook_url, redirect_url, memo } => {
                if let Some(account_id) = session.account_id {
                    println!("account_id in create invoice: {:?}", account_id);
                    match invoices::create_invoice(
                        &supabase,
                        amount,
                        &currency,
                        account_id,
                        webhook_url,
                        redirect_url,
                        memo
                    ).await {
                        Ok(invoice) => json!({
                            "status": "success",
                            "data": invoice
                        }),
                        Err(e) => json!({
                            "status": "error",
                            "message": format!("Failed to create invoice: {}", e)
                        })
                    }
                } else {
                    json!({
                        "status": "error",
                        "message": "Unauthorized: API key required: See https://www.anypayx.com/developer/websockets/authentication"
                    })
                }
            }
            Message::ListPrices => {
                tracing::info!("Listing all prices");
                match supabase.list_prices().await {
                    Ok(prices) => json!({
                        "status": "success",
                        "data": prices
                    }),
                    Err(e) => json!({
                        "status": "error",
                        "message": format!("Error fetching prices: {}", e)
                    }),
                }
            }
            Message::ConvertPrice { quote_currency, base_currency, quote_value } => {
                let req = ConversionRequest {
                    quote_currency,
                    base_currency,
                    quote_value,
                };
                
                match convert(req, supabase).await {
                    // if ok log the result
                    Ok(result) => {
                        json!({
                        
                        "status": "success",
                        "data": result
                    })},
                    Err(e) => {
                        json!({
                            "status": "error",
                            "message": format!("Conversion failed: {}", e)
                        })
                    },
                }
            }
            Message::CancelInvoice { uid } => {
                if let Some(account_id) = session.account_id {
                    match supabase.cancel_invoice(&uid, account_id).await {
                        Ok(()) => json!({
                            "status": "success",
                            "message": "Invoice cancelled successfully"
                        }),
                        Err(e) => json!({
                            "status": "error",
                            "message": e.to_string()
                        })
                    }
                } else {
                    json!({
                        "status": "error",
                        "message": "Unauthorized"
                    })
                }
            }
            Message::Ping => {
                json!({
                    "type": "pong",
                    "status": "success",
                    "timestamp": chrono::Utc::now().timestamp()
                })
            },
        }
    }

    async fn handle_connection(
        stream: TcpStream,
        event_dispatcher: Arc<EventDispatcher>,
        sessions: Arc<RwLock<HashMap<Uuid, Session>>>,
        supabase: Arc<SupabaseClient>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
        let (sender, mut receiver) = futures::channel::mpsc::unbounded();
        let mut session = Session::new(Uuid::new_v4(), sender);
        let supabase_clone = supabase.clone();

        let ws_stream = accept_hdr_async(stream, |req: &Request, res: Response| {
            
            if let Some(auth) = req.headers().get("Authorization") {
                println!("Authorization: {:?}", auth);
                if let Ok(auth_str) = auth.to_str() {
                    println!("Authorization string: {:?}", auth_str);
                    if auth_str.starts_with("Bearer ") {
                        let token = auth_str[7..].trim().to_string();
                        // Store token in session for async validation after handshake
                        println!("Token: {:?}", token);
                        session.auth_token = Some(token);
                    }
                }
            }
            Ok(res)
        }).await?;

        // Validate token after handshake
        if let Some(token) = &session.auth_token {
            println!("session.auth_token: {:?}", token);
            if let Ok(Some(account_id)) = supabase_clone.validate_api_key(token).await {
                println!("Account ID: {:?}", account_id);
                session.set_account_id(account_id);
                tracing::info!("Authenticated session {} for account {}", session.id, account_id);
            }
        }

        let (mut ws_sender, mut ws_receiver) = ws_stream.split();
        let (sender, mut receiver) = futures::channel::mpsc::unbounded();
        session.sender = Some(sender).unwrap();

        // Store the session
        sessions.write().await.insert(session.id, session.clone());

        // Create a flag to track connection state
        let is_connected = Arc::new(std::sync::atomic::AtomicBool::new(true));
        let is_connected_clone = is_connected.clone();

        // Spawn a task to forward messages from the channel to the websocket
        let _send_task = tokio::spawn(async move {
            while let Some(message) = receiver.next().await {
                if !is_connected_clone.load(std::sync::atomic::Ordering::SeqCst) {
                    break;
                }
                if let Err(e) = ws_sender.send(message).await {
                    tracing::debug!("Connection closed by client: {}", e);
                    break;
                }
            }
        });

        // Handle incoming messages
        while let Some(msg) = ws_receiver.next().await {
            match msg {
                Ok(msg) => {
                    if let Ok(text) = msg.to_text() {
                        println!("text in handle connection: {:?}", text);
                        let response = match serde_json::from_str::<Message>(text) {
                            Ok(message) => {
                                Self::handle_message(
                                    message,
                                    &session,
                                    &event_dispatcher,
                                    &supabase,
                                ).await
                            }
                            Err(_) => json!({
                                "status": "error",
                                "message": "Invalid message format"
                            })
                        };

                        if let Err(e) = session.send(tokio_tungstenite::tungstenite::Message::Text(response.to_string().into())) {
                            tracing::debug!("Failed to send response, client likely disconnected: {}", e);
                            break;
                        }
                    }
                }
                Err(e) => {
                    tracing::debug!("WebSocket error: {}", e);
                    break;
                }
            }
        }

        // Mark connection as closed
        is_connected.store(false, std::sync::atomic::Ordering::SeqCst);
        
        // Clean up session
        sessions.write().await.remove(&session.id);
        tracing::info!("Connection closed for session: {}", session.id);
        
        Ok(())
    }
} 