use bitcoin::{
    Network, Address, psbt::Psbt, 
    secp256k1::{Secp256k1, Message, SecretKey, PublicKey},
    sighash::{SighashCache, EcdsaSighashType},
    ecdsa,
};
use bip32::{Mnemonic, XPrv, XPub, DerivationPath};
use rand_core::OsRng;
use anyhow::{Result, anyhow};
use std::str::FromStr;

pub struct Wallet {
    mnemonic: Mnemonic,
    master_key: XPrv,
}

pub struct Card {
    pub chain: String,
    pub currency: String,
    pub network: Network,
    pub derivation_path: String,
    pub address: Address,
    pub private_key: XPrv,
    pub public_key: XPub,
}

impl Wallet {
    /// Create a new wallet from an existing seed phrase
    pub fn from_seed_phrase(seed_phrase: &str) -> Result<Self> {
        let mnemonic = Mnemonic::new(seed_phrase, Default::default())
            .map_err(|e| anyhow!("Invalid seed phrase: {}", e))?;
        
        let seed = mnemonic.to_seed("");
        let master_key = XPrv::new(&seed)
            .map_err(|e| anyhow!("Failed to derive master key: {}", e))?;

        Ok(Self {
            mnemonic,
            master_key,
        })
    }

    /// Generate a new wallet with a random seed phrase
    pub fn new() -> Result<Self> {
        let mnemonic = Mnemonic::random(&mut OsRng, Default::default());
        let seed = mnemonic.to_seed("");
        let master_key = XPrv::new(&seed)
            .map_err(|e| anyhow!("Failed to derive master key: {}", e))?;

        Ok(Self {
            mnemonic,
            master_key,
        })
    }

    /// Get the seed phrase
    pub fn seed_phrase(&self) -> &str {
        self.mnemonic.phrase()
    }

    /// Create a new card for a specific chain and currency
    pub fn create_card(&self, chain: &str, currency: &str, network: Network, account: u32) -> Result<Card> {
        // Define BIP44 derivation path:
        // m/44'/coin_type'/account'/0/0
        let coin_type = match (chain, currency) {
            ("BTC", "BTC") => 0,
            ("ETH", "ETH") => 60,
            ("BSV", "BSV") => 236,
            ("XRP", "XRP") => 144,
            _ => return Err(anyhow!("Unsupported chain/currency combination: {}/{}", chain, currency))
        };

        let path = format!("m/44'/{}'/{}'/{}/{}", coin_type, account, 0, 0);
        let derivation_path = DerivationPath::from_str(&path)
            .map_err(|e| anyhow!("Invalid derivation path: {}", e))?;

        // Derive child key
        let private_key = XPrv::derive_from_path(&self.master_key.to_bytes(), &derivation_path)
            .map_err(|e| anyhow!("Failed to derive child key: {}", e))?;

        // Get public key
        let public_key = private_key.public_key();

        // Generate address (currently only supports Bitcoin-style addresses)
        let pubkey_bytes = public_key.to_bytes();
        let address = match chain {
            "BTC" => Address::p2wpkh(&bitcoin::PublicKey::from_slice(&pubkey_bytes)?, network)
                .map_err(|e| anyhow!("Failed to generate Bitcoin address: {}", e))?,
            // TODO: Add support for other chain address formats
            _ => return Err(anyhow!("Address generation not yet implemented for chain: {}", chain))
        };

        Ok(Card {
            chain: chain.to_string(),
            currency: currency.to_string(),
            network,
            derivation_path: path,
            address,
            private_key,
            public_key,
        })
    }
}

impl Card {
    pub fn sign_bitcoin_transaction(&self, psbt: &mut Psbt) -> Result<()> {
        let secp = Secp256k1::new();
        let mut sighash_cache = SighashCache::new(&psbt.unsigned_tx);
        
        // Sign each input
        for (i, input) in psbt.inputs.iter_mut().enumerate() {
            if let Some(witness_utxo) = &input.witness_utxo {
                // Convert bip32 private key to secp256k1 secret key
                let secret_bytes = self.private_key.to_bytes();
                let secret_key = SecretKey::from_slice(&secret_bytes)
                    .map_err(|e| anyhow!("Invalid private key: {}", e))?;
                let public_key = PublicKey::from_secret_key(&secp, &secret_key);
                
                // Calculate sighash
                let sighash = sighash_cache
                    .segwit_signature_hash(i, &witness_utxo.script_pubkey, witness_utxo.value, EcdsaSighashType::All)
                    .map_err(|e| anyhow!("Failed to calculate sighash: {}", e))?;

                // Sign the sighash
                let msg = Message::from_slice(&sighash[..])?;
                let sig = secp.sign_ecdsa(&msg, &secret_key);
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

impl std::fmt::Display for Card {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Chain: {}\nCurrency: {}\nNetwork: {:?}\nDerivation Path: {}\nAddress: {}", 
            self.chain,
            self.currency,
            self.network,
            self.derivation_path,
            self.address
        )
    }
} 