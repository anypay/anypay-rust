use super::{Plugin, Account, Address, PaymentOption, Transaction, Payment, Confirmation, Price};
use anyhow::{Result, anyhow};
use bigdecimal::BigDecimal;
use std::str::FromStr;
use bitcoin::{Transaction as BtcTransaction, consensus::deserialize, Address as BtcAddress};
use reqwest::Client;


pub struct FractalBitcoinPlugin;

#[async_trait::async_trait]
impl Plugin for FractalBitcoinPlugin {
    fn currency(&self) -> &str { "FB" }
    fn chain(&self) -> &str { "FB" }
    fn decimals(&self) -> u8 { 8 }

    async fn build_signed_payment(&self, payment_option: &PaymentOption, mnemonic: &str) -> Result<Transaction> {
        // TODO: Implement FB transaction signing using bitcoin crate
        Ok(Transaction {
            txhex: "mock_fb_tx".into(),
            txid: Some("mock_fb_txid".into()),
            txkey: None,
        })
    }

    async fn verify_payment(&self, payment_option: &PaymentOption, transaction: &Transaction) -> Result<bool> {
        // Deserialize the raw transaction hex
        let tx_bytes = hex::decode(&transaction.txhex)?;
        let btc_tx: BtcTransaction = deserialize(&tx_bytes)?;

        // Calculate total output amount in satoshis
        let total_output_amount: u64 = btc_tx.output.iter()
            .map(|output| output.value.to_sat())
            .sum();

        // Convert payment option amount to satoshis for comparison
        let expected_amount = payment_option.amount as u64;

        // Verify that total outputs meet or exceed expected amount
        if total_output_amount < expected_amount {
            return Ok(false);
        }

        // Parse the payment address
        let payment_address = BtcAddress::from_str(&payment_option.address)
            .map_err(|e| anyhow!("Invalid Fractal Bitcoin address: {}", e))?;

        // Verify that at least one output matches the payment address
        let has_matching_output = btc_tx.output.iter().any(|output| {
            // Try to parse the output script to an address
            if let Ok(script_addr) = BtcAddress::from_script(&output.script_pubkey, bitcoin::Network::Bitcoin) {
                script_addr == payment_address
            } else {
                false
            }
        });

        Ok(has_matching_output)
    }

    async fn validate_address(&self, address: &str) -> Result<bool> {
        // TODO: Implement FB address validation
        // For now, assuming FB addresses follow the same format as BTC
        Ok(address.starts_with("1") || address.starts_with("3") || address.starts_with("fb1"))
    }

    async fn get_transaction(&self, txid: &str) -> Result<Transaction> {
        // TODO: Implement FB transaction fetching
        Ok(Transaction {
            txhex: "mock_fb_tx".into(),
            txid: Some(txid.to_string()),
            txkey: None,
        })
    }
    async fn broadcast_tx(&self, txhex: &str, txid: Option<&str>, txkey: Option<&str>) -> Result<Transaction> {
        let client = Client::new();
        let api_url = format!("{}/tx", "https://mempool.fractalbitcoin.io/api/v1");
        
        let response = client
            .post(&api_url)
            .header("Content-Type", "text/plain")
            .body(txhex.to_string())
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(anyhow::anyhow!("FB broadcast failed: {}", error_text));
        }

        let txid = response.text().await?;
        Ok(Transaction {
            txhex: txhex.to_string(),
            txid: Some(txid),
            txkey: None,
        })
    }


    async fn get_new_address(&self, _account: &Account, address: &Address) -> Result<String> {
        // TODO: Implement FB address generation
        Ok(address.value.clone())
    }

    async fn transform_address(&self, address: &str) -> Result<String> {
        Ok(address.split(':').last().unwrap_or(address).to_string())
    }

    async fn get_confirmation(&self, _txid: &str) -> Result<Option<Confirmation>> {
        // TODO: Implement FB confirmation checking
        Ok(Some(Confirmation {
            confirmations: 6,
            confirmed: true,
        }))
    }

    async fn get_payments(&self, txid: &str) -> Result<Vec<Payment>> {
        // TODO: Implement FB payment parsing
        Ok(vec![Payment {
            chain: self.chain().to_string(),
            currency: self.currency().to_string(),
            address: "mock_fb_address".to_string(),
            amount: 100000000, // 1 FB
            txid: txid.to_string(),
        }])
    }

    async fn parse_payments(&self, transaction: &Transaction) -> Result<Vec<Payment>> {
        // TODO: Implement FB transaction parsing
        Ok(vec![])
    }

    async fn get_price(&self) -> Result<Price> {
        // TODO: Implement price fetching from exchange
        Ok(Price {
            currency: self.currency().to_string(),
            price: BigDecimal::from_str("15000.00")?,
            timestamp: chrono::Utc::now().timestamp(),
        })
    }
}
