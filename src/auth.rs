use tokio_tungstenite::tungstenite::http::HeaderMap;
use crate::supabase::SupabaseClient;

pub async fn validate_connection(headers: &HeaderMap, supabase: &SupabaseClient) -> bool {
    if let Some(auth_header) = headers.get("Authorization") {
        if let Ok(auth_str) = auth_header.to_str() {
            if let Some(token) = auth_str.strip_prefix("Bearer ") {
                match supabase.validate_api_key(token).await {
                    Ok(valid) => return valid,
                    Err(e) => {
                        tracing::error!("Error validating API key: {}", e);
                        return false;
                    }
                }
            }
        }
    }
    false
} 