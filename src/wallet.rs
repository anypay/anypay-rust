use bitcoin::{
    Network, Address as BtcAddress, psbt::Psbt, 
    secp256k1::{Secp256k1, Message, SecretKey, PublicKey},
    sighash::{SighashCache, EcdsaSighashType},
    ecdsa, Amount,
    Transaction, TxIn, TxOut, OutPoint, Script, ScriptBuf,
    transaction::Version,
    absolute::LockTime,
    transaction::Sequence,
    witness::Witness,
    address::Payload,
    consensus::encode::serialize_hex,
};
use bip32::{Mnemonic, XPrv, XPub, DerivationPath};
use rand_core::OsRng;
use anyhow::{Result, anyhow};
use std::str::FromStr;
use url::Url;
use crate::client::{AnypayClient, Utxo};
use crate::cards;
use serde::Deserialize;

pub struct Wallet {
    mnemonic: Mnemonic,
    master_key: XPrv,
}

pub struct BitcoinCard {
    pub chain: String,
    pub currency: String,
    pub network: Network,
    pub derivation_path: String,
    pub address: String,
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
    pub fn create_card(&self, chain: &str, currency: &str, network: Network, account: u32) -> Result<Box<dyn cards::Card>> {
        cards::create_card(chain, currency, network, account, self.seed_phrase())
    }

    pub fn parse_invoice_identifier(invoice: &str) -> Result<String> {
        if let Ok(url) = Url::parse(invoice) {
            if url.scheme() == "pay" {
                // Handle pay:?r=... URLs
                let r_param = url.query_pairs()
                    .find(|(key, _)| key == "r")
                    .ok_or_else(|| anyhow!("Invalid payment URL: missing 'r' parameter"))?
                    .1;
                return Self::extract_uid_from_url(&r_param.to_string());
            } else {
                // Handle https://anypayx.com/i/{uid}
                return Self::extract_uid_from_url(invoice);
            }
        }
        // Assume it's just a UID
        Ok(invoice.to_string())
    }

    fn extract_uid_from_url(url: &str) -> Result<String> {
        let parts: Vec<&str> = url.split('/').collect();
        parts.last()
            .ok_or_else(|| anyhow!("Invalid URL format"))
            .map(|s| s.to_string())
    }

    pub async fn fetch_invoice_details(uid: &str, api_key: &str) -> Result<InvoiceDetails> {
        let client = AnypayClient::new(api_key);
        let invoice = client.get_invoice(uid).await?;
        
        let mut outputs = Vec::new();
        for opt in &invoice.payment_options {
            let currency = opt.currency.clone();
            for inst in &opt.instructions {
                for out in &inst.outputs {
                    let amount = if currency == "BTC" {
                        out.amount  // Keep as satoshis for BTC
                    } else {
                        out.amount
                    };
                    outputs.push(PaymentOutput {
                        address: out.address.clone(),
                        amount,
                        currency: currency.clone(),
                    });
                }
            }
        }

        Ok(InvoiceDetails {
            uid: invoice.uid,
            outputs,
        })
    }

    pub fn select_utxos(utxos: &[Utxo], required_amount: Amount) -> Result<Vec<Utxo>> {
        let mut sorted_utxos = utxos.to_vec();
        sorted_utxos.sort_by(|a, b| {
            let a_amount = Amount::from_btc(a.amount).unwrap_or(Amount::ZERO);
            let b_amount = Amount::from_btc(b.amount).unwrap_or(Amount::ZERO);
            b_amount.cmp(&a_amount)
                .then_with(|| b.confirmations.cmp(&a.confirmations))
        });

        let mut selected = Vec::new();
        let mut total = Amount::ZERO;

        // First try to find a single UTXO that's close to the required amount
        if let Some(utxo) = sorted_utxos.iter().find(|utxo| {
            let amount = Amount::from_btc(utxo.amount).unwrap_or(Amount::ZERO);
            amount >= required_amount && amount <= required_amount * 2
        }).cloned() {
            selected.push(utxo);
            return Ok(selected);
        }

        // Otherwise, accumulate UTXOs until we have enough
        let mut remaining_utxos = sorted_utxos;
        while let Some(utxo) = remaining_utxos.pop() {
            selected.push(utxo);
            total += Amount::from_btc(selected.last().unwrap().amount).unwrap_or(Amount::ZERO);
            if total >= required_amount {
                break;
            }
        }

        if total < required_amount {
            return Err(anyhow!("Insufficient funds. Required: {}, Available: {}", 
                required_amount.to_btc(), total.to_btc()));
        }

        Ok(selected)
    }

    pub async fn pay_invoice(card: &Box<dyn cards::Card>, invoice: &InvoiceDetails) -> Result<()> {
        // Handle both BTC and FB payments
        let outputs = invoice.outputs.iter()
            .filter(|output| output.currency == card.currency())
            .collect::<Vec<_>>();

        if outputs.is_empty() {
            return Err(anyhow!("No {} payment options found for this invoice", card.currency()));
        }

        let api_key = std::env::var("ANYPAY_API_KEY")
            .map_err(|_| anyhow!("ANYPAY_API_KEY environment variable not set"))?;
        
        let client = AnypayClient::new(&api_key);

        // 1. Fetch UTXOs for the source address
        println!("Fetching UTXOs...");
        
        // Special handling for Fractal Bitcoin (FB) UTXOs
        let utxos = if card.chain() == "FB" {
            // Use the Fractal API to get UTXOs
            #[derive(Deserialize)]
            struct FractalUtxo {
                txid: String,
                vout: u32,
                value: u64,
                status: FractalUtxoStatus,
            }
            
            #[derive(Deserialize)]
            struct FractalUtxoStatus {
                confirmed: bool,
                block_height: Option<u32>,
                block_time: Option<u64>,
            }
            
            println!("Fetching UTXOs from Fractal API for {}", card.address());
            let response = reqwest::Client::new()
                .get(&format!("https://mempool.fractalbitcoin.io/api/v1/address/{}/utxo", card.address()))
                .send()
                .await?;
                
            if !response.status().is_success() {
                let error = response.text().await?;
                return Err(anyhow!("Failed to fetch UTXOs from Fractal API: {}", error));
            }
            
            let fractal_utxos = response.json::<Vec<FractalUtxo>>().await?;
            
            // Convert fractal UTXOs to our standard format
            fractal_utxos.into_iter()
                .map(|u| {
                    Utxo {
                        txid: u.txid,
                        vout: u.vout,
                        amount: u.value as f64 / 100_000_000.0, // Convert satoshis to BTC
                        confirmations: if u.status.confirmed { 1 } else { 0 }, // Simple confirmation handling
                        script_pub_key: String::new(),
                    }
                })
                .collect()
        } else {
            // For regular BTC, use the standard mempool API
            client.get_utxos(card.address()).await?
        };
        
        // 2. Calculate total required amount (including estimated fee)
        let fee_rate = 10.0; // sats/vbyte
        let total_output_amount = Amount::from_sat(
            outputs.iter()
                .map(|output| output.amount)
                .sum()
        );
        let estimated_size = 200; // Rough estimate for a typical transaction
        let fee_amount = Amount::from_sat((fee_rate * estimated_size as f64) as u64);
        let total_required = total_output_amount + fee_amount;

        // 3. Select UTXOs
        let selected_utxos = Self::select_utxos(&utxos, total_required)?;
        let total_input = selected_utxos.iter()
            .map(|utxo| Amount::from_btc(utxo.amount).unwrap_or(Amount::ZERO))
            .sum::<Amount>();

        // 4. Create transaction
        let mut tx_builder = Transaction {
            version: Version(2),
            lock_time: LockTime::ZERO,
            input: vec![],
            output: vec![],
        };

        // Add inputs
        for utxo in &selected_utxos {
            let outpoint = OutPoint::from_str(&format!("{}:{}", utxo.txid, utxo.vout))
                .map_err(|_| anyhow!("Invalid UTXO txid: {}", utxo.txid))?;
            tx_builder.input.push(TxIn {
                previous_output: outpoint,
                script_sig: ScriptBuf::new(),
                sequence: Sequence::MAX,
                witness: Witness::default(),
            });
        }

        // Add all payment outputs
        for output in outputs {
            println!("\nProcessing output address: {}", output.address);
            println!("Output amount: {} sats", output.amount);
            
            let recipient_address = BtcAddress::from_str(&output.address)
                .map_err(|e| anyhow!("Invalid recipient address {}: {}", output.address, e))?;
            
            let network_address = recipient_address
                .require_network(card.network())
                .map_err(|e| anyhow!("Address network mismatch for {}: {}", output.address, e))?;
            
            tx_builder.output.push(TxOut {
                value: Amount::from_sat(output.amount),
                script_pubkey: network_address.payload().script_pubkey(),
            });
        }

        // Add change output if needed
        let change_amount = total_input - total_output_amount - fee_amount;
        if change_amount > Amount::ZERO {
            let change_address = BtcAddress::from_str(card.address())
                .map_err(|e| anyhow!("Invalid change address: {}", e))?;
            
            tx_builder.output.push(TxOut {
                value: change_amount,
                script_pubkey: change_address.payload().script_pubkey(),
            });
        }

        // 5. Sign transaction
        let mut psbt = Psbt::from_unsigned_tx(tx_builder)?;
        
        // Add UTXO information
        for (i, utxo) in selected_utxos.iter().enumerate() {
            let script = ScriptBuf::from_hex(&utxo.script_pub_key)
                .map_err(|_| anyhow!("Invalid script: {}", utxo.script_pub_key))?;
            psbt.inputs[i].witness_utxo = Some(TxOut {
                value: Amount::from_btc(utxo.amount)?,
                script_pubkey: script,
            });
        }

        // Sign with the card's private key
        card.sign_transaction(&mut psbt)?;

        // Extract final transaction
        let final_tx = psbt.extract_tx()?;
        
        // Verify all outputs are present with correct amounts
        println!("\nVerifying transaction outputs:");
        for (i, output) in final_tx.output.iter().enumerate() {
            println!("Output {}: {} sats", i, output.value.to_sat());
            println!("Script: {}", output.script_pubkey.to_hex_string());
        }

        let tx_hex = serialize_hex(&final_tx);
        println!("\nTransaction hex: {}", tx_hex);

        // 6. Submit payment
        println!("Submitting payment...");
        client.submit_payment(&invoice.uid, card.chain(), card.currency(), &tx_hex).await?;

        println!("Payment submitted successfully!");

        Ok(())
    }
}

impl BitcoinCard {
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

impl std::fmt::Display for BitcoinCard {
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

#[derive(Debug)]
pub struct InvoiceDetails {
    pub uid: String,
    pub outputs: Vec<PaymentOutput>,
}

#[derive(Debug, Clone)]
pub struct PaymentOutput {
    pub address: String,
    pub amount: u64,  // Store as satoshis for BTC, regular amount for others
    pub currency: String,
} 