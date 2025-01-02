#!/usr/bin/env python3

import asyncio
import websockets
import json
import aiohttp
import logging
from datetime import datetime
from typing import Dict, List, Optional
import os
from dotenv import load_dotenv

load_dotenv()

logging.basicConfig(
    level=logging.INFO,
    format='%(asctime)s - %(levelname)s - %(message)s'
)
logger = logging.getLogger(__name__)

async def get_btc_prices() -> float:
    """Get BTC price from multiple sources"""
    prices = {}
    
    async with aiohttp.ClientSession() as session:
        # CoinGecko
        try:
            url = "https://api.coingecko.com/api/v3/simple/price"
            params = {"ids": "bitcoin", "vs_currencies": "usd"}
            async with session.get(url, params=params) as response:
                if response.status == 200:
                    data = await response.json()
                    prices['coingecko'] = data["bitcoin"]["usd"]
        except Exception as e:
            logger.error(f"CoinGecko error: {e}")

        # Binance
        try:
            url = "https://api.binance.com/api/v3/ticker/price"
            params = {"symbol": "BTCUSDT"}
            async with session.get(url, params=params) as response:
                if response.status == 200:
                    data = await response.json()
                    prices['binance'] = float(data["price"])
        except Exception as e:
            logger.error(f"Binance error: {e}")

        # Kraken
        try:
            url = "https://api.kraken.com/0/public/Ticker"
            params = {"pair": "XBTUSD"}
            async with session.get(url, params=params) as response:
                if response.status == 200:
                    data = await response.json()
                    prices['kraken'] = float(data["result"]["XXBTZUSD"]["c"][0])
        except Exception as e:
            logger.error(f"Kraken error: {e}")

    if not prices:
        raise Exception("Could not fetch BTC price from any source")

    # Calculate median price
    median_price = sorted(prices.values())[len(prices) // 2]
    logger.info("\nBTC Prices:")
    for source, price in prices.items():
        logger.info(f"{source}: ${price:,.2f}")
    logger.info(f"Median price: ${median_price:,.2f}\n")
    
    return median_price

class PriceTest:
    def __init__(self, uri: str):
        self.uri = uri
        self.websocket: Optional[websockets.WebSocketClientProtocol] = None

    async def connect(self):
        self.websocket = await websockets.connect(self.uri)
        logger.info(f"Connected to {self.uri}")

    async def close(self):
        if self.websocket:
            await self.websocket.close()
            logger.info("Connection closed")

    async def convert_price(self, quote_currency: str, base_currency: str, quote_value: float) -> Dict:
        if not self.websocket:
            raise Exception("Not connected")

        request = {
            "action": "convert_price",
            "quote_currency": quote_currency,
            "base_currency": base_currency,
            "quote_value": quote_value
        }
        
        logger.info(f"Sending request: {json.dumps(request, indent=2)}")
        await self.websocket.send(json.dumps(request))
        
        response = await self.websocket.recv()
        return json.loads(response)

    async def test_conversion(self, quote_currency: str, base_currency: str, quote_value: float) -> bool:
        logger.info(f"Testing conversion of {quote_value} {quote_currency} to {base_currency}")
        
        # Get reference price
        reference_price = await get_btc_prices()
        
        try:
            result = await self.convert_price(quote_currency, base_currency, quote_value)
            
            if result["status"] != "success":
                logger.error(f"Conversion failed: {result.get('message', 'Unknown error')}")
                return False
                
            conversion = result["data"]
            converted_price = conversion["base_value"]
            price_difference = abs(converted_price - reference_price) / reference_price
            
            logger.info("\nConversion Results:")
            logger.info(f"Converted price: ${converted_price:,.2f}")
            logger.info(f"Reference price: ${reference_price:,.2f}")
            logger.info(f"Price difference: {price_difference * 100:.2f}%")
            
            if price_difference <= 0.05:
                logger.info("✅ Price is within 5% of market rate")
                return True
            else:
                logger.error("❌ Price difference exceeds 5% threshold")
                return False
                
        except Exception as e:
            logger.error(f"Error during conversion: {e}")
            return False

async def main():
    uri = os.getenv("WEBSOCKET_URI", "ws://localhost:8080")
    tester = PriceTest(uri)
    
    try:
        await tester.connect()
        success = await tester.test_conversion("BTC", "USD", 1)
        
        if not success:
            logger.error("Test failed!")
            exit(1)
        else:
            logger.info("Test completed successfully!")
            
    finally:
        await tester.close()

if __name__ == "__main__":
    asyncio.run(main()) 