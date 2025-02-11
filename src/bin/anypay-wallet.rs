use clap::{Parser, Subcommand};
use bitcoin::Network;
use anyhow::{Result, anyhow};
use anypay::wallet::Wallet;
use anypay::client::AnypayClient;
use serde_json::json;
use url::Url;
use std::env;
use bitcoin::{
    Transaction, TxIn, TxOut, OutPoint, Script, ScriptBuf, Address as BtcAddress,
    Amount,
};
use bitcoin::transaction::Version;
use bitcoin::absolute::LockTime;
use bitcoin::transaction::Sequence;
use bitcoin::witness::Witness;
use bitcoin::consensus::encode::serialize_hex;
use bitcoin::hashes::hex::FromHex;
use bitcoin::psbt::Psbt;
use anypay::client::Utxo;
use std::str::FromStr;
use bitcoin::address::Payload;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// BIP39 seed phrase (or set ANYPAY_WALLET_SEED_PHRASE env var)
    #[arg(long, env = "ANYPAY_WALLET_SEED_PHRASE")]
    seed_phrase: Option<String>,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Generate a new wallet
    Generate,

    /// Create a new card for a specific chain
    CreateCard {
        /// Chain to use (BTC, ETH, BSV, XRP)
        #[arg(long)]
        chain: String,

        /// Currency to use (BTC, ETH, BSV, XRP)
        #[arg(long)]
        currency: String,

        /// Network to use (mainnet or testnet)
        #[arg(long, default_value = "mainnet")]
        network: String,

        /// Account index for derivation
        #[arg(long, default_value = "0")]
        account: u32,
    },

    /// List all cards in the wallet
    ListCards,

    /// Get balance for all cards or a specific card
    Balance {
        /// Chain to check (optional - if not provided, shows all balances)
        #[arg(long)]
        chain: Option<String>,

        /// Currency to check
        #[arg(long)]
        currency: Option<String>,

        /// Network to use (mainnet or testnet)
        #[arg(long, default_value = "mainnet")]
        network: String,

        /// Account index
        #[arg(long, default_value = "0")]
        account: u32,
    },

    /// Pay an Anypay invoice
    Pay {
        /// Invoice URL or UID (https://anypayx.com/i/{uid}, pay:?r=..., or just {uid})
        invoice: String,

        /// Chain to pay with (BTC, ETH, BSV, XRP)
        #[arg(long)]
        chain: String,

        /// Currency to pay with (BTC, ETH, BSV, XRP)
        #[arg(long)]
        currency: String,

        /// Network to use (mainnet or testnet)
        #[arg(long, default_value = "mainnet")]
        network: String,

        /// Account index to pay from
        #[arg(long, default_value = "0")]
        account: u32,
    },
}

#[derive(Debug)]
struct Balance {
    sats: u64,
    btc: f64,
    usd: f64,
}

impl std::fmt::Display for Balance {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:.8} BTC (${:.2} USD)", self.btc, self.usd)
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    // Load or create wallet
    let wallet = match args.command {
        Commands::Generate => {
            let wallet = Wallet::new()?;
            println!("\n🔐 Generated new seed phrase (KEEP THIS SAFE!):");
            println!("{}\n", wallet.seed_phrase());
            wallet
        }
        _ => {
            let seed_phrase = args.seed_phrase
                .ok_or_else(|| anyhow!("Please provide a seed phrase via --seed-phrase or ANYPAY_WALLET_SEED_PHRASE"))?;
            Wallet::from_seed_phrase(&seed_phrase)?
        }
    };

    match args.command {
        Commands::Generate => Ok(()), // Already handled above

        Commands::CreateCard { chain, currency, network, account } => {
            let network = match network.as_str() {
                "mainnet" => Network::Bitcoin,
                "testnet" => Network::Testnet,
                _ => return Err(anyhow!("Invalid network. Use 'mainnet' or 'testnet'")),
            };

            let card = wallet.create_card(&chain, &currency, network, account)?;
            println!("\n💳 Card Info:");
            println!("{}", card);
            Ok(())
        }

        Commands::ListCards => {
            println!("\n👛 Wallet Info:");
            println!("Seed Phrase: {}", wallet.seed_phrase());
            println!("\nAvailable Chains/Currencies:");
            println!("- BTC/BTC (Bitcoin)");
            println!("- ETH/ETH (Ethereum)");
            println!("- BSV/BSV (Bitcoin SV)");
            println!("- XRPL/XRP (Ripple)");
            println!("\nUse the create-card command to generate addresses for specific chains");
            Ok(())
        }

        Commands::Balance { chain, currency, network, account } => {
            let network = match network.as_str() {
                "mainnet" => Network::Bitcoin,
                "testnet" => Network::Testnet,
                _ => return Err(anyhow!("Invalid network. Use 'mainnet' or 'testnet'")),
            };

            if let (Some(chain), Some(currency)) = (chain, currency) {
                // Get balance for specific card
                let card = wallet.create_card(&chain, &currency, network, account)?;
                let balance = get_balance(&card).await?;
                println!("\n💰 Balance for {}:", card.address);
                println!("Satoshis: {} sats", balance.sats);
                println!("Bitcoin: {:.8} BTC", balance.btc);
                println!("USD Value: ${:.2}", balance.usd);
            } else {
                // Get all balances
                println!("\n💰 All Balances:");
                for (chain, currency) in [("BTC", "BTC"), ("ETH", "ETH"), ("BSV", "BSV"), ("XRP", "XRP")] {
                    if let Ok(card) = wallet.create_card(chain, currency, network, account) {
                        if let Ok(balance) = get_balance(&card).await {
                            println!("{} {}: {} sats ({:.8} BTC = ${:.2})", 
                                chain, 
                                card.address, 
                                balance.sats,
                                balance.btc,
                                balance.usd
                            );
                        }
                    }
                }
            }
            Ok(())
        }

        Commands::Pay { invoice, chain, currency, network, account } => {
            let network = match network.as_str() {
                "mainnet" => Network::Bitcoin,
                "testnet" => Network::Testnet,
                _ => return Err(anyhow!("Invalid network. Use 'mainnet' or 'testnet'")),
            };

            // Parse invoice identifier
            let invoice_uid = parse_invoice_identifier(&invoice)?;

            // Create card for payment
            let card = wallet.create_card(&chain, &currency, network, account)?;

            // Get invoice details
            let invoice_details = fetch_invoice_details(&invoice_uid).await?;
            println!("\n📄 Invoice Details:");
            println!("Invoice ID: {}", invoice_details.uid);
            println!("\nPayment Options:");
            for (i, output) in invoice_details.outputs.iter().enumerate() {
                println!("{}. {} {} to {}", 
                    i + 1,
                    if output.currency == "BTC" { format!("{} sats", output.amount) } else { output.amount.to_string() },
                    output.currency,
                    output.address
                );
            }

            // Find matching payment option for user's chosen currency
            let matching_output = invoice_details.outputs.iter()
                .find(|output| output.currency == currency)
                .ok_or_else(|| anyhow!("No payment option found for currency: {}", currency))?;

            // Confirm payment
            println!("\nPay {} {} to {} using {}? (y/N)", 
                if currency == "BTC" { format!("{} sats", matching_output.amount) } else { matching_output.amount.to_string() },
                currency,
                matching_output.address,
                card.address);

            let mut input = String::new();
            std::io::stdin().read_line(&mut input)?;
            if input.trim().to_lowercase() == "y" {
                pay_invoice(&card, &invoice_details).await?;
                println!("✅ Payment sent successfully!");
            } else {
                println!("Payment cancelled");
            }
            Ok(())
        }
    }
}

async fn get_balance(card: &anypay::wallet::Card) -> Result<Balance> {
    if card.chain != "BTC" || card.currency != "BTC" {
        return Err(anyhow!("Balance checking only supported for BTC/BTC"));
    }

    let api_key = env::var("ANYPAY_API_KEY")
        .map_err(|_| anyhow!("ANYPAY_API_KEY environment variable not set"))?;
    
    let client = AnypayClient::new(&api_key);

    // Fetch UTXOs
    let utxos = client.get_utxos(&card.address.to_string()).await?;
    
    // Calculate total balance in satoshis
    let total_sats: u64 = utxos.iter()
        .map(|utxo| Amount::from_btc(utxo.amount).unwrap_or(Amount::ZERO))
        .map(|amount| amount.to_sat())
        .sum();

    // Convert to BTC
    let total_btc = Amount::from_sat(total_sats).to_btc();

    // Get current BTC price
    let btc_price = client.get_btc_price().await?;
    let total_usd = total_btc * btc_price;

    Ok(Balance {
        sats: total_sats,
        btc: total_btc,
        usd: total_usd,
    })
}

#[derive(Debug)]
struct InvoiceDetails {
    uid: String,
    outputs: Vec<PaymentOutput>,
}

#[derive(Debug, Clone)]
struct PaymentOutput {
    address: String,
    amount: u64,  // Store as satoshis for BTC, regular amount for others
    currency: String,
}

fn parse_invoice_identifier(invoice: &str) -> Result<String> {
    if let Ok(url) = Url::parse(invoice) {
        if url.scheme() == "pay" {
            // Handle pay:?r=... URLs
            let r_param = url.query_pairs()
                .find(|(key, _)| key == "r")
                .ok_or_else(|| anyhow!("Invalid payment URL: missing 'r' parameter"))?
                .1;
            return extract_uid_from_url(&r_param.to_string());
        } else {
            // Handle https://anypayx.com/i/{uid}
            return extract_uid_from_url(invoice);
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

async fn fetch_invoice_details(uid: &str) -> Result<InvoiceDetails> {
    let api_key = env::var("ANYPAY_API_KEY")
        .map_err(|_| anyhow!("ANYPAY_API_KEY environment variable not set"))?;
    
    let client = AnypayClient::new(&api_key);
    let invoice = client.get_invoice(uid).await?;
    
    let mut outputs = Vec::new();
    for opt in &invoice.payment_options {
        let currency = opt.currency.clone();
        println!("\nProcessing payment option for currency: {}", currency);
        for inst in &opt.instructions {
            for out in &inst.outputs {
                println!("Raw output amount: {}", out.amount);
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
                println!("Added output: {} {} to {}", 
                    if currency == "BTC" { format!("{} sats", amount) } else { amount.to_string() },
                    currency, 
                    out.address
                );
            }
        }
    }

    Ok(InvoiceDetails {
        uid: invoice.uid,
        outputs,
    })
}

async fn pay_invoice(card: &anypay::wallet::Card, invoice: &InvoiceDetails) -> Result<()> {
    // Only handle BTC payments for now
    let outputs = invoice.outputs.iter()
        .filter(|output| output.currency == "BTC")
        .collect::<Vec<_>>();

    if outputs.is_empty() {
        return Err(anyhow!("No BTC payment options found"));
    }

    let api_key = env::var("ANYPAY_API_KEY")
        .map_err(|_| anyhow!("ANYPAY_API_KEY environment variable not set"))?;
    
    let client = AnypayClient::new(&api_key);

    // 1. Fetch UTXOs for the source address
    println!("Fetching UTXOs...");
    let utxos = client.get_utxos(&card.address.to_string()).await?;
    
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
    let selected_utxos = select_utxos(&utxos, total_required)?;
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
        
        // Check if it's a Taproot address
        let is_taproot = output.address.starts_with("bc1p");
        println!("Is Taproot address: {}", is_taproot);
        
        let recipient_address = BtcAddress::from_str(&output.address)
            .map_err(|e| anyhow!("Invalid recipient address {}: {}", output.address, e))?;
        
        let network_address = recipient_address
            .require_network(card.network)
            .map_err(|e| anyhow!("Address network mismatch for {}: {}", output.address, e))?;
        
        let output_amount = Amount::from_sat(output.amount);
        let script_pubkey = network_address.script_pubkey();
        
        // Debug output for script analysis
        println!("Script details for {}", output.address);
        println!("Amount: {} sats", output_amount.to_sat());
        println!("Script hex: {}", script_pubkey.to_hex_string());
        println!("Script asm: {}", script_pubkey.to_asm_string());
        
        // For Taproot addresses, verify the script format
        if is_taproot {
            let script_hex = script_pubkey.to_hex_string();
            if !script_hex.starts_with("5120") {
                return Err(anyhow!("Invalid Taproot script - should start with OP_1 (0x51) followed by 0x20 for 32-byte push"));
            }
            println!("✓ Verified Taproot script format");
            
            // Extract and verify the key
            let key_hex = &script_hex[4..];  // Skip the 5120 prefix
            println!("Taproot key: {}", key_hex);
            
            // Try to reconstruct the address
            if let Ok(addr) = BtcAddress::from_script(&script_pubkey, card.network) {
                println!("Reconstructed: {}", addr);
                if addr.to_string() != output.address {
                    return Err(anyhow!("Address mismatch:\nExpected: {}\nGot:      {}", output.address, addr));
                }
                println!("✓ Address reconstruction verified");
            } else {
                return Err(anyhow!("Failed to reconstruct address from script"));
            }
        }
        
        tx_builder.output.push(TxOut {
            value: output_amount,
            script_pubkey,
        });
        println!("Added output {} of {} sats to {}", 
            tx_builder.output.len() - 1,
            output_amount.to_sat(), 
            output.address);
    }

    // Add change output if necessary
    let change_amount = total_input - total_output_amount - fee_amount;
    if change_amount > Amount::ZERO {
        let change_address = BtcAddress::from_str(&card.address.to_string())
            .map_err(|_| anyhow!("Invalid change address: {}", card.address))?
            .require_network(card.network)
            .map_err(|_| anyhow!("Address network mismatch"))?;
        tx_builder.output.push(TxOut {
            value: Amount::from_sat(change_amount.to_sat()),
            script_pubkey: change_address.script_pubkey(),
        });
        println!("Added change output: {} BTC to {}", Amount::from_sat(change_amount.to_sat()).to_btc(), card.address);
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
    card.sign_bitcoin_transaction(&mut psbt)?;

    // Extract final transaction
    let final_tx = psbt.extract_tx()?;
    
    // Verify all outputs are present with correct amounts
    println!("\nVerifying transaction outputs:");
    for (i, output) in final_tx.output.iter().enumerate() {
        println!("Output {}: {} sats", i, output.value.to_sat());
        println!("Script: {}", output.script_pubkey.to_hex_string());
    }

    // Double check the specific output we're looking for
    let found_output = final_tx.output.iter()
        .find(|out| {
            let addr = BtcAddress::from_script(&out.script_pubkey, card.network).ok();
            addr.map(|a| a.to_string()) == Some("bc1pgvvd6umckwre32s74p0n4yqetq9mexhk4egglv6arz7k39kzk6dqwxaj9t".to_string())
        });

    if let Some(out) = found_output {
        println!("\nFound target output:");
        println!("Amount: {} sats", out.value.to_sat());
        println!("Script: {}", out.script_pubkey.to_hex_string());
    } else {
        println!("\nWARNING: Target output not found in final transaction!");
    }

    let tx_hex = serialize_hex(&final_tx);
    println!("\nTransaction hex: {}", tx_hex);

    // 6. Submit payment
    println!("Submitting payment...");
    client.submit_payment(&invoice.uid, "BTC", "BTC", &tx_hex).await?;

    Ok(())
}

fn select_utxos(utxos: &[Utxo], required_amount: Amount) -> Result<Vec<Utxo>> {
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