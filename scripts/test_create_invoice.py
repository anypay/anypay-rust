#!/usr/bin/env python3

import asyncio
import websockets
import json
import logging

logging.basicConfig(
    level=logging.INFO,
    format='%(asctime)s - %(levelname)s - %(message)s'
)

async def test_create_invoice():
    uri = "ws://localhost:8080"
    logging.info("Connecting to WebSocket server...")
    
    async with websockets.connect(uri) as websocket:
        # Create a new invoice
        create_msg = {
            "action": "create_invoice",
            "amount": 1000,
            "currency": "USD",
            "account_id": 1
        }
        
        logging.info(f"Sending create invoice request: {create_msg}")
        await websocket.send(json.dumps(create_msg))
        
        response = await websocket.recv()
        logging.info(f"Received response: {response}")
        
        response_data = json.loads(response)
        
        # Validate response
        assert response_data["status"] == "success", f"Failed to create invoice: {response_data}"
        invoice = response_data["data"]
        assert invoice["amount"] == 1000, f"Incorrect amount: {invoice['amount']}"
        assert invoice["currency"] == "USD", f"Incorrect currency: {invoice['currency']}"
        assert invoice["status"] == "unpaid", f"Incorrect status: {invoice['status']}"
        
        # Verify we can fetch the created invoice
        fetch_msg = {
            "action": "fetch_invoice",
            "id": invoice["uid"]
        }
        
        logging.info(f"Fetching created invoice: {fetch_msg}")
        await websocket.send(json.dumps(fetch_msg))
        
        fetch_response = await websocket.recv()
        logging.info(f"Fetch response: {fetch_response}")
        
        fetch_data = json.loads(fetch_response)
        assert fetch_data["status"] == "success", f"Failed to fetch created invoice: {fetch_data}"
        fetched_invoice = fetch_data["data"]
        assert fetched_invoice["uid"] == invoice["uid"], "Invoice IDs don't match"

async def main():
    try:
        await test_create_invoice()
        logging.info("✅ Invoice creation test passed!")
    except Exception as e:
        logging.error(f"❌ Test failed: {e}")
        raise

if __name__ == "__main__":
    asyncio.run(main()) 