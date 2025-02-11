use super::Card;
use anyhow::{Result, anyhow};
use async_trait::async_trait;
use bitcoin::Network;
use bitcoin::psbt::Psbt;
use bip39::Mnemonic;
use ed25519_dalek::{Keypair, SecretKey, PublicKey};
use solana_sdk::{
    commitment_config::CommitmentConfig, pubkey::Pubkey, signature::Keypair as SolanaKeypair, signer::Signer
};
use solana_client::rpc_client::RpcClient;
use std::str::FromStr;

pub struct SolanaCard {
    network: Network,
    account: u32,
    address: String,
    derivation_path: String,
    keypair: SolanaKeypair,
}

impl SolanaCard {
    pub fn new(network: Network, account: u32, seed_phrase: &str) -> Result<Self> {
        let mnemonic = Mnemonic::parse(seed_phrase)
            .map_err(|e| anyhow!("Invalid seed phrase: {}", e))?;
        
        let seed = mnemonic.to_seed("");
        
        // Derive BIP44 path for SOL: m/44'/501'/account'/0'
        let path = format!("m/44'/501'/{}'/0'", account);
        
        // Use ed25519 derivation
        let derived_bytes = {
            use hmac::{Hmac, Mac};
            use sha2::Sha512;
            
            let mut mac = Hmac::<Sha512>::new_from_slice(b"ed25519 seed")
                .map_err(|_| anyhow!("Failed to create HMAC"))?;
            mac.update(&seed);
            let result = mac.finalize();
            result.into_bytes().to_vec()
        };
        
        // Create Solana keypair from derived bytes
        let secret = SecretKey::from_bytes(&derived_bytes[..32])
            .map_err(|e| anyhow!("Failed to create secret key: {}", e))?;
        let public = PublicKey::from(&secret);
        let ed_keypair = Keypair { secret, public };
        
        // Convert to Solana keypair
        let keypair = SolanaKeypair::from_bytes(&ed_keypair.to_bytes())
            .map_err(|e| anyhow!("Failed to create Solana keypair: {}", e))?;
            
        let address = keypair.pubkey().to_string();

        Ok(Self {
            network,
            account,
            address,
            derivation_path: path,
            keypair,
        })
    }
    
    fn get_rpc_url(&self) -> &str {
        match self.network {
            Network::Bitcoin => "https://api.mainnet-beta.solana.com",
            _ => "https://api.testnet.solana.com",
        }
    }
}

#[async_trait]
impl Card for SolanaCard {
    fn chain(&self) -> &str {
        "SOL"
    }

    fn currency(&self) -> &str {
        "SOL"
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
        let rpc_client = RpcClient::new(self.get_rpc_url());
        let pubkey = Pubkey::from_str(&self.address)
            .map_err(|e| anyhow!("Invalid Solana address: {}", e))?;
            
        let balance = rpc_client
            .get_balance_with_commitment(&pubkey, CommitmentConfig::confirmed())
            .map_err(|e| anyhow!("Failed to get balance: {}", e))?
            .value;
            
        Ok(balance)
    }

    async fn get_decimal_balance(&self) -> Result<f64> {
        let lamports = self.get_balance().await?;
        Ok(lamports as f64 / 1_000_000_000.0)  // Convert lamports to SOL (1 SOL = 1e9 lamports)
    }

    async fn get_usd_balance(&self) -> Result<f64> {
        let sol = self.get_decimal_balance().await?;
        let api_key = std::env::var("ANYPAY_API_KEY")
            .map_err(|_| anyhow!("ANYPAY_API_KEY environment variable not set"))?;
        
        let client = crate::client::AnypayClient::new(&api_key);
        let sol_price = client.get_price("SOL").await?;
        
        Ok(sol * sol_price)
    }

    fn sign_transaction(&self, _psbt: &mut Psbt) -> Result<()> {
        // Solana doesn't use PSBT format
        Err(anyhow!("Solana does not support PSBT transactions"))
    }
} 