import os
import asyncio
import websockets
import json
from dotenv import load_dotenv

async def test_create_and_cancel_invoice():
    load_dotenv()
    api_key = os.getenv('ANYPAY_API_KEY')
    
    if not api_key:
        print("Error: ANYPAY_API_KEY not found in .env file")
        return

    async with websockets.connect(
        'ws://localhost:8080',
        extra_headers={'Authorization': f'Bearer {api_key}'}
    ) as websocket:
        # First create an invoice
        create_message = {
            "action": "create_invoice",
                "amount": 1000,
                "currency": "USD",
                "memo": "Test invoice for cancellation"
        }

        print("Creating invoice...")
        await websocket.send(json.dumps(create_message))
        create_response = await websocket.recv()
        print(f"Create response: {create_response}")
        create_data = json.loads(create_response).get('data').get('invoice')
        print(f"Create data: {create_data.get('status')}")

        if create_data.get('status') != 'unpaid':
            print(f"❌ Failed to create invoice: {create_data.get('message')}")
            return

        invoice_uid = create_data['uid']
        print(f"✅ Invoice created with UID: {invoice_uid}")

        # Then cancel the invoice
        cancel_message = {
            "action": "cancel_invoice",
            "uid": invoice_uid
        }

        print(f"Cancelling invoice: {cancel_message}")

        print("Cancelling invoice...")
        await websocket.send(json.dumps(cancel_message))
        cancel_response = await websocket.recv()
        cancel_data = json.loads(cancel_response)

        if cancel_data.get('status') == 'success':
            print("✅ Invoice cancelled successfully")
        else:
            print(f"❌ Failed to cancel invoice: {cancel_data.get('message')}")

if __name__ == "__main__":
    asyncio.run(test_create_and_cancel_invoice())