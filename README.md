# Anypay WebSocket Server

A real-time payment processing server that handles WebSocket connections, HTTP endpoints, and integrates with various payment networks.

## Features

- WebSocket server for real-time payment notifications
- HTTP API for payment processing
- Price conversion service with automatic updates
- XRPL integration
- AMQP support for message queuing
- Supabase integration for data storage

## Setup

1. Clone the repository
2. Copy `.env.example` to `.env` and configure:

```
SUPABASE_URL=your_supabase_url
SUPABASE_ANON_KEY=your_anon_key
SUPABASE_SERVICE_ROLE_KEY=your_service_role_key
AMQP_URL=optional_amqp_url
WEBSOCKET_HOST=127.0.0.1
WEBSOCKET_PORT=8080
HTTP_HOST=127.0.0.1
HTTP_PORT=3000
ETH_WSS_URL=optional_ethereum_websocket_url
AVAX_WSS_URL=optional_avalanche_websocket_url
BNB_WSS_URL=optional_bnb_websocket_url
POLYGON_WSS_URL=optional_polygon_websocket_url
XRPL_WSS_URL=optional_xrpl_websocket_url
```

3. Install dependencies:

```
cargo build
```

4. Run the server:

```
cargo run
```

## Services

### WebSocket Server
- Handles real-time payment notifications
- Price conversion
- Payment status updates

### HTTP Server
- Payment processing endpoints
- Price information
- Account management

### Price Service
- Automatic price updates every minute
- Multiple currency support
- Real-time conversion

### XRPL Integration
- XRP Ledger connection
- Payment monitoring
- Transaction processing

## Testing

Run the test suite:

```
cargo test
```

Test WebSocket price conversion:

```
python scripts/test_prices.py
```

## API Documentation

### WebSocket Messages

Price conversion:

```json
{
  "action": "convert_price",
  "quote_currency": "BTC",
  "base_currency": "USD",
  "quote_value": 1
}
```

### HTTP Endpoints

- `GET /prices` - Get current prices
- `POST /convert` - Convert between currencies
- `POST /invoices` - Create new invoice

## Development

1. Install development dependencies:

```
cargo install cargo-watch
```

2. Run in development mode:

```
cargo watch -x run
```

## Contributing

1. Fork the repository
2. Create a feature branch
3. Submit a pull request

## License

[Insert License Information]

## WebSocket API

Connect to `ws://localhost:8080` to interact with the server.

### Message Format
All messages follow this format:
```json
{
    "action": "action_name",
    "status": "success|error",
    ... additional fields
}
```

### Available Actions

#### Price Conversion
```json
// Request
{
    "action": "convert_price",
    "quote_currency": "BTC",
    "base_currency": "USD",
    "quote_value": 1
}

// Response
{
    "status": "success",
    "data": {
        "quote_currency": "BTC",
        "base_currency": "USD",
        "quote_value": 1,
        "base_value": 43000.00,
        "timestamp": "2024-01-01T12:00:00Z"
    }
}
```

#### List Prices
```json
// Request
{
    "action": "list_prices"
}

// Response
{
    "status": "success",
    "data": [
        {
            "id": "price_123",
            "currency": "BTC",
            "value": 43000.00,
            "createdAt": "2024-01-01T12:00:00Z",
            "updatedAt": "2024-01-01T12:00:00Z"
        }
    ]
}
```

#### Create Invoice
```json
// Request
{
    "action": "create_invoice",
    "amount": 1000,
    "currency": "USD",
    "account_id": 1,
    "webhook_url": "https://example.com/webhook",
    "redirect_url": "https://example.com/return",
    "memo": "Payment for services"
}

// Response
{
    "status": "success",
    "data": {
        "invoice": {
            "uid": "inv_123",
            "amount": 1000,
            "currency": "USD",
            "status": "unpaid",
            "created_at": "2024-01-01T12:00:00Z"
        }
    }
}
```

#### Fetch Invoice
```json
// Request
{
    "action": "fetch_invoice",
    "id": "inv_123"
}

// Response
{
    "status": "success",
    "data": {
        "uid": "inv_123",
        "amount": 1000,
        "currency": "USD",
        "status": "unpaid",
        "created_at": "2024-01-01T12:00:00Z"
    }
}
```

#### Subscribe to Events
```json
// Request
{
    "action": "subscribe",
    "type": "invoice|account|address",
    "id": "resource_id"
}

// Response
{
    "status": "success",
    "message": "Subscribed to invoice resource_id"
}

// Event Message
{
    "type": "invoice.updated",
    "data": {
        "id": "inv_123",
        "status": "paid",
        "updated_at": "2024-01-01T12:00:00Z"
    }
}
```

#### Unsubscribe from Events
```json
// Request
{
    "action": "unsubscribe",
    "type": "invoice|account|address",
    "id": "resource_id"
}

// Response
{
    "status": "success",
    "message": "Unsubscribed from invoice resource_id"
}
```

### Testing WebSocket API

The repository includes several test scripts to verify API functionality:

1. Test price conversion:
```bash
python scripts/test_prices.py
```

2. Test invoice creation:
```bash
python scripts/test_create_invoice.py
```

3. Test price listing:
```bash
python scripts/test_list_prices.py
```

4. Run comprehensive tests:
```bash
python scripts/test_daemon_comprehensive.py
```

### Event Types

The WebSocket server emits various events that you can subscribe to:

- `invoice.created` - New invoice created
- `invoice.updated` - Invoice status changed
- `payment.received` - Payment detected
- `price.updated` - Price update received

### Error Handling

Error responses follow this format:
```json
{
    "status": "error",
    "message": "Error description"
}
```

Common error scenarios:
- Invalid message format
- Resource not found
- Invalid subscription type
- Missing required fields
- Server error
