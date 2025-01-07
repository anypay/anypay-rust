import os
import asyncio
import websockets
import json
from datetime import datetime

async def test_ping():
    try:
        # Connect to WebSocket server
        async with websockets.connect('ws://localhost:8080') as websocket:
            # Create ping message
            ping_message = {
                "action": "ping"
            }

            print("Sending ping...")
            await websocket.send(json.dumps(ping_message))
            
            # Wait for pong response
            response = await websocket.recv()
            response_data = json.loads(response)

            print(f"Received response: {response_data}")
            
            # Validate response
            if (response_data.get('type') == 'pong' and 
                response_data.get('status') == 'success' and 
                'timestamp' in response_data):
                
                # Convert timestamp to readable format
                timestamp = datetime.fromtimestamp(response_data['timestamp'])
                print(f"✅ Received pong at {timestamp}")
                print(f"Round trip latency: {datetime.utcnow().timestamp() - response_data['timestamp']}s")
            else:
                print("❌ Invalid pong response:", response_data)

    except Exception as e:
        print(f"❌ Error: {e}")

if __name__ == "__main__":
    asyncio.run(test_ping()) 