use super::Card;
use anyhow::{Result, anyhow};
use async_trait::async_trait;
use bitcoin::{
    Network, Address, PublicKey,
    secp256k1::{Secp256k1, SecretKey},
    psbt::Psbt,
};
use bip32::{DerivationPath, XPrv};
use std::str::FromStr;
use bip39::Mnemonic;
use serde::{Deserialize, Serialize};

// Custom UTXO struct for Fractal Bitcoin API response format
#[derive(Debug, Deserialize, Clone)]
struct FractalUtxo {
    pub txid: String,
    pub vout: u32,
    pub value: u64,  // Fractal API uses 'value' instead of 'amount'
    pub status: FractalUtxoStatus,
}

#[derive(Debug, Deserialize, Clone)]
struct FractalUtxoStatus {
    pub confirmed: bool,
    pub block_height: Option<u32>,
    pub block_time: Option<u64>,
}

pub struct FractalBitcoinCard {
    network: Network,
    account: u32,
    address: String,
    derivation_path: String,
    private_key: SecretKey,
}

impl FractalBitcoinCard {
    pub fn new(network: Network, account: u32, seed_phrase: &str) -> Result<Self> {
        let mnemonic = Mnemonic::parse(seed_phrase)
            .map_err(|e| anyhow!("Invalid seed phrase: {}", e))?;
        
        let seed = mnemonic.to_seed("");
        let secp = Secp256k1::new();

        // Derive BIP44 path: m/44'/0'/account'/0/0 for FB
        let path = format!("m/44'/0'/{}'/0/0", account);
        let derivation_path = DerivationPath::from_str(&path)
            .map_err(|e| anyhow!("Invalid derivation path: {}", e))?;

        // Use the separate bip32 crate to derive keys
        let xpriv = bip32::XPrv::derive_from_path(&seed, &derivation_path)
            .map_err(|e| anyhow!("Failed to derive private key: {}", e))?;
        
        // Convert to bitcoin SecretKey
        let private_key = SecretKey::from_slice(&xpriv.private_key().to_bytes())
            .map_err(|e| anyhow!("Failed to create secret key: {}", e))?;
        
        // Get a secp256k1 public key first, then convert to bitcoin public key
        let secp256k1_pubkey = secp256k1::PublicKey::from_secret_key(&secp, &private_key);
        let public_key = PublicKey::new(secp256k1_pubkey);
        
        let address = Address::p2wpkh(&public_key, network)
            .map_err(|e| anyhow!("Failed to create address: {}", e))?;

        Ok(Self {
            network,
            account,
            address: address.to_string(),
            derivation_path: path,
            private_key,
        })
    }
}

#[async_trait]
impl Card for FractalBitcoinCard {
    fn chain(&self) -> &str {
        "FB"
    }

    fn currency(&self) -> &str {
        "FB"
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
        // log the url
        tracing::info!("Fetching UTXOs from Fractal API: {}", &format!("https://mempool.fractalbitcoin.io/api/v1/address/{}/utxo", self.address));

        // print the url to the console
        println!("Fetching UTXOs from Fractal API: {}", &format!("https://mempool.fractalbitcoin.io/api/v1/address/{}/utxo", self.address));
        
        // Use Fractal-specific API for getting UTXOs
        let fractal_utxos = match reqwest::Client::new()
            .get(&format!("https://mempool.fractalbitcoin.io/api/v1/address/{}/utxo", self.address))
            .send()
            .await {
                Ok(response) => {
                    if response.status().is_success() {
                        response.json::<Vec<FractalUtxo>>().await
                            .map_err(|e| anyhow!("Failed to parse UTXOs: {}", e))?
                    } else {
                        let error = response.text().await?;
                        return Err(anyhow!("Failed to fetch UTXOs from Fractal API: {}", error));
                    }
                },
                Err(e) => return Err(anyhow!("Failed to connect to Fractal API: {}", e))
            };
        
        // Sum the values directly (they're already in satoshis)
        let total_sats: u64 = fractal_utxos.iter()
            .map(|utxo| utxo.value)
            .sum();

        Ok(total_sats)
    }

    async fn get_decimal_balance(&self) -> Result<f64> {
        let sats = self.get_balance().await?;
        // Convert satoshis to FB (same as BTC, 1 FB = 100,000,000 satoshis)
        Ok(sats as f64 / 100_000_000.0)
    }

    async fn get_usd_balance(&self) -> Result<f64> {
        let fb = self.get_decimal_balance().await?;
        let api_key = std::env::var("ANYPAY_API_KEY")
            .map_err(|_| anyhow!("ANYPAY_API_KEY environment variable not set"))?;
        
        // Get FB price instead of BTC price
        let response = reqwest::Client::new()
            .get("https://api.anypayx.com/convert/1-FB/to-USD")
            .send()
            .await?;

        if !response.status().is_success() {
            let error = response.text().await?;
            return Err(anyhow!("Failed to fetch FB price: {}", error));
        }

        let data = response.json::<serde_json::Value>().await?;
        let fb_price = data
            .get("conversion")
            .and_then(|c| c.get("output"))
            .and_then(|o| o.get("value"))
            .and_then(|v| v.as_f64())
            .ok_or_else(|| anyhow!("Failed to extract FB price from response"))?;
        
        Ok(fb * fb_price)
    }

    fn sign_transaction(&self, psbt: &mut Psbt) -> Result<()> {
        use bitcoin::sighash::{SighashCache, EcdsaSighashType};
        use bitcoin::secp256k1::Message;

        let secp = Secp256k1::new();
        let mut sighash_cache = SighashCache::new(&psbt.unsigned_tx);
        
        // Sign each input
        for (i, input) in psbt.inputs.iter_mut().enumerate() {
            if let Some(witness_utxo) = &input.witness_utxo {
                // Same pattern as in new() method
                let secp256k1_pubkey = secp256k1::PublicKey::from_secret_key(&secp, &self.private_key);
                let public_key = PublicKey::new(secp256k1_pubkey);
                
                // Calculate sighash - use p2wpkh instead of segwit hash
                let sighash = sighash_cache
                    .p2wpkh_signature_hash(i, &witness_utxo.script_pubkey, witness_utxo.value, EcdsaSighashType::All)
                    .map_err(|e| anyhow!("Failed to calculate sighash: {}", e))?;

                // Sign the sighash - use from_digest_slice instead of from_slice
                let msg = Message::from_digest_slice(&sighash[..]).unwrap();
                let sig = secp.sign_ecdsa(&msg, &self.private_key);
                let mut sig_bytes = sig.serialize_der().to_vec();
                sig_bytes.push(EcdsaSighashType::All as u8);

                // Add the signature to the PSBT - use a more direct approach
                input.partial_sigs.insert(
                    public_key,
                    bitcoin::ecdsa::Signature::from_slice(&sig_bytes)
                        .map_err(|e| anyhow!("Failed to create signature: {}", e))?,
                );
            }
        }

        Ok(())
    }
} 