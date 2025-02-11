use super::Card;
use anyhow::{Result, anyhow};
use async_trait::async_trait;
use bitcoin::{
    Network, Address, PublicKey,
    secp256k1::{Secp256k1, SecretKey},
    psbt::Psbt,
    util::bip32::{ExtendedPrivKey, DerivationPath},
};
use std::str::FromStr;
use bip39::{Mnemonic, Language};

pub struct BitcoinCard {
    network: Network,
    account: u32,
    address: String,
    derivation_path: String,
    private_key: SecretKey,
}

impl BitcoinCard {
    pub fn new(network: Network, account: u32, seed_phrase: &str) -> Result<Self> {
        let mnemonic = Mnemonic::from_phrase(seed_phrase, Language::English)
            .map_err(|e| anyhow!("Invalid seed phrase: {}", e))?;
        
        let seed = mnemonic.to_seed("");
        let secp = Secp256k1::new();

        // Derive BIP44 path: m/44'/0'/account'/0/0 for BTC
        let path = format!("m/44'/0'/{}'/0/0", account);
        let derivation_path = DerivationPath::from_str(&path)
            .map_err(|e| anyhow!("Invalid derivation path: {}", e))?;

        let master_key = ExtendedPrivKey::new_master(network, &seed)
            .map_err(|e| anyhow!("Failed to derive master key: {}", e))?;
        let derived_key = master_key.derive_priv(&secp, &derivation_path)
            .map_err(|e| anyhow!("Failed to derive private key: {}", e))?;

        let private_key = derived_key.private_key;
        let public_key = PublicKey::from_private_key(&secp, &private_key);
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
impl Card for BitcoinCard {
    fn chain(&self) -> &str {
        "BTC"
    }

    fn currency(&self) -> &str {
        "BTC"
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
            .map(|utxo| bitcoin::Amount::from_btc(utxo.amount).unwrap_or(bitcoin::Amount::ZERO))
            .map(|amount| amount.to_sat())
            .sum();

        Ok(total_sats)
    }

    async fn get_decimal_balance(&self) -> Result<f64> {
        let sats = self.get_balance().await?;
        Ok(sats as f64 / 100_000_000.0)
    }

    async fn get_usd_balance(&self) -> Result<f64> {
        let btc = self.get_decimal_balance().await?;
        let api_key = std::env::var("ANYPAY_API_KEY")
            .map_err(|_| anyhow!("ANYPAY_API_KEY environment variable not set"))?;
        
        let client = crate::client::AnypayClient::new(&api_key);
        let btc_price = client.get_btc_price().await?;
        
        Ok(btc * btc_price)
    }

    fn sign_transaction(&self, psbt: &mut Psbt) -> Result<()> {
        use bitcoin::{
            sighash::{SighashCache, EcdsaSighashType},
            ecdsa::{self, Message},
        };

        let secp = Secp256k1::new();
        let mut sighash_cache = SighashCache::new(&psbt.unsigned_tx);
        
        // Sign each input
        for (i, input) in psbt.inputs.iter_mut().enumerate() {
            if let Some(witness_utxo) = &input.witness_utxo {
                let public_key = PublicKey::from_private_key(&secp, &self.private_key);
                
                // Calculate sighash
                let sighash = sighash_cache
                    .segwit_signature_hash(i, &witness_utxo.script_pubkey, witness_utxo.value, EcdsaSighashType::All)
                    .map_err(|e| anyhow!("Failed to calculate sighash: {}", e))?;

                // Sign the sighash
                let msg = Message::from_slice(&sighash[..])?;
                let sig = secp.sign_ecdsa(&msg, &self.private_key);
                let mut sig_bytes = sig.serialize_der().to_vec();
                sig_bytes.push(EcdsaSighashType::All as u8);
                let final_sig = ecdsa::Signature::from_slice(&sig_bytes)?;

                // Add the signature to the PSBT
                input.partial_sigs.insert(
                    public_key.into(),
                    final_sig,
                );
            }
        }

        Ok(())
    }
} 