use std::collections::HashMap;
use lazy_static::lazy_static;

lazy_static! {
    static ref PROTOCOLS: HashMap<&'static str, &'static str> = {
        let mut m = HashMap::new();
        m.insert("DASH", "dash");
        m.insert("ZEC", "zcash");
        m.insert("BTC", "bitcoin");
        m.insert("LTC", "litecoin");
        m.insert("ETH", "ethereum");
        m.insert("XMR", "monero");
        m.insert("DOGE", "dogecoin");
        m.insert("BCH", "bitcoincash");
        m.insert("XRP", "ripple");
        m.insert("ZEN", "horizen");
        m.insert("SMART", "smartcash");
        m.insert("RVN", "ravencoin");
        m.insert("BSV", "pay");
        m
    };
}

#[derive(Debug)]
pub struct InvoiceUriParams {
    pub currency: String,
    pub uid: String,
}

pub fn compute_invoice_uri(params: &InvoiceUriParams) -> String {
    let protocol = PROTOCOLS.get(params.currency.as_str()).unwrap_or(&"pay");
    let base_url = get_base_url();
    
    format!("{}:?r={}/r/{}", protocol, base_url, params.uid)
}

fn get_base_url() -> String {
    std::env::var("BASE_URL").unwrap_or_else(|_| "https://api.anypayx.com".to_string())
} 