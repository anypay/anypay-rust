#!/usr/bin/env python3

import asyncio
import websockets
import json
import logging
from dotenv import load_dotenv
import os

logging.basicConfig(
    level=logging.INFO,
    format='%(asctime)s - %(levelname)s - %(message)s'
)

def verify_payment_option(payment_option, invoice_amount):
    """Verify a payment option has reasonable values"""
    logging.info(f"\nVerifying payment option for {payment_option['currency']} on {payment_option['chain']}:")
    logging.info(f"Total amount: {payment_option['amount']}")
    logging.info(f"Fee: {payment_option['fee']}")
    
    assert payment_option["amount"] > 0, f"Payment option amount should be > 0, got {payment_option['amount']}"
    
    # Verify outputs
    total_output_amount = sum(output["amount"] for output in payment_option["outputs"])
    logging.info(f"Total from outputs: {total_output_amount}")
    for i, output in enumerate(payment_option["outputs"]):
        logging.info(f"Output {i+1}: {output['amount']} to {output['address']}")
    
    assert total_output_amount == payment_option["amount"], \
        f"Sum of outputs ({total_output_amount}) should equal payment option amount ({payment_option['amount']})"
    
    # Verify each output has reasonable values
    for output in payment_option["outputs"]:
        assert output["amount"] > 0, f"Output amount should be > 0, got {output['amount']}"
        assert output["address"], "Output should have an address"
    
    # Verify single output for all chains
    assert len(payment_option["outputs"]) == 1, f"Should have exactly 1 output, got {len(payment_option['outputs'])}"
    assert payment_option["outputs"][0]["amount"] == payment_option["amount"], \
        f"Output amount ({payment_option['outputs'][0]['amount']}) should equal total amount ({payment_option['amount']})"

async def test_create_invoice():
    load_dotenv()
    api_key = os.getenv('ANYPAY_API_KEY')
    
    if not api_key:
        print("Error: ANYPAY_API_KEY not found in .env file")
        return

    uri = "ws://localhost:8080"
    logging.info("Connecting to WebSocket server...")
    
    async with websockets.connect(
        'ws://localhost:8080',
        extra_headers={'Authorization': f'Bearer {api_key}'}
    ) as websocket:  
        # Create a new invoice
        create_msg = {
            "action": "create_invoice",
            "amount": 100000,  # $1,000.00 USD
            "currency": "USD",
            "webhook_url": "https://example.com/webhook",
            "redirect_url": "https://example.com/return",
            "memo": "Test invoice creation"
        }
        
        logging.info(f"Sending create invoice request: {create_msg}")
        await websocket.send(json.dumps(create_msg))
        
        response = await websocket.recv()
        logging.info(f"Received response: {response}")
        
        response_data = json.loads(response)
        
        # Validate response
        assert response_data["status"] == "success", f"Failed to create invoice: {response_data}"
        invoice = response_data["data"]["invoice"]
        assert invoice["amount"] == 100000, f"Incorrect amount: {invoice['amount']}"
        assert invoice["currency"] == "USD", f"Incorrect currency: {invoice['currency']}"
        assert invoice["status"] == "unpaid", f"Incorrect status: {invoice['status']}"
        assert "uid" in invoice, "Invoice missing uid"
        assert "createdAt" in invoice, "Invoice missing createdAt"
        assert "updatedAt" in invoice, "Invoice missing updatedAt"
        assert invoice["webhook_url"] == "https://example.com/webhook", "Incorrect webhook_url"
        assert invoice["redirect_url"] == "https://example.com/return", "Incorrect redirect_url"
        assert invoice["memo"] == "Test invoice creation", "Incorrect memo"
        assert "uri" in invoice, "Invoice missing uri"
        assert invoice["uri"].startswith("pay:?r=https://api.anypayx.com/r/"), "Invalid URI format"
        
        # Validate payment options
        payment_options = response_data["data"]["payment_options"]
        assert isinstance(payment_options, list), "payment_options should be a list"
        assert len(payment_options) > 0, "Should have at least one payment option"
        
        logging.info(f"\nPayment Options:")
        for option in payment_options:
            verify_payment_option(option, invoice["amount"])
        
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
        fetched_invoice = fetch_data["data"]["invoice"]
        assert fetched_invoice["uid"] == invoice["uid"], "Invoice IDs don't match"
        assert fetched_invoice["amount"] == invoice["amount"], "Invoice amounts don't match"
        assert fetched_invoice["currency"] == invoice["currency"], "Invoice currencies don't match"
        assert fetched_invoice["status"] == invoice["status"], "Invoice statuses don't match"

async def main():
    try:
        await test_create_invoice()
        logging.info("✅ Invoice creation test passed!")
    except Exception as e:
        logging.error(f"❌ Test failed: {e}")
        raise

if __name__ == "__main__":
    asyncio.run(main()) 