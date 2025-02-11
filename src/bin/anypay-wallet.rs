use clap::{Parser, Subcommand};
use bitcoin::Network;
use anyhow::{Result, anyhow};

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
    currency: String,
}

impl std::fmt::Display for Balance {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:.8} {} (${:.2} USD)", self.btc, self.currency, self.usd)
    }
}

async fn get_balance(card: &Box<dyn anypay::cards::Card>) -> Result<Balance> {
    let sats = card.get_balance().await?;
    let btc = card.get_decimal_balance().await?;
    let usd = card.get_usd_balance().await?;
    
    Ok(Balance { sats, btc, usd, currency: String::new() })
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    
    // Get seed phrase from args or env
    let seed_phrase = match args.seed_phrase {
        Some(phrase) => phrase,
        None => std::env::var("ANYPAY_WALLET_SEED_PHRASE")
            .map_err(|_| anyhow!("No seed phrase provided. Use --seed-phrase or set ANYPAY_WALLET_SEED_PHRASE"))?
    };

    match args.command {
        Commands::Generate => {
            let wallet = anypay::wallet::Wallet::new()?;
            println!("New wallet generated!");
            println!("Seed phrase: {}", wallet.seed_phrase());
        },
        Commands::CreateCard { chain, currency, network, account } => {
            let wallet = anypay::wallet::Wallet::from_seed_phrase(&seed_phrase)?;
            let network = match network.as_str() {
                "mainnet" => Network::Bitcoin,
                "testnet" => Network::Testnet,
                _ => return Err(anyhow!("Invalid network: {}", network))
            };
            
            let card = wallet.create_card(&chain, &currency, network, account)?;
            println!("Card created successfully!");
            println!("Chain: {}", card.chain());
            println!("Currency: {}", card.currency());
            println!("Network: {:?}", card.network());
            println!("Derivation Path: {}", card.derivation_path());
            println!("Address: {}", card.address());
        },
        Commands::ListCards => {
            // TODO: Implement card storage/listing
            println!("Card listing not yet implemented");
        },
        Commands::Balance { chain, currency, network, account } => {
            let wallet = anypay::wallet::Wallet::from_seed_phrase(&seed_phrase)?;
            let network = match network.as_str() {
                "mainnet" => Network::Bitcoin,
                "testnet" => Network::Testnet,
                _ => return Err(anyhow!("Invalid network: {}", network))
            };
            
            if let (Some(chain), Some(currency)) = (chain, currency) {
                // Check specific card balance
                let card = wallet.create_card(&chain, &currency, network, account)?;
                let balance = get_balance(&card).await?;
                println!("Balance for {}/{} card:", chain, currency);
                println!("{}", balance);
            } else {
                // Check all supported cards
                let supported_pairs = vec![
                    ("ETH", "ETH"),
                    ("POLYGON", "MATIC"),
                    ("XRPL", "XRP"),
                    ("SOL", "SOL"),
                    ("DOGE", "DOGE"),
                ];
                
                for (chain, currency) in supported_pairs {
                    if let Ok(card) = wallet.create_card(chain, currency, network, account) {
                        match get_balance(&card).await {
                            Ok(balance) => {
                                println!("Balance for {}/{} card:", chain, currency);
                                println!("{}", balance);
                            },
                            Err(e) => println!("Error getting {}/{} balance: {}", chain, currency, e),
                        }
                    }
                }
            }
        },
        Commands::Pay { invoice, chain, currency, network, account } => {
            let wallet = anypay::wallet::Wallet::from_seed_phrase(&seed_phrase)?;
            
            // Parse network
            let network = match network.as_str() {
                "mainnet" => Network::Bitcoin,
                "testnet" => Network::Testnet,
                _ => return Err(anyhow!("Invalid network: {}", network))
            };

            // Get API key from environment
            let api_key = std::env::var("ANYPAY_API_KEY")
                .map_err(|_| anyhow!("ANYPAY_API_KEY environment variable not set"))?;

            // Parse invoice identifier
            println!("Parsing invoice identifier...");
            let invoice_uid = anypay::wallet::Wallet::parse_invoice_identifier(&invoice)?;
            
            // Fetch invoice details
            println!("Fetching invoice details...");
            let invoice_details = anypay::wallet::Wallet::fetch_invoice_details(&invoice_uid, &api_key).await?;
            
            // Create card for payment
            println!("Creating card for {}/{}", chain, currency);
            let card = wallet.create_card(&chain, &currency, network, account)?;
            
            // Execute payment
            println!("Executing payment...");
            anypay::wallet::Wallet::pay_invoice(&card, &invoice_details).await?;
            
            println!("Payment submitted successfully!");
        }
    }

    Ok(())
} 