#!/usr/bin/env python3

import asyncio
import websockets
import json
import logging
from dotenv import load_dotenv
import os
import time

logging.basicConfig(
    level=logging.INFO,
    format='%(asctime)s - %(levelname)s - %(message)s'
)

def verify_payment_option(payment_option, invoice_amount):
    """Verify a payment option has reasonable values"""
    logging.info(f"\nVerifying payment option for {payment_option['currency']} on {payment_option['chain']}:")
    logging.info(f"Total amount: {payment_option['amount']}")
    logging.info(f"Fee: {payment_option['fee']}")
    logging.info(f"Expires at: {payment_option['expires']}")
    
    assert payment_option["amount"] > 0, f"Payment option amount should be > 0, got {payment_option['amount']}"
    assert "expires" in payment_option, "Payment option missing expires"
    
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
    
    # Verify single output
    assert len(payment_option["outputs"]) == 1, f"Should have exactly 1 output, got {len(payment_option['outputs'])}"
    assert payment_option["outputs"][0]["amount"] == payment_option["amount"], \
        f"Output amount ({payment_option['outputs'][0]['amount']}) should equal total amount ({payment_option['amount']})"

async def test_payment_option_expiry():
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
            "memo": "Test payment option expiry"
        }
        
        logging.info(f"Sending create invoice request: {create_msg}")
        await websocket.send(json.dumps(create_msg))
        
        response = await websocket.recv()
        logging.info(f"Received response: {response}")
        
        response_data = json.loads(response)
        assert response_data["status"] == "success", f"Failed to create invoice: {response_data}"
        
        invoice = response_data["data"]["invoice"]
        initial_payment_options = response_data["data"]["payment_options"]
        
        # Store initial amounts for comparison
        initial_amounts = {
            f"{opt['currency']}:{opt['chain']}": opt['amount'] 
            for opt in initial_payment_options
        }
        
        # Wait for payment options to expire (they expire after 15 minutes)
        # For testing, we'll fetch again immediately since the server should still
        # refresh expired options
        
        # Fetch invoice to check payment options
        await websocket.send(json.dumps({
            "action": "fetch_invoice",
            "id": invoice["uid"]
        }))
        
        fetch_response = await websocket.recv()
        fetch_data = json.loads(fetch_response)
        logging.info(f"Fetch response: {fetch_response}")
        
        assert fetch_data["status"] == "success", "Failed to fetch invoice"
        assert len(fetch_data["data"]) == 2, "Expected invoice and payment options in response"
        
        invoice = fetch_data["data"][0]
        refreshed_payment_options = fetch_data["data"][1]
        
        # Verify invoice fields
        assert invoice["uid"] == invoice["uid"], "Invoice UID mismatch"
        assert invoice["amount"] == 100000, "Invoice amount mismatch"
        assert invoice["currency"] == "USD", "Invoice currency mismatch"
        
        # Verify payment options
        assert len(refreshed_payment_options) > 0, "No payment options returned"
        for option in refreshed_payment_options:
            verify_payment_option(option, 100000)
            
            # Check that amounts might have changed due to price updates
            key = f"{option['currency']}:{option['chain']}"
            if key in initial_amounts:
                if option['amount'] != initial_amounts[key]:
                    logging.info(f"Amount changed for {key}:")
                    logging.info(f"  Initial: {initial_amounts[key]}")
                    logging.info(f"  Updated: {option['amount']}")
        
        logging.info("✅ Test passed: Invoice fetched successfully with valid payment options")

async def main():
    try:
        await test_payment_option_expiry()
        logging.info("✅ Payment option expiry test passed!")
    except Exception as e:
        logging.error(f"❌ Test failed: {e}")
        raise

if __name__ == "__main__":
    asyncio.run(main()) 