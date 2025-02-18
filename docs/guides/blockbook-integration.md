# 🚀 Anypay Blockbook Integration Guide

## Overview
This guide explains how Anypay integrates with Blockbook to monitor blockchain transactions and automatically confirm payments. We'll cover everything from setup to implementation details.

## 🔧 Setup & Configuration

### Command Line Options
```bash
# Start server with Blockbook integration
anypay-server \
  --blockbook-url "btcbook.nownodes.io" \
  --blockbook-api-key "your-api-key" \
  --supabase-url "your-supabase-url" \
  --supabase-anon-key "your-anon-key" \
  --supabase-service-role-key "your-service-key"

# Enable debug logging
anypay-server --debug
```

### Environment Variables
```bash
BLOCKBOOK_WS_URL=btcbook.nownodes.io
BLOCKBOOK_API_KEY=your-api-key
SUPABASE_URL=your-supabase-url
SUPABASE_ANON_KEY=your-anon-key
SUPABASE_SERVICE_ROLE_KEY=your-service-key
```

## 📡 Blockbook WebSocket API

### Connection
```rust
// Format: wss://btcbook.nownodes.io/wss/{api_key}
let url = format!("wss://{}/{}", ws_url, api_key);
```

### Subscribe to New Blocks
```json
// Subscribe Request
{
    "id": "1",
    "method": "subscribeNewBlock",
    "params": []
}

// Response
{
    "id": "1",
    "data": {
        "subscribed": true
    }
}

// Block Notification
{
    "id": "1",
    "data": {
        "blockHash": "000000000000000000046dc3641e0929b5c8e310055d4c3fe936ed8e2c4d5050",
        "height": 789123,
        "timestamp": 1678901234
    }
}
```

## 🔍 Block Processing Flow

1. **Subscribe to Blocks**
```rust
// BlockbookClient establishes WebSocket connection
let blockbook = BlockbookClient::new(ws_url, api_key, supabase);
let handle = blockbook.start_subscription().await?;
```

2. **Process New Blocks**
```rust
// When new block arrives:
async fn process_block(&self, block: &BlockNotification) -> Result<()> {
    // 1. Fetch block transactions
    let txids = self.get_block_txids(&block.block_hash).await?;
    
    // 2. Check each transaction
    for txid in txids {
        // 3. Look for matching unconfirmed payments
        if let Some(payment) = self.supabase
            .get_unconfirmed_payment_by_txid(&txid).await? {
            // 4. Create confirmation
            let confirmation = Confirmation {
                confirmation_hash: block.block_hash.clone(),
                confirmation_height: block.height as i32,
                confirmation_date: chrono::Utc::now(),
                confirmations: Some(1),
            };
            
            // 5. Confirm payment
            self.confirm_payment(payment, confirmation).await?;
        }
    }
}
```

## 💾 Database Integration

### Payment States
- `unconfirmed`: Initial state when payment is received
- `confirming`: Payment found in block, being processed
- `confirmed`: Payment fully confirmed
- `failed`: Payment failed or reverted (EVM chains)

### Supabase Tables
```sql
-- payments table
create table payments (
  id serial primary key,
  txid text not null,
  chain text not null,
  currency text not null,
  status text not null,
  invoice_uid text not null,
  confirmation_hash text,
  confirmation_height integer,
  confirmation_date timestamp
);

-- indexes
create index payments_txid_idx on payments(txid);
create index unconfirmed_payments_idx on payments(chain, currency) 
where confirmation_hash is null;
```

## 🎯 Payment Confirmation Process

1. **Block Reception**
   - Receive block notification via WebSocket
   - Extract block hash, height, and timestamp

2. **Transaction Lookup**
   - Fetch full block details from Blockbook API
   - Get list of all transactions in block

3. **Payment Matching**
   - Query database for unconfirmed payments matching txids
   - Create confirmation records for matches

4. **Update Records**
   - Update payment status to "confirmed"
   - Update associated invoice status
   - Store confirmation details

5. **Notifications**
   - Send webhook notifications to merchants
   - Publish confirmation events

## 🔄 Graceful Shutdown
```rust
// Handle Ctrl+C
tokio::select! {
    _ = signal::ctrl_c() => {
        info!("Received shutdown signal");
        if let Some(handle) = blockbook_handle {
            handle.shutdown().await;
        }
    }
}
```

## 📊 Monitoring & Debugging

### Log Levels
```rust
// Debug logging
tracing::debug!("Processing block {} at height {}", block.hash, block.height);

// Info for important events
tracing::info!("Confirmed payment for txid {}", txid);

// Errors
tracing::error!("Failed to confirm payment: {}", error);
```

### Metrics to Monitor
- Block processing time
- Payment confirmation latency
- WebSocket connection stability
- Error rates
- Unconfirmed payment count

## 🚨 Error Handling
- WebSocket disconnections
- API rate limits
- Database connectivity
- Invalid block data
- Payment validation failures

## 🔮 Future Improvements
1. Reorg handling
2. Multi-chain support
3. Confirmation threshold configuration
4. Payment batching
5. Enhanced metrics and monitoring
6. Automated retries
7. Performance optimizations

## 🎓 Best Practices
1. Always verify transaction data
2. Implement proper error handling
3. Monitor connection health
4. Log important events
5. Handle shutdowns gracefully
6. Keep security in mind
7. Regular testing and maintenance

This integration provides robust, real-time payment confirmations while maintaining high reliability and performance. 🎉
