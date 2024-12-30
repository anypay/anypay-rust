#!/usr/bin/env python3

import asyncio
import websockets
import json
import logging

logging.basicConfig(
    level=logging.INFO,
    format='%(asctime)s - %(levelname)s - %(message)s'
)

async def test_list_prices():
    uri = "ws://localhost:8080"
    logging.info("Connecting to WebSocket server...")
    
    async with websockets.connect(uri) as websocket:
        # List all prices
        list_msg = {
            "action": "list_prices"
        }
        
        logging.info(f"Sending list prices request: {list_msg}")
        await websocket.send(json.dumps(list_msg))
        
        response = await websocket.recv()
        logging.info(f"Received response: {response}")
        
        response_data = json.loads(response)
        
        # Validate response
        assert response_data["status"] == "success", f"Failed to list prices: {response_data}"
        prices = response_data["data"]
        assert isinstance(prices, list), "Prices should be a list"
        
        # If we have prices, validate their structure
        if prices:
            price = prices[0]
            required_fields = ["id", "currency", "value", "createdAt", "updatedAt"]
            for field in required_fields:
                assert field in price, f"Price missing required field: {field}"
            
            logging.info(f"Found {len(prices)} prices")
            for price in prices:
                logging.info(f"Price: {price['value']} {price['currency']}")

async def main():
    try:
        await test_list_prices()
        logging.info("✅ Price listing test passed!")
    except Exception as e:
        logging.error(f"❌ Test failed: {e}")
        raise

if __name__ == "__main__":
    asyncio.run(main()) 