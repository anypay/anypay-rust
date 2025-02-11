use super::Card;
use anyhow::{Result, anyhow};
use async_trait::async_trait;
use bitcoin::Network;
use bitcoin::psbt::Psbt;
use xrpl::core::keypairs::derive_keypair;
use bip39::Mnemonic;
use zerocopy::AsBytes;
use reqwest;
use serde_json;

pub struct RippleCard {
    network: Network,
    account: u32,
    address: String,
    derivation_path: String,
    private_key: String,
    public_key: String
}

impl RippleCard {
    pub fn new(network: Network, account: u32, seed_phrase: &str) -> Result<Self> {
        let mnemonic = Mnemonic::parse(seed_phrase)
            .map_err(|e| anyhow!("Invalid seed phrase: {}", e))?;
        
        let seed = mnemonic.to_seed("");

        // print seed
        println!("Seed: {:?}", seed);
        
        // Derive BIP44 path for XRP: m/44'/144'/account'/0/0
        let path = format!("m/44'/144'/{}'/0/0", account);
        
        // Use xrpl library to derive keypair
        /*let (private_key, public_key) = derive_keypair(seed.as_bytes(), false)
            .map_err(|e| anyhow!("Failed to derive XRP keypair: {}", e))?;
        
        // Convert keys to strings
        let private_key = String::from_utf8_lossy(&private_key).to_string();
        let public_key = String::from_utf8_lossy(&public_key).to_string();
        
        // Generate XRP address from public key
        let address = xrpl::core::addresscodec::encode_account_public_key(public_key.as_bytes())
            .map_err(|e| anyhow!("Failed to create XRP address: {}", e))?;
        */

        Ok(Self {
            network,
            account,
            address: "".to_string(),
            derivation_path: path,
            private_key: "".to_string(),
            public_key: "".to_string(),
        })
    }
}

#[async_trait]
impl Card for RippleCard {
    fn chain(&self) -> &str {
        "XRPL"
    }

    fn currency(&self) -> &str {
        "XRP"
    }

    fn network(&self) -> Network {
        self.network
    }

    fn derivation_path(&self) -> &str {
        &self.derivation_path
    }

    fn address(&self) -> &str {
        &self.address
    }

    fn account(&self) -> u32 {
        self.account
    }

    async fn get_balance(&self) -> Result<u64> {
        let client = reqwest::Client::new();
        let response = client
            .post("https://s1.ripple.com:51234")
            .json(&serde_json::json!({
                "method": "account_info",
                "params": [{
                    "account": self.address,
                    "strict": true,
                    "ledger_index": "current",
                    "queue": true
                }]
            }))
            .send()
            .await?
            .json::<serde_json::Value>()
            .await?;

        let balance = response["result"]["account_data"]["Balance"]
            .as_str()
            .ok_or_else(|| anyhow!("Balance not found"))?
            .parse::<f64>()
            .map_err(|e| anyhow!("Failed to parse balance: {}", e))?;
            
        Ok((balance * 1_000_000.0) as u64)
    }

    async fn get_decimal_balance(&self) -> Result<f64> {
        let drops = self.get_balance().await?;
        Ok(drops as f64 / 1_000_000.0)  // Convert drops to XRP
    }

    async fn get_usd_balance(&self) -> Result<f64> {
        let xrp = self.get_decimal_balance().await?;
        let api_key = std::env::var("ANYPAY_API_KEY")
            .map_err(|_| anyhow!("ANYPAY_API_KEY environment variable not set"))?;
        
        let client = crate::client::AnypayClient::new(&api_key);
        let xrp_price = client.get_price("XRP").await?;
        
        Ok(xrp * xrp_price)
    }

    fn sign_transaction(&self, _psbt: &mut Psbt) -> Result<()> {
        // XRP doesn't use PSBT, this is just a placeholder to satisfy the trait
        Err(anyhow!("XRP does not support PSBT transactions"))
    }
} 