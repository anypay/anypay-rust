#!/usr/bin/env python3

import os
import pika
import json
import logging
from dotenv import load_dotenv
from urllib.parse import urlparse

logging.basicConfig(
    level=logging.INFO,
    format='%(asctime)s - %(levelname)s - %(message)s'
)

def test_amqp_messaging():
    # Load environment variables
    load_dotenv()
    amqp_url = os.getenv('AMQP_URL')
    if not amqp_url:
        raise ValueError("AMQP_URL not found in environment")

    logging.info("Connecting to AMQP server...")
    
    # Parse AMQP URL
    params = pika.URLParameters(amqp_url)
    connection = pika.BlockingConnection(params)
    channel = connection.channel()

    # Declare exchange only if it doesn't exist
    channel.exchange_declare(
        exchange='events',
        exchange_type='topic',
        durable=True,
        passive=True  # Only check if exists, don't create
    )
    logging.info("Exchange 'events' exists")

    # Test messages
    test_events = [
        {
            "type": "invoice.created",
            "data": {
                "id": "inv_123",
                "amount": 1000,
                "currency": "USD"
            }
        },
        {
            "type": "payment.received",
            "data": {
                "invoice_id": "inv_123",
                "amount": 1000,
                "currency": "USD",
                "txid": "abc123"
            }
        }
    ]

    # Send test messages
    for event in test_events:
        routing_key = event["type"]
        message = json.dumps(event)
        
        channel.basic_publish(
            exchange='events',
            routing_key=routing_key,
            body=message
        )
        logging.info(f"✅ Sent message: {message}")

    connection.close()
    logging.info("Connection closed")

if __name__ == "__main__":
    try:
        test_amqp_messaging()
    except pika.exceptions.ChannelClosedByBroker as e:
        if e.reply_code == 404:  # Exchange doesn't exist
            logging.error("❌ Exchange 'events' doesn't exist. Start the daemon first.")
        else:
            logging.error(f"❌ Channel error: {e}")
        raise
    except Exception as e:
        logging.error(f"❌ Error: {e}")
        raise 