use super::Card;
use anyhow::{Result, anyhow};
use async_trait::async_trait;
use bitcoin::Network;
use bitcoin::psbt::Psbt;
use nintondo_dogecoin::{
    bip32::{DerivationPath, ExtendedPrivKey}, key::Secp256k1, Address, Network as DogeNetwork, PrivateKey, PublicKey
};
use bip39::Mnemonic;


pub struct DogeCard {
    network: Network,
    account: u32,
    address: String,
    derivation_path: String,
    private_key: PrivateKey,
    public_key: PublicKey,
}

impl DogeCard {
    pub fn new(network: Network, account: u32, seed_phrase: &str) -> Result<Self> {
        let mnemonic = Mnemonic::parse(seed_phrase)
            .map_err(|e| anyhow!("Invalid seed phrase: {}", e))?;
        
        let seed = mnemonic.to_seed("");
        
        // Derive BIP44 path for DOGE: m/44'/3'/account'/0/0
        let path = format!("m/44'/3'/{}'/0/0", account);
        let derivation_path = path.parse::<DerivationPath>()
            .map_err(|_| anyhow!("Invalid derivation path"))?;

        // Convert bitcoin network to dogecoin network
        let doge_network = match network {
            Network::Bitcoin => DogeNetwork::Dogecoin,
            Network::Testnet => DogeNetwork::Testnet,
            Network::Signet => DogeNetwork::Signet,
            Network::Regtest => DogeNetwork::Regtest,
            _ => return Err(anyhow!("Unsupported network")),
        };
        
        let master_key = ExtendedPrivKey::new_master(doge_network, &seed)
            .map_err(|e| anyhow!("Failed to derive master key: {}", e))?;

        let secp = Secp256k1::new();
        let child_key = master_key.derive_priv(&secp, &derivation_path)
            .map_err(|e| anyhow!("Failed to derive child key: {}", e))?;
            
        let private_key = PrivateKey::new(child_key.private_key, DogeNetwork::Dogecoin);
        let public_key = PublicKey::from_private_key(&secp, &private_key);
        
        // Generate DOGE address
        let address = Address::p2pkh(&public_key, doge_network)
            .to_string();

        Ok(Self {
            network,
            account,
            address,
            derivation_path: path,
            private_key,
            public_key,
        })
    }
}

#[async_trait]
impl Card for DogeCard {
    fn chain(&self) -> &str {
        "DOGE"
    }

    fn currency(&self) -> &str {
        "DOGE"
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
        let api_key = std::env::var("ANYPAY_API_KEY")
            .map_err(|_| anyhow!("ANYPAY_API_KEY environment variable not set"))?;
            
        let client = crate::client::AnypayClient::new(&api_key);
        let utxos = client.get_utxos(&self.address).await?;
        
        let total_sats: u64 = utxos.iter()
            .map(|utxo| utxo.amount as u64)
            .sum();
            
        Ok(total_sats)
    }

    async fn get_decimal_balance(&self) -> Result<f64> {
        let sats = self.get_balance().await?;
        Ok(sats as f64 / 100_000_000.0)  // Convert satoshis to DOGE
    }

    async fn get_usd_balance(&self) -> Result<f64> {
        let doge = self.get_decimal_balance().await?;
        let api_key = std::env::var("ANYPAY_API_KEY")
            .map_err(|_| anyhow!("ANYPAY_API_KEY environment variable not set"))?;
        
        let client = crate::client::AnypayClient::new(&api_key);
        let doge_price = client.get_price("DOGE").await?;
        
        Ok(doge * doge_price)
    }

    fn sign_transaction(&self, psbt: &mut Psbt) -> Result<()> {
        // TODO: Implement DOGE transaction signing
        // This will require converting the Bitcoin PSBT to a DOGE transaction
        // and signing it with the DOGE private key
        Err(anyhow!("DOGE transaction signing not yet implemented"))
    }
} 