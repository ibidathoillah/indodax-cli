use crate::types::{ExchangeGate, MarketPair, PriceTick, GateError, PriceStream, OrderbookStream, OrderRequest, OrderResponse};
use crate::rest::RestClient;
use crate::ws::{connect_price_stream, connect_orderbook_stream};
use async_trait::async_trait;
use std::sync::Arc;

pub struct IndodaxGate {
    rest: Arc<RestClient>,
}

impl IndodaxGate {
    pub fn new() -> Self {
        Self {
            rest: Arc::new(RestClient::new()),
        }
    }
}

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl ExchangeGate for IndodaxGate {
    fn exchange_id(&self) -> &str {
        "Indodax"
    }

    async fn list_pairs(&self) -> Result<Vec<MarketPair>, GateError> {
        self.rest.list_pairs().await
    }

    async fn last_price(&self, pair: &str) -> Result<PriceTick, GateError> {
        self.rest.last_price(pair).await
    }

    async fn ws_price_stream(&self, pairs: &[String]) -> Result<PriceStream, GateError> {
        connect_price_stream(pairs).await
    }

    async fn ws_orderbook_stream(&self, pair: &str, depth: usize) -> Result<OrderbookStream, GateError> {
        connect_orderbook_stream(pair, depth).await
    }

    async fn place_order(&self, order: OrderRequest) -> Result<OrderResponse, GateError> {
        let timestamp = chrono::Utc::now().timestamp_millis();
        let order_id = format!("paper-indodax-{}", timestamp);
        
        let is_limit = order.order_type.to_uppercase() == "LIMIT";
        let status = if is_limit {
            "OPEN".to_string()
        } else {
            "FILLED".to_string()
        };
        
        let price = order.price.unwrap_or(0.0);
        
        Ok(OrderResponse {
            order_id,
            symbol: order.symbol,
            status,
            filled_amount: if is_limit { 0.0 } else { order.amount },
            price,
        })
    }
}
