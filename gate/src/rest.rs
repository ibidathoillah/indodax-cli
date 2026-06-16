use crate::types::{MarketPair, PriceTick, GateError};
use indodax_cli::integration::sdk::IndodaxClient;
use serde_json::Value;

pub struct RestClient {
    client: IndodaxClient,
}

impl RestClient {
    pub fn new() -> Self {
        Self {
            client: IndodaxClient::new(None).expect("failed to create IndodaxClient"),
        }
    }

    pub async fn list_pairs(&self) -> Result<Vec<MarketPair>, GateError> {
        let val: Value = self.client.public_get("/api/pairs").await
            .map_err(|e| GateError::Network(e.to_string()))?;
        
        let mut pairs = Vec::new();
        if let Some(arr) = val.as_array() {
            for item in arr {
                let base = item.get("traded_currency").and_then(|v| v.as_str()).unwrap_or_default();
                let quote = item.get("base_currency").and_then(|v| v.as_str()).unwrap_or_default();
                
                let is_maintenance = item.get("is_maintenance").and_then(|v| v.as_u64()).unwrap_or(0);
                let is_suspended = item.get("is_market_suspended").and_then(|v| v.as_u64()).unwrap_or(0);
                let active = is_maintenance == 0 && is_suspended == 0;
                
                if !base.is_empty() && !quote.is_empty() {
                    pairs.push(MarketPair {
                        symbol: format!("{}/{}", base.to_uppercase(), quote.to_uppercase()),
                        base: base.to_uppercase(),
                        quote: quote.to_uppercase(),
                        active,
                    });
                }
            }
        }
        Ok(pairs)
    }

    pub async fn last_price(&self, pair: &str) -> Result<PriceTick, GateError> {
        let clean_pair = pair.replace('/', "").to_lowercase();
        let path = format!("/api/ticker/{}", clean_pair);
        
        let val: Value = self.client.public_get(&path).await
            .map_err(|e| GateError::Network(e.to_string()))?;
            
        let ticker = val.get("ticker")
            .ok_or_else(|| GateError::Api("Missing ticker field".to_string()))?;
            
        let last_str = ticker.get("last")
            .and_then(|v| {
                v.as_str().map(|s| s.to_string())
                 .or_else(|| v.as_f64().map(|f| f.to_string()))
                 .or_else(|| v.as_u64().map(|u| u.to_string()))
            })
            .ok_or_else(|| GateError::Api("Missing last price field".to_string()))?;
            
        let price: f64 = last_str.parse()
            .map_err(|e| GateError::Api(format!("Invalid last price value ({}): {}", last_str, e)))?;
            
        Ok(PriceTick {
            symbol: pair.to_string(),
            price,
            timestamp: chrono::Utc::now().timestamp_millis() as u64,
        })
    }
}
