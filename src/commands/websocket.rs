use crate::client::IndodaxClient;
use crate::commands::helpers;
use crate::output::CommandOutput;
use anyhow::Result;
use futures_util::{SinkExt, StreamExt};
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::Message;

const PUBLIC_WS_URL: &str = "wss://ws3.indodax.com/ws/";
const PRIVATE_WS_URL: &str = "wss://pws.indodax.com/ws/?cf_ws_frame_ping_pong=true";

const WS_TOKEN: &str = "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJleHAiOjE5NDY2MTg0MTV9.UR1lBM6Eqh0yWz-PVirw1uPCxe60FdchR8eNVdsskeo";

#[derive(Debug, clap::Subcommand)]
pub enum WebSocketCommand {
    #[command(name = "ticker", about = "Stream real-time ticker for a pair")]
    Ticker {
        #[arg(default_value = "btc_idr")]
        pair: String,
    },

    #[command(name = "trades", about = "Stream real-time trades for a pair")]
    Trades {
        #[arg(default_value = "btc_idr")]
        pair: String,
    },

    #[command(name = "book", about = "Stream real-time order book for a pair")]
    Book {
        #[arg(default_value = "btc_idr")]
        pair: String,
    },

    #[command(name = "summary", about = "Stream 24h summary for all pairs")]
    Summary,

    #[command(name = "orders", about = "Stream private order updates")]
    Orders,
}

pub async fn execute(
    _client: &IndodaxClient,
    cmd: &WebSocketCommand,
) -> Result<CommandOutput> {
    match cmd {
        WebSocketCommand::Ticker { pair } => ws_ticker(pair).await,
        WebSocketCommand::Trades { pair } => ws_trades(pair).await,
        WebSocketCommand::Book { pair } => ws_book(pair).await,
        WebSocketCommand::Summary => ws_summary().await,
        WebSocketCommand::Orders => ws_orders().await,
    }
}

async fn ws_connect_and_listen(
    channel: &str,
    handler: impl Fn(serde_json::Value),
) -> Result<CommandOutput> {
    eprintln!("Connecting to Indodax WebSocket...");
    let (mut ws_stream, _) = connect_async(PUBLIC_WS_URL).await?;
    eprintln!("Connected. Authenticating...");

    let auth_msg = serde_json::json!({
        "params": { "token": WS_TOKEN },
        "id": 1
    });
    ws_stream
        .send(Message::Text(auth_msg.to_string().into()))
        .await?;

    let mut authed = false;

    while let Some(msg) = ws_stream.next().await {
        match msg {
            Ok(Message::Text(text)) => {
                let val = match serde_json::from_str::<serde_json::Value>(&text) {
                    Ok(v) => v,
                    Err(_) => continue,
                };

                if !authed {
                    if val.get("id") == Some(&serde_json::Value::Number(1.into()))
                        && val.get("result").is_some()
                    {
                        authed = true;
                        eprintln!("Authenticated. Subscribing to channel: {}\n", channel);
                        let sub_msg = serde_json::json!({
                            "method": 1,
                            "params": { "channel": channel },
                            "id": 2
                        });
                        ws_stream
                            .send(Message::Text(sub_msg.to_string().into()))
                            .await?;
                    }
                    continue;
                }

                if val.get("id") == Some(&serde_json::Value::Number(2.into())) {
                    // subscription confirmation, skip
                } else if val.get("result").is_some() {
                    handler(val);
                }
            }
            Ok(Message::Ping(_)) => {
                let _ = ws_stream.send(Message::Pong(vec![])).await;
            }
            Ok(Message::Close(_)) => {
                eprintln!("Connection closed by server.");
                break;
            }
            Err(e) => {
                eprintln!("WebSocket error: {}", e);
                break;
            }
            _ => {}
        }
    }

    Ok(CommandOutput::json(serde_json::json!({"status": "disconnected"})))
}

fn format_ws_price(val: &serde_json::Value) -> String {
    val.as_u64()
        .or_else(|| val.as_f64().map(|f| f as u64))
        .or_else(|| val.as_str().and_then(|s| s.parse().ok()))
        .map(|v| format!("{}", v))
        .unwrap_or_else(|| val.to_string())
}

async fn ws_ticker(pair: &str) -> Result<CommandOutput> {
    let channel = format!("chart:tick-{}", pair);
    ws_connect_and_listen(&channel, |val| {
        let rows = &val["result"]["data"]["data"];
        if let serde_json::Value::Array(arr) = rows {
            for row in arr {
                if let serde_json::Value::Array(fields) = row {
                    if fields.len() >= 4 {
                        let ts = fields[0].as_u64().unwrap_or(0);
                        let price = format_ws_price(&fields[2]);
                        let time_str = chrono::DateTime::from_timestamp(ts as i64, 0)
                            .map(|d| d.format("%H:%M:%S").to_string())
                            .unwrap_or_default();
                        println!("[{}] {}  {}", time_str, pair, price);
                    }
                }
            }
        }
    })
    .await
}

async fn ws_trades(pair: &str) -> Result<CommandOutput> {
    let channel = format!("market:trade-activity-{}", pair);
    ws_connect_and_listen(&channel, |val| {
        let rows = &val["result"]["data"]["data"];
        if let serde_json::Value::Array(arr) = rows {
            for row in arr {
                if let serde_json::Value::Array(fields) = row {
                    if fields.len() >= 7 {
                        let ts = fields[1].as_u64().unwrap_or(0);
                        let side = fields[3].as_str().unwrap_or("?");
                        let price = fields[4].as_f64().unwrap_or(0.0);
                        let volume = fields[6].as_str().and_then(|s| s.parse().ok()).unwrap_or(0.0);
                        let time_str = chrono::DateTime::from_timestamp(ts as i64, 0)
                            .map(|d| d.format("%H:%M:%S").to_string())
                            .unwrap_or_default();
                        println!("[{}] {} {} @ {} vol: {}", time_str, side, pair, price, volume);
                    }
                }
            }
        }
    })
    .await
}

async fn ws_book(pair: &str) -> Result<CommandOutput> {
    let channel = format!("market:order-book-{}", pair);
    ws_connect_and_listen(&channel, |val| {
        let data = &val["result"]["data"]["data"];
        if let serde_json::Value::Array(asks) = &data["ask"] {
            if let Some(best) = asks.last() {
                let price = helpers::value_to_string(&best.get("price").unwrap_or(&serde_json::Value::Null));
                let amount = helpers::value_to_string(
                    &best.get("btc_volume")
                        .or_else(|| best.get("volume"))
                        .or_else(|| best.get("amount"))
                        .unwrap_or(&serde_json::Value::Null),
                );
                print!("\r\x1b[KAsk: {} @ {} | ", price, amount);
            }
        }
        if let serde_json::Value::Array(bids) = &data["bid"] {
            if let Some(best) = bids.first() {
                let price = helpers::value_to_string(&best.get("price").unwrap_or(&serde_json::Value::Null));
                let amount = helpers::value_to_string(
                    &best.get("btc_volume")
                        .or_else(|| best.get("volume"))
                        .or_else(|| best.get("amount"))
                        .unwrap_or(&serde_json::Value::Null),
                );
                println!("Bid: {} @ {}", price, amount);
            }
        }
    })
    .await
}

async fn ws_summary() -> Result<CommandOutput> {
    ws_connect_and_listen("market:summary-24h", |val| {
        let rows = &val["result"]["data"]["data"];
        if let serde_json::Value::Array(arr) = rows {
            for row in arr {
                if let serde_json::Value::Array(fields) = row {
                    if fields.len() >= 8 {
                        let pair = fields[0].as_str().unwrap_or("?");
                        let last = helpers::value_to_string(&fields[2]);
                        let high = helpers::value_to_string(&fields[4]);
                        let low = helpers::value_to_string(&fields[3]);
                        let price_24h = fields[5].as_f64().unwrap_or(0.0);
                        let last_f = fields[2].as_f64().unwrap_or(0.0);
                        let change = if price_24h > 0.0 {
                            format!("{:+.2}%", (last_f - price_24h) / price_24h * 100.0)
                        } else {
                            "0%".to_string()
                        };
                        println!("\x1b[K{:15}  last: {:>15}  high: {:>15}  low: {:>15}  change: {}", pair, last, high, low, change);
                    }
                }
            }
        }
    })
    .await
}

async fn ws_orders() -> Result<CommandOutput> {
    eprintln!("Private WebSocket requires authentication.");
    eprintln!("Use Indodax auth set first, then connect to private channel.");
    eprintln!("Private WebSocket URL: {}", PRIVATE_WS_URL);
    eprintln!("Generate token via POST /api/private_ws/v1/generate_token");

    Ok(CommandOutput::json(serde_json::json!({
        "status": "info",
        "message": "Private WebSocket connection requires manual token generation. See docs for /api/private_ws/v1/generate_token"
    })))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_websocket_command_variants() {
        let _cmd1 = WebSocketCommand::Ticker { pair: "btc_idr".into() };
        let _cmd2 = WebSocketCommand::Trades { pair: "eth_idr".into() };
        let _cmd3 = WebSocketCommand::Book { pair: "btc_idr".into() };
        let _cmd4 = WebSocketCommand::Summary;
        let _cmd5 = WebSocketCommand::Orders;
    }

    #[test]
    fn test_websocket_command_ticker() {
        let cmd = WebSocketCommand::Ticker { pair: "xrp_idr".into() };
        match cmd {
            WebSocketCommand::Ticker { pair } => {
                assert_eq!(pair, "xrp_idr");
            }
            _ => panic!("Expected Ticker command"),
        }
    }

    #[test]
    fn test_websocket_command_trades() {
        let cmd = WebSocketCommand::Trades { pair: "doge_idr".into() };
        match cmd {
            WebSocketCommand::Trades { pair } => {
                assert_eq!(pair, "doge_idr");
            }
            _ => panic!("Expected Trades command"),
        }
    }

    #[test]
    fn test_websocket_command_book() {
        let cmd = WebSocketCommand::Book { pair: "eth_idr".into() };
        match cmd {
            WebSocketCommand::Book { pair } => {
                assert_eq!(pair, "eth_idr");
            }
            _ => panic!("Expected Book command"),
        }
    }

    #[test]
    fn test_websocket_command_summary() {
        let cmd = WebSocketCommand::Summary;
        match cmd {
            WebSocketCommand::Summary => (),
            _ => panic!("Expected Summary command"),
        }
    }

    #[test]
    fn test_websocket_command_orders() {
        let cmd = WebSocketCommand::Orders;
        match cmd {
            WebSocketCommand::Orders => (),
            _ => panic!("Expected Orders command"),
        }
    }

    #[test]
    fn test_format_ws_price_u64() {
        let val = serde_json::json!(123456);
        let result = format_ws_price(&val);
        assert!(result.contains("123456"));
    }

    #[test]
    fn test_format_ws_price_f64() {
        let val = serde_json::json!(123.456);
        let result = format_ws_price(&val);
        assert!(result.contains("123") || result.contains("123.456"));
    }

    #[test]
    fn test_format_ws_price_str() {
        let val = serde_json::json!("789");
        let result = format_ws_price(&val);
        assert!(result.contains("789") || result == "\"789\"");
    }

    #[test]
    fn test_format_ws_price_null() {
        let val = serde_json::json!(null);
        let result = format_ws_price(&val);
        // Should return empty string or "null" string representation
        assert!(result.is_empty() || result == "0" || result.contains("null"));
    }

    #[test]
    fn test_public_ws_url() {
        assert!(PUBLIC_WS_URL.contains("ws3.indodax.com"));
    }

    #[test]
    fn test_private_ws_url() {
        assert!(PRIVATE_WS_URL.contains("pws.indodax.com"));
    }

    #[test]
    fn test_ws_token_not_empty() {
        assert!(!WS_TOKEN.is_empty());
    }
}
