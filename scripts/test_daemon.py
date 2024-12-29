#!/usr/bin/env python3

import asyncio
import websockets
import json
import logging
from datetime import datetime

logging.basicConfig(
    level=logging.INFO,
    format='%(asctime)s - %(levelname)s - %(message)s'
)

async def subscribe_and_listen(uri, subscription_type, id):
    """Connect to WebSocket server and subscribe to events."""
    async with websockets.connect(uri) as websocket:
        # Subscribe to the event
        subscribe_msg = {
            "action": "subscribe",
            "type": subscription_type,
            "id": id
        }
        
        logging.info(f"Subscribing to {subscription_type} {id}")
        await websocket.send(json.dumps(subscribe_msg))
        
        response = await websocket.recv()
        logging.info(f"Subscription response: {response}")
        
        # Keep listening for events
        try:
            while True:
                message = await websocket.recv()
                logging.info(f"Received event: {message}")
        except websockets.exceptions.ConnectionClosed:
            logging.info("Connection closed")

async def test_multiple_subscriptions():
    """Test multiple concurrent subscriptions."""
    uri = "ws://localhost:8080"
    tasks = []
    
    # Test cases
    subscriptions = [
        ("invoice", "inv_123"),
        ("account", "acc_456"),
        ("address", "addr_789"),
        ("invoice", "inv_999")
    ]
    
    # Create tasks for each subscription
    for sub_type, sub_id in subscriptions:
        task = asyncio.create_task(
            subscribe_and_listen(uri, sub_type, sub_id)
        )
        tasks.append(task)
    
    # Wait for all tasks
    try:
        await asyncio.gather(*tasks)
    except KeyboardInterrupt:
        logging.info("Test terminated by user")
    except Exception as e:
        logging.error(f"Error during test: {e}")

def main():
    logging.info("Starting WebSocket daemon test")
    
    try:
        asyncio.run(test_multiple_subscriptions())
    except KeyboardInterrupt:
        logging.info("Test terminated by user")
    except Exception as e:
        logging.error(f"Test failed: {e}")

if __name__ == "__main__":
    main() 