use async_trait::async_trait;
use futures_util::Stream;
use serde::{Deserialize, Serialize};
use std::pin::Pin;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketPair {
    pub symbol: String, // e.g. "BTC/IDR"
    pub base: String,   // e.g. "BTC"
    pub quote: String,  // e.g. "IDR"
    pub active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PriceTick {
    pub symbol: String,
    pub price: f64,
    pub timestamp: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderbookLevel {
    pub price: f64,
    pub amount: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderbookSnapshot {
    pub symbol: String,
    pub bids: Vec<OrderbookLevel>,
    pub asks: Vec<OrderbookLevel>,
    pub timestamp: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderRequest {
    pub symbol: String, // e.g. "BTC/IDR"
    pub side: String,   // "BUY" or "SELL"
    pub order_type: String, // "LIMIT" or "MARKET"
    pub price: Option<f64>,
    pub amount: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderResponse {
    pub order_id: String,
    pub symbol: String,
    pub status: String, // "OPEN", "FILLED", "REJECTED"
    pub filled_amount: f64,
    pub price: f64,
}

#[derive(Debug, thiserror::Error)]
pub enum GateError {
    #[error("network error: {0}")]
    Network(String),

    #[error("websocket error: {0}")]
    WebSocket(String),

    #[error("api error: {0}")]
    Api(String),

    #[error("invalid symbol: {0}")]
    InvalidSymbol(String),

    #[error("unknown error: {0}")]
    Other(String),
}

pub type PriceStream = Pin<Box<dyn Stream<Item = Result<PriceTick, GateError>> + Send>>;
pub type OrderbookStream = Pin<Box<dyn Stream<Item = Result<OrderbookSnapshot, GateError>> + Send>>;

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
pub trait ExchangeGate: Send + Sync {
    fn exchange_id(&self) -> &str;
    async fn list_pairs(&self) -> Result<Vec<MarketPair>, GateError>;
    async fn last_price(&self, pair: &str) -> Result<PriceTick, GateError>;
    async fn ws_price_stream(&self, pairs: &[String]) -> Result<PriceStream, GateError>;
    async fn ws_orderbook_stream(&self, pair: &str, depth: usize) -> Result<OrderbookStream, GateError>;
    async fn place_order(&self, order: OrderRequest) -> Result<OrderResponse, GateError>;
}
