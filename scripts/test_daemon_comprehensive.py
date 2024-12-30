#!/usr/bin/env python3

import asyncio
import websockets
import json
import logging
import random
from datetime import datetime
import uuid

logging.basicConfig(
    level=logging.INFO,
    format='%(asctime)s - %(levelname)s - %(message)s'
)

class WebSocketClient:
    def __init__(self, uri):
        self.uri = uri
        self.websocket = None
    
    async def connect(self):
        self.websocket = await websockets.connect(self.uri)
        return self
    
    async def subscribe(self, sub_type, sub_id):
        message = {
            "action": "subscribe",
            "type": sub_type,
            "id": sub_id
        }
        await self.websocket.send(json.dumps(message))
        response = await self.websocket.recv()
        return response
    
    async def unsubscribe(self, sub_type, sub_id):
        message = {
            "action": "unsubscribe",
            "type": sub_type,
            "id": sub_id
        }
        await self.websocket.send(json.dumps(message))
        response = await self.websocket.recv()
        return response
    
    async def fetch_invoice(self, invoice_id):
        message = {
            "action": "fetch_invoice",
            "id": invoice_id
        }
        await self.websocket.send(json.dumps(message))
        response = await self.websocket.recv()
        return response
    
    async def close(self):
        if self.websocket:
            await self.websocket.close()

async def test_basic_functionality():
    """Test basic subscribe/unsubscribe functionality."""
    client = await WebSocketClient("ws://localhost:8080").connect()
    
    try:
        # Test subscribe
        response = await client.subscribe("invoice", "test_inv_1")
        logging.info(f"Subscribe response: {response}")
        
        # Wait a bit
        await asyncio.sleep(2)
        
        # Test unsubscribe
        response = await client.unsubscribe("invoice", "test_inv_1")
        logging.info(f"Unsubscribe response: {response}")
    
    finally:
        await client.close()

async def test_invoice_fetch():
    """Test invoice fetching functionality."""
    client = await WebSocketClient("ws://localhost:8080").connect()
    
    try:
        # Test fetching a valid invoice
        response = await client.fetch_invoice("test_inv_1")
        logging.info(f"Fetch invoice response: {response}")
        
        # Parse the response
        response_data = json.loads(response)
        assert response_data["status"] in ["success", "error"], "Response should have a status"
        
        if response_data["status"] == "success":
            assert "data" in response_data, "Success response should contain invoice data"
            invoice = response_data["data"]
            assert "id" in invoice, "Invoice should have an ID"
            assert "amount" in invoice, "Invoice should have an amount"
            assert "currency" in invoice, "Invoice should have a currency"
            assert "status" in invoice, "Invoice should have a status"
        
        # Test fetching a non-existent invoice
        response = await client.fetch_invoice("non_existent_invoice")
        logging.info(f"Fetch non-existent invoice response: {response}")
        response_data = json.loads(response)
        assert response_data["status"] == "error", "Non-existent invoice should return error"
        
    finally:
        await client.close()

async def test_error_cases():
    """Test various error cases."""
    client = await WebSocketClient("ws://localhost:8080").connect()
    
    try:
        # Test invalid message format
        await client.websocket.send("invalid json")
        response = await client.websocket.recv()
        logging.info(f"Invalid JSON response: {response}")
        
        # Test invalid subscription type
        response = await client.subscribe("invalid_type", "test_id")
        logging.info(f"Invalid type response: {response}")
        
        # Test missing fields
        await client.websocket.send(json.dumps({"action": "subscribe"}))
        response = await client.websocket.recv()
        logging.info(f"Missing fields response: {response}")
    
    finally:
        await client.close()

async def test_concurrent_connections():
    """Test multiple concurrent connections."""
    num_clients = 5
    clients = []
    
    try:
        # Create multiple clients
        for i in range(num_clients):
            client = await WebSocketClient("ws://localhost:8080").connect()
            clients.append(client)
            
            # Subscribe to random event types
            event_types = ["invoice", "account", "address"]
            event_type = random.choice(event_types)
            event_id = f"test_{event_type}_{i}"
            
            response = await client.subscribe(event_type, event_id)
            logging.info(f"Client {i} subscribe response: {response}")
        
        # Keep connections open for a bit
        await asyncio.sleep(5)
    
    finally:
        # Cleanup
        for client in clients:
            await client.close()

async def test_create_invoice():
    uri = "ws://localhost:8080"
    async with websockets.connect(uri) as websocket:
        # Create a new invoice
        create_msg = {
            "action": "create_invoice",
            "amount": 1000,
            "currency": "USD",
            "account_id": 1
        }
        await websocket.send(json.dumps(create_msg))
        response = json.loads(await websocket.recv())
        
        assert response["status"] == "success", f"Failed to create invoice: {response}"
        invoice = response["data"]
        assert invoice["amount"] == 1000
        assert invoice["currency"] == "USD"
        assert invoice["status"] == "pending"
        
        # Try to fetch the created invoice
        fetch_msg = {
            "action": "fetch_invoice",
            "id": invoice["uid"]
        }
        await websocket.send(json.dumps(fetch_msg))
        response = json.loads(await websocket.recv())
        
        assert response["status"] == "success", f"Failed to fetch created invoice: {response}"
        fetched_invoice = response["data"]
        assert fetched_invoice["uid"] == invoice["uid"]

async def run_all_tests():
    """Run all test cases."""
    logging.info("Starting comprehensive WebSocket daemon tests")
    
    try:
        logging.info("Testing basic functionality...")
        await test_basic_functionality()
        
        logging.info("Testing invoice fetching...")
        await test_invoice_fetch()
        
        logging.info("Testing error cases...")
        await test_error_cases()
        
        logging.info("Testing concurrent connections...")
        await test_concurrent_connections()
        
        logging.info("Testing invoice creation...")
        await test_create_invoice()
        
        logging.info("All tests completed successfully")
    
    except websockets.exceptions.ConnectionRefused:
        logging.error("Could not connect to the daemon service. Is it running?")
    except Exception as e:
        logging.error(f"Test failed: {e}")

def main():
    try:
        asyncio.run(run_all_tests())
    except KeyboardInterrupt:
        logging.info("Tests terminated by user")

if __name__ == "__main__":
    main() 