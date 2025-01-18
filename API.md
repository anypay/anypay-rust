# AnyPay API Documentation

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

### Event Types

The WebSocket server emits various events that you can subscribe to:

- `invoice.created` - New invoice created
- `invoice.updated` - Invoice status changed
- `payment.received` - Payment detected
- `price.updated` - Price update received

## HTTP API

### Endpoints

#### GET /prices
Get current prices for all supported currencies.

Response:
```json
{
    "prices": [
        {
            "currency": "BTC",
            "base": "USD",
            "value": 43000.00,
            "updatedAt": "2024-01-01T12:00:00Z",
            "source": "coinbase"
        }
    ]
}
```

#### POST /payment-requests
Create a new payment request.

Request:
```json
[{
    "currency": "BTC",
    "to": [{
        "address": "bc1qxy2kgdygjrsqtzq2n0yrf2493p83kkfjhx0wlh",
        "amount": 100.0,
        "currency": "USD"
    }]
}]
```

Response:
```json
{
    "invoice": {
        "uid": "inv_123",
        "uri": "pay:?r=https://api.anypayx.com/r/inv_123",
        "status": "unpaid",
        "currency": "USD",
        "amount": 100.0,
        "payment_options": {
            "payment_options": [{
                "chain": "BTC",
                "currency": "BTC",
                "instructions": [{
                    "outputs": [{
                        "address": "bc1qxy2kgdygjrsqtzq2n0yrf2493p83kkfjhx0wlh",
                        "amount": 0.0023
                    }]
                }]
            }]
        }
    }
}
```

#### POST /r/{uid}
Submit a payment for an invoice.

Request:
```json
{
    "chain": "BTC",
    "currency": "BTC",
    "transactions": [{
        "tx": "0200000001..."
    }]
}
```

Response:
```json
{
    "status": "success",
    "message": "Payment submitted successfully"
}
```

#### GET /i/{uid}
Get invoice details.

Response:
```json
{
    "invoice": {
        "uid": "inv_123",
        "uri": "pay:?r=https://api.anypayx.com/r/inv_123",
        "status": "unpaid",
        "currency": "USD",
        "amount": 100.0,
        "payment_options": {
            "payment_options": [...]
        }
    }
}
```

### Error Handling

Error responses follow this format:
```json
{
    "status": "error",
    "message": "Error description"
}
```

Common error scenarios:
- Invalid request format
- Resource not found
- Authentication failed
- Invalid payment data
- Server error

### Authentication

Most endpoints require Basic authentication:
- Username: Your API token
- Password: Empty string

Example:
```
Authorization: Basic YOUR_TOKEN_BASE64
``` 