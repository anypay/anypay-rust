use bitcoin::Network;
use anyhow::Result;
use async_trait::async_trait;
use bitcoin::psbt::Psbt;

//pub mod btc;
pub mod xrp;
pub mod sol;
pub mod eth;
pub mod doge;
#[async_trait]
pub trait Card {
    /// Get the chain identifier (e.g., "BTC", "XRPL")
    fn chain(&self) -> &str;
    
    /// Get the currency identifier (e.g., "BTC", "XRP")
    fn currency(&self) -> &str;
    
    /// Get the network (mainnet/testnet)
    fn network(&self) -> Network;
    
    /// Get the derivation path used to generate this card
    fn derivation_path(&self) -> &str;
    
    /// Get the address for this card
    fn address(&self) -> &str;
    
    /// Get the account index used to generate this card
    fn account(&self) -> u32;
    
    /// Get the balance in the smallest unit (satoshis for BTC, drops for XRP)
    async fn get_balance(&self) -> Result<u64>;
    
    /// Get the balance in the standard unit (BTC for Bitcoin, XRP for Ripple)
    async fn get_decimal_balance(&self) -> Result<f64>;
    
    /// Get the balance in USD
    async fn get_usd_balance(&self) -> Result<f64>;
    
    /// Sign a transaction (implementation depends on chain)
    fn sign_transaction(&self, tx: &mut Psbt) -> Result<()>;
}

#[derive(Debug)]
pub struct Balance {
    pub smallest_unit: u64,  // satoshis, drops, etc.
    pub decimal: f64,        // BTC, XRP, etc.
    pub usd: f64,
}

// Factory function to create the appropriate card type
pub fn create_card(
    chain: &str,
    currency: &str,
    network: Network,
    account: u32,
    seed_phrase: &str,
) -> Result<Box<dyn Card>> {
    match (chain, currency) {
        //("BTC", "BTC") => Ok(Box::new(btc::BitcoinCard::new(network, account, seed_phrase)?)),
        ("XRPL", "XRP") => Ok(Box::new(xrp::RippleCard::new(network, account, seed_phrase)?)),
        ("SOL", "SOL") => Ok(Box::new(sol::SolanaCard::new(network, account, seed_phrase)?)),
        ("ETH", "ETH") => Ok(Box::new(eth::EthereumCard::new(network, account, seed_phrase, "ETH", "ETH")?)),
        ("POLYGON", "MATIC") => Ok(Box::new(eth::EthereumCard::new(network, account, seed_phrase, "POLYGON", "MATIC")?)),
        ("DOGE", "DOGE") => Ok(Box::new(doge::DogeCard::new(network, account, seed_phrase)?)),
        _ => Err(anyhow::anyhow!("Unsupported chain/currency combination: {}/{}", chain, currency))
    }
} 