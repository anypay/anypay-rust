#!/usr/bin/env python3

import os
import asyncio
import logging
from dotenv import load_dotenv
from realtime import AsyncRealtimeClient, AsyncRealtimeChannel

# Configure logging
logging.basicConfig(
    level=logging.INFO,
    format='%(asctime)s - %(levelname)s - %(message)s'
)
logger = logging.getLogger(__name__)

class InvoiceMonitor:
    def __init__(self):
        load_dotenv()
        
        # Get Supabase credentials
        self.supabase_url = os.getenv("SUPABASE_URL")
        self.anon_key = os.getenv("SUPABASE_ANON_KEY")
        
        if not self.supabase_url or not self.anon_key:
            raise ValueError("Missing Supabase credentials in .env file")
        
        # Initialize Realtime client
        self.client = AsyncRealtimeClient(
            f"{self.supabase_url}/realtime/v1",
            self.anon_key,
            auto_reconnect=True
        )
        self.channel = None

    def handle_insert(self, payload, *args):
        """Handle new invoice insertions"""
        new_invoice = payload['record']
        logger.info("🆕 New Invoice Created:")
        logger.info(f"  ID: {new_invoice.get('uid')}")
        logger.info(f"  Amount: {new_invoice.get('amount')} {new_invoice.get('currency')}")
        logger.info(f"  Status: {new_invoice.get('status')}")
        logger.info(f"  Created: {new_invoice.get('created_at')}")
        logger.info("  ---")
    
    def handle_update(self, payload, *args):
        """Handle invoice updates"""
        old = payload.get('old_record', {})
        new = payload['record']
        
        logger.info("📝 Invoice Updated:")
        logger.info(f"  ID: {new.get('uid')}")
        
        # Compare old and new values
        for key in ['amount', 'currency', 'status']:
            old_val = old.get(key)
            new_val = new.get(key)
            if old_val != new_val:
                logger.info(f"  {key.title()}: {old_val} → {new_val}")
        
        logger.info(f"  Updated: {new.get('updated_at')}")
        logger.info("  ---")

    def handle_all_changes(self, payload, *args):
        """Handle all changes (debug)"""
        logger.debug(f"Received change: {payload}")
    
    async def monitor(self):
        """Start monitoring invoice changes"""
        logger.info("🔄 Starting invoice monitor...")
        logger.info("Press Ctrl+C to stop")
        
        try:
            # Connect to Realtime
            await self.client.connect()
            
            # Create channel for database changes
            self.channel: AsyncRealtimeChannel = self.client.channel("invoice-changes")

            # Subscribe to changes
            await self.channel \
                .on_postgres_changes(
                    event='INSERT',
                    schema='public',
                    table='invoices',
                    callback=self.handle_insert
                ) \
                .on_postgres_changes(
                    event='UPDATE',
                    schema='public',
                    table='invoices',
                    callback=self.handle_update
                ) \
                .on_postgres_changes(
                    event='*',
                    schema='public',
                    table='invoices',
                    callback=self.handle_all_changes
                ) \
                .subscribe()

            # Start listening for changes
            await self.client.listen()
                
        except KeyboardInterrupt:
            logger.info("\n👋 Stopping invoice monitor...")
            if self.channel:
                await self.channel.unsubscribe()
            await self.client.disconnect()
        except Exception as e:
            logger.error(f"❌ Error: {e}")
            raise

async def main():
    monitor = InvoiceMonitor()
    await monitor.monitor()

if __name__ == "__main__":
    asyncio.run(main()) 