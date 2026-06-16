use crate::types::{PriceTick, GateError, PriceStream, OrderbookSnapshot, OrderbookLevel, OrderbookStream};
use futures_util::{StreamExt, SinkExt};
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};
use serde_json::Value;
use reqwest::Client;

const PUBLIC_WS_URL: &str = "wss://ws3.indodax.com/ws/";
const DEFAULT_STATIC_WS_TOKEN: &str = "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJleHAiOjE5NDY2MTg0MTV9.UR1lBM6Eqh0yWz-PVirw1uPCxe60FdchR8eNVdsskeo";

async fn get_ws_token() -> String {
    let client = Client::new();
    let res = client.get("https://indodax.com/api/ws/v1/generate_token").send().await;
    if let Ok(resp) = res {
        if let Ok(payload) = resp.json::<Value>().await {
            if let Some(token) = payload.get("token").and_then(|v| v.as_str()) {
                return token.to_string();
            }
            if let Some(token) = payload.get("data").and_then(|d| d.get("token")).and_then(|v| v.as_str()) {
                return token.to_string();
            }
        }
    }
    DEFAULT_STATIC_WS_TOKEN.to_string()
}

pub async fn connect_price_stream(pairs: &[String]) -> Result<PriceStream, GateError> {
    if pairs.is_empty() {
        return Err(GateError::InvalidSymbol("No pairs specified".to_string()));
    }

    let token = get_ws_token().await;
    let (ws_stream, _) = connect_async(PUBLIC_WS_URL).await
        .map_err(|e| GateError::WebSocket(e.to_string()))?;
        
    let (mut tx, rx) = ws_stream.split();
    
    let auth_msg = serde_json::json!({
        "params": { "token": token },
        "id": 1
    }).to_string();
    tx.send(Message::Text(auth_msg)).await
        .map_err(|e| GateError::WebSocket(e.to_string()))?;
        
    for pair in pairs {
        let clean = pair.replace('/', "").to_lowercase();
        let channel = format!("market:trade-activity-{}", clean);
        let sub_msg = serde_json::json!({
            "method": 1,
            "params": { "channel": channel },
            "id": 2
        }).to_string();
        tx.send(Message::Text(sub_msg)).await
            .map_err(|e| GateError::WebSocket(e.to_string()))?;
    }
    
    let pairs_clone = pairs.to_vec();
    let mapped = rx.filter_map(move |msg_res| {
        let pairs = pairs_clone.clone();
        async move {
            match msg_res {
                Ok(Message::Text(text)) => {
                    let val: Value = serde_json::from_str(&text).ok()?;
                    let result = val.get("result")?;
                    let channel = result.get("channel").and_then(|v| v.as_str())?;
                    
                    let pair = pairs.iter().find(|p| {
                        let clean = p.replace('/', "").to_lowercase();
                        channel.contains(&clean)
                    })?;
                    
                    let data = result.get("data")?.get("data")?.as_array()?;
                    let latest_trade = data.last()?.as_array()?;
                    let timestamp = latest_trade.get(1)?.as_u64()?;
                    let price_val = latest_trade.get(4)?;
                    let price = price_val.as_f64()
                        .or_else(|| price_val.as_str().and_then(|s| s.parse().ok()))?;
                        
                    Some(Ok(PriceTick {
                        symbol: pair.clone(),
                        price,
                        timestamp: timestamp * 1000,
                    }))
                }
                Err(e) => Some(Err(GateError::WebSocket(e.to_string()))),
                _ => None,
            }
        }
    });
    
    Ok(Box::pin(mapped))
}

pub async fn connect_orderbook_stream(pair: &str, _depth: usize) -> Result<OrderbookStream, GateError> {
    let token = get_ws_token().await;
    let (ws_stream, _) = connect_async(PUBLIC_WS_URL).await
        .map_err(|e| GateError::WebSocket(e.to_string()))?;
        
    let (mut tx, rx) = ws_stream.split();
    
    let auth_msg = serde_json::json!({
        "params": { "token": token },
        "id": 1
    }).to_string();
    tx.send(Message::Text(auth_msg)).await
        .map_err(|e| GateError::WebSocket(e.to_string()))?;
        
    let clean = pair.replace('/', "").to_lowercase();
    let channel = format!("market:order-book-{}", clean);
    let sub_msg = serde_json::json!({
        "method": 1,
        "params": { "channel": channel },
        "id": 2
    }).to_string();
    tx.send(Message::Text(sub_msg)).await
        .map_err(|e| GateError::WebSocket(e.to_string()))?;
        
    let symbol = pair.to_string();
    let mapped = rx.filter_map(move |msg_res| {
        let symbol = symbol.clone();
        async move {
            match msg_res {
                Ok(Message::Text(text)) => {
                    let val: Value = serde_json::from_str(&text).ok()?;
                    let data = val.get("result")?.get("data")?.get("data")?;
                    let bids_val = data.get("bid").or_else(|| data.get("buy")).and_then(|v| v.as_array())?;
                    let asks_val = data.get("ask").or_else(|| data.get("sell")).and_then(|v| v.as_array())?;
                    
                    let parse_levels = |levels: &Vec<Value>| -> Vec<OrderbookLevel> {
                        levels.iter().filter_map(|level| {
                            let arr = level.as_array()?;
                            let price = arr.first()?.as_f64()
                                .or_else(|| arr.first()?.as_str().and_then(|s| s.parse().ok()))?;
                            let amount = arr.get(1)?.as_f64()
                                .or_else(|| arr.get(1)?.as_str().and_then(|s| s.parse().ok()))?;
                            Some(OrderbookLevel { price, amount })
                        }).collect()
                    };
                    
                    let bids = parse_levels(bids_val);
                    let asks = parse_levels(asks_val);
                    let timestamp = chrono::Utc::now().timestamp_millis() as u64;
                    
                    Some(Ok(OrderbookSnapshot {
                        symbol,
                        bids,
                        asks,
                        timestamp,
                    }))
                }
                Err(e) => Some(Err(GateError::WebSocket(e.to_string()))),
                _ => None,
            }
        }
    });
    
    Ok(Box::pin(mapped))
}
