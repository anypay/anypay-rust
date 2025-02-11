use super::Card;
use anyhow::{Result, anyhow};
use async_trait::async_trait;
use bitcoin::Network;
use bitcoin::psbt::Psbt;
use ethers::{
    core::k256::ecdsa::SigningKey, providers::{Http, Middleware, Provider}, signers::{coins_bip39::English, LocalWallet, MnemonicBuilder, Signer, Wallet}, types::H160
};

pub struct EthereumCard {
    network: Network,
    account: u32,
    address: String,
    derivation_path: String,
    wallet: Wallet<SigningKey>,
    chain: String,
    currency: String,
}

impl EthereumCard {
    pub fn new(network: Network, account: u32, seed_phrase: &str, chain: &str, currency: &str) -> Result<Self> {


        // Derive BIP44 path
        // ETH: m/44'/60'/account'/0/0
        // MATIC: m/44'/966'/account'/0/0
        let coin_type = match chain {
            "ETH" => 60,
            "POLYGON" => 966,
            _ => return Err(anyhow!("Unsupported chain: {}", chain)),
        };
        
        let path = format!("m/44'/{}'/{:?}'/0/0", coin_type, account);
        
        // Create wallet from mnemonic using MnemonicBuilder
        let wallet = MnemonicBuilder::<English>::default()
            .phrase(seed_phrase)
            .derivation_path(&path)
            .unwrap()
            .build()
            .map_err(|e| anyhow!("Failed to create wallet: {}", e))?;
        
        let address = wallet.address().to_string();

        Ok(Self {
            network,
            account,
            address,
            derivation_path: path,
            wallet,
            chain: chain.to_string(),
            currency: currency.to_string(),
        })
    }
    
    fn get_rpc_url(&self) -> &str {
        match (self.chain.as_str(), self.network) {
            ("ETH", Network::Bitcoin) => "https://eth-mainnet.g.alchemy.com/v2/your-api-key",
            ("ETH", _) => "https://eth-sepolia.g.alchemy.com/v2/your-api-key",
            ("POLYGON", Network::Bitcoin) => "https://polygon-mainnet.g.alchemy.com/v2/your-api-key",
            ("POLYGON", _) => "https://polygon-mumbai.g.alchemy.com/v2/your-api-key",
            _ => "https://eth-mainnet.g.alchemy.com/v2/your-api-key", // default to ETH mainnet
        }
    }
}

#[async_trait]
impl Card for EthereumCard {
    fn chain(&self) -> &str {
        &self.chain
    }

    fn currency(&self) -> &str {
        &self.currency
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
        let provider = Provider::<Http>::try_from(self.get_rpc_url())
            .map_err(|e| anyhow!("Failed to create provider: {}", e))?;
            
        let address = self.address.parse::<H160>()
            .map_err(|e| anyhow!("Invalid address: {}", e))?;
            
        // Use the Middleware trait for get_balance
        let balance = provider.get_balance(address, None).await
            .map_err(|e| anyhow!("Failed to get balance: {}", e))?;
            
        Ok(balance.low_u64())  // Convert U256 to u64
    }

    async fn get_decimal_balance(&self) -> Result<f64> {
        let wei = self.get_balance().await?;
        Ok(wei as f64 / 1_000_000_000_000_000_000.0)  // Convert wei to ETH/MATIC (1 = 1e18 wei)
    }

    async fn get_usd_balance(&self) -> Result<f64> {
        let amount = self.get_decimal_balance().await?;
        let api_key = std::env::var("ANYPAY_API_KEY")
            .map_err(|_| anyhow!("ANYPAY_API_KEY environment variable not set"))?;
        
        let client = crate::client::AnypayClient::new(&api_key);
        let price = client.get_price(&self.currency).await?;
        
        Ok(amount * price)
    }

    fn sign_transaction(&self, _psbt: &mut Psbt) -> Result<()> {
        // ETH/MATIC don't use PSBT format
        Err(anyhow!("{} does not support PSBT transactions", self.chain))
    }
} 