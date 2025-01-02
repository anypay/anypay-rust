use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct InvoiceUriParams {
    pub currency: String,
    pub uid: String,
}

pub fn compute_invoice_uri(params: &InvoiceUriParams) -> String {
    // Format: anypay:{currency}_{uid}
    format!("anypay:{}_{}", params.currency.to_lowercase(), params.uid)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compute_invoice_uri() {
        let params = InvoiceUriParams {
            currency: "BTC".to_string(),
            uid: "inv_123".to_string(),
        };

        let uri = compute_invoice_uri(&params);
        assert_eq!(uri, "anypay:btc_inv_123");
    }
} 