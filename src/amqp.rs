//! AMQP Client Tutorial
//! 
//! This module demonstrates how to implement an AMQP (Advanced Message Queuing Protocol) client
//! in Rust using async/await patterns. AMQP is a messaging protocol that enables applications
//! to communicate through message brokers like RabbitMQ.
//! 
//! //! # Getting Started with CloudAMQP
//! 
//! 1. Sign up for a free account at https://www.cloudamqp.com/
//!    - Click "Sign Up"
//!    - Choose the "Little Lemur" (free) plan
//!    - Select a region close to your application
//!
//! 2. Create a new instance:
//!    ```plaintext
//!    Name: my-app-events
//!    Plan: Little Lemur (Free)
//!    Region: (closest to you)
//!    Tags: development
//!    ```
//!
//! 3. Get your connection URL:
//!    - Click on your instance
//!    - Look for "AMQP URL" - it looks like:
//!    ```plaintext
//!    amqps://username:password@hostname/instance
//!    ```
//! 
//! 4. Add to your .env file:
//!    ```plaintext
//!    AMQP_URL=amqps://username:password@hostname/instance
//!    ```
//! 
//! # Key Concepts
//!
//! ## AMQP Components
//! - Exchange: A message routing agent that receives messages and routes them to queues
//! - Queue: A buffer that stores messages
//! - Binding: Rules that tell exchanges which queues to route messages to
//! - Consumer: Application that receives messages from queues
//!
//! ## Rust Async Patterns Used
//! 1. Arc (Atomic Reference Counting):
//!    ```rust
//!    use std::sync::Arc;
//!    let shared_data = Arc::new(data); // Share data across threads safely
//!    ```
//!
//! 2. Mutex for Async:
//!    ```rust
//!    use tokio::sync::Mutex;
//!    let protected_data = Arc::new(Mutex::new(data));
//!    // In async function:
//!    let mut lock = protected_data.lock().await;
//!    ```
//!
//! 3. Tokio Spawn:
//!    ```rust
//!    tokio::spawn(async move {
//!        // Run this code in a separate task
//!    });
//!    ```
//!
//! # Example Usage
//! ```rust
//! let amqp_client = AmqpClient::new("amqp://localhost").await?;
//! // The client will automatically:
//! // 1. Connect to AMQP server
//! // 2. Create/verify exchange
//! // 3. Create anonymous queue
//! // 4. Bind queue to exchange
//! // 5. Start consuming messages
//! ```
//!
//! # Message Flow
//! ```plaintext
//! Publisher -> Exchange -> Queue -> Consumer
//!                    ^
//!                    |
//!              Binding (#)
//! ```
//!
//! # Implementation Details
//! - We use a topic exchange which allows routing based on patterns
//! - The "#" binding means "receive all messages"
//! - Messages are acknowledged after processing
//! - Connection and channel are managed automatically
//!
//! # Error Handling
//! The implementation uses Rust's Result type for error handling:
//! ```rust
//! pub async fn new(url: &str) -> Result<Self, Box<dyn std::error::Error>>
//! ```
//! This allows callers to handle connection, channel, and messaging errors appropriately.

use lapin::{
    options::*, types::FieldTable, BasicProperties, Channel, Connection, ConnectionProperties, Consumer
};
use serde_json::json;
use std::sync::Arc;
use tokio::sync::Mutex;
use futures::StreamExt;

pub struct AmqpClient {
    channel: Arc<Mutex<Channel>>,
}

impl AmqpClient {
    pub async fn new(url: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let conn = Connection::connect(
            url,
            ConnectionProperties::default(),
        ).await?;

        let channel = conn.create_channel().await?;
        
        // Declare exchange if it doesn't exist
        channel
            .exchange_declare(
                "events",
                lapin::ExchangeKind::Topic,
                ExchangeDeclareOptions::default(),
                FieldTable::default(),
            )
            .await?;

        // Subscribe to all events
        let queue = channel
            .queue_declare(
                "",
                QueueDeclareOptions::default(),
                FieldTable::default(),
            )
            .await?;

        channel
            .queue_bind(
                queue.name().as_str(),  // Convert ShortString to &str
                "events",
                "#",  // Subscribe to all topics
                QueueBindOptions::default(),
                FieldTable::default(),
            )
            .await?;

        let consumer = channel
            .basic_consume(
                queue.name().as_str(),  // Convert ShortString to &str
                "event-logger",
                BasicConsumeOptions::default(),
                FieldTable::default(),
            )
            .await?;

        // Start consuming events
        tokio::spawn(async move {
            consume_events(consumer).await;
        });

        Ok(Self {
            channel: Arc::new(Mutex::new(channel)),
        })
    }

    async fn publish(&self, routing_key: &str, payload: &serde_json::Value) -> Result<(), Box<dyn std::error::Error>> {
        let channel = self.channel.lock().await;
        channel
            .basic_publish(
                "events",
                routing_key,
                BasicPublishOptions::default(),
                &serde_json::to_vec(payload)?,
                BasicProperties::default(),
            )
            .await?;
        Ok(())
    }

    pub async fn publish_invoice_created(
        &self,
        uid: &str,
        amount: i64,
        currency: &str,
        account_id: i32,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let payload = json!({
            "type": "invoice.created",
            "data": {
                "uid": uid,
                "amount": amount,
                "currency": currency,
                "account_id": account_id
            }
        });

        self.publish("invoice.created", &payload).await?;
        tracing::info!("Published invoice.created event for invoice {}", uid);
        Ok(())
    }
}

async fn consume_events(mut consumer: Consumer) {
    while let Some(delivery) = consumer.next().await {
        if let Ok(delivery) = delivery {
            if let Ok(data) = std::str::from_utf8(&delivery.data) {
                tracing::info!("AMQP Event: {}", data);
            }
            delivery.ack(BasicAckOptions::default()).await.ok();
        }
    }
} 