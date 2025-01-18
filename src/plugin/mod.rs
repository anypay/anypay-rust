use anyhow::Result;
use async_trait::async_trait;
use bigdecimal::BigDecimal;
use chrono::Utc;
use serde::{Serialize, Deserialize};

mod btc;
mod bsv;
mod eth;
mod xrp;
mod sol;
mod rlusd_eth;

pub use btc::BitcoinPlugin;
pub use bsv::BitcoinSVPlugin;
pub use eth::EthereumPlugin;
pub use xrp::RipplePlugin;
pub use sol::SolanaPlugin;
pub use rlusd_eth::RLUSDEthereumPlugin;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Account {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Address {
    pub id: String,
    pub value: String,
    pub chain: String,
    pub currency: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaymentOption {
    pub chain: String,
    pub currency: String,
    pub address: String,
    pub amount: i64,
    pub uri: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transaction {
    pub txhex: String,
    pub txid: Option<String>,
    pub txkey: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Payment {
    pub chain: String,
    pub currency: String,
    pub address: String,
    pub amount: i64,
    pub txid: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Confirmation {
    pub confirmations: i32,
    pub confirmed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Price {
    pub currency: String,
    pub price: BigDecimal,
    pub timestamp: i64,
}

#[async_trait]
pub trait Plugin: Send + Sync {
    fn currency(&self) -> &str;
    fn chain(&self) -> &str;
    fn decimals(&self) -> u8;

    async fn build_signed_payment(&self, payment_option: &PaymentOption, mnemonic: &str) -> Result<Transaction>;
    async fn verify_payment(&self, payment_option: &PaymentOption, transaction: &Transaction) -> Result<bool>;
    async fn validate_address(&self, address: &str) -> Result<bool>;
    async fn get_transaction(&self, txid: &str) -> Result<Transaction>;
    async fn broadcast_tx(&self, txhex: &str, txid: Option<&str>, txkey: Option<&str>) -> Result<Transaction>;
    async fn get_new_address(&self, account: &Account, address: &Address) -> Result<String>;
    async fn transform_address(&self, address: &str) -> Result<String>;
    async fn get_confirmation(&self, txid: &str) -> Result<Option<Confirmation>>;
    async fn get_payments(&self, txid: &str) -> Result<Vec<Payment>>;
    async fn parse_payments(&self, transaction: &Transaction) -> Result<Vec<Payment>>;
    async fn get_price(&self) -> Result<Price>;

    fn satoshis_to_decimal(&self, satoshis: i64) -> BigDecimal {
        let decimals = self.decimals() as u32;
        let divisor = BigDecimal::from(10i64.pow(decimals));
        BigDecimal::from(satoshis) / divisor
    }

    fn decimal_to_satoshis(&self, decimal: &BigDecimal) -> i64 {
        let decimals = self.decimals() as u32;
        let multiplier = BigDecimal::from(10i64.pow(decimals));
        (decimal * multiplier).to_string().parse().unwrap_or(0)
    }
}

pub fn get_plugin(chain: &str, currency: &str) -> Option<Box<dyn Plugin>> {
    match (chain, currency) {
        ("BTC", "BTC") => Some(Box::new(BitcoinPlugin)),
        ("BSV", "BSV") => Some(Box::new(BitcoinSVPlugin)),
        ("ETH", "ETH") => Some(Box::new(EthereumPlugin)),
        ("ETH", "RLUSD") => Some(Box::new(RLUSDEthereumPlugin)),
        ("XRP", "XRP") => Some(Box::new(RipplePlugin)),
        ("SOL", "SOL") => Some(Box::new(SolanaPlugin)),
        _ => None,
    }
} 