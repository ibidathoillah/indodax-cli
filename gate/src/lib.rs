pub mod types;
pub mod rest;
pub mod ws;
pub mod gate;

pub use types::{ExchangeGate, MarketPair, PriceTick, OrderbookLevel, OrderbookSnapshot, OrderRequest, OrderResponse, GateError, PriceStream, OrderbookStream};
pub use gate::IndodaxGate;
pub use rest::RestClient;
