use crate::client::IndodaxClient;
use crate::commands::helpers;
use crate::output::{CommandOutput, OutputFormat};
use anyhow::Result;
use futures_util::{SinkExt, StreamExt};
use indicatif::ProgressBar;
use std::io::IsTerminal;
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::Message;
use tracing;

const PUBLIC_WS_URL: &str = "wss://ws3.indodax.com/ws/";
const PRIVATE_WS_URL: &str = "wss://pws.indodax.com/ws/?cf_ws_frame_ping_pong=true";
const PUBLIC_WS_TOKEN_URL: &str = "https://indodax.com/api/ws/v1/generate_token";

async fn fetch_public_ws_token(client: &reqwest::Client) -> Result<String> {
    let resp = client.get(PUBLIC_WS_TOKEN_URL).send().await
        .map_err(|e| anyhow::anyhow!("Failed to fetch WebSocket token: {}", e))?;
    let text = resp.text().await
        .map_err(|e| anyhow::anyhow!("Failed to read token response: {}", e))?;
    let val: serde_json::Value = serde_json::from_str(&text)
        .map_err(|e| anyhow::anyhow!("Invalid token response: {}", e))?;
    val.get("token").and_then(|t| t.as_str()).map(|t| t.to_string())
        .or_else(|| val.get("data").and_then(|d| d.get("token")).and_then(|t| t.as_str()).map(|t| t.to_string()))
        .ok_or_else(|| anyhow::anyhow!("No token in response: {}", text))
}

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
    client: &IndodaxClient,
    cmd: &WebSocketCommand,
    output_format: OutputFormat,
) -> Result<CommandOutput> {
    match cmd {
        WebSocketCommand::Ticker { pair } => {
            let pair = helpers::normalize_pair(pair);
            ws_ticker(client, &pair, output_format).await
        }
        WebSocketCommand::Trades { pair } => {
            let pair = helpers::normalize_pair(pair);
            ws_trades(client, &pair, output_format).await
        }
        WebSocketCommand::Book { pair } => {
            let pair = helpers::normalize_pair(pair);
            ws_book(client, &pair, output_format).await
        }
        WebSocketCommand::Summary => ws_summary(client, output_format).await,
        WebSocketCommand::Orders => ws_orders(client, output_format).await,
    }
}

async fn ws_connect_and_listen(
    ws_url: &str,
    token: &str,
    channel: &str,
    handler: impl Fn(serde_json::Value),
    output_format: OutputFormat,
) -> Result<CommandOutput> {
    let spinner = if output_format == OutputFormat::Json {
        eprintln!(
            "{}",
            serde_json::json!({"event": "connecting", "url": ws_url})
        );
        None
    } else {
        let pb = ProgressBar::new_spinner();
        pb.set_message("Connecting to Indodax WebSocket...");
        pb.enable_steady_tick(std::time::Duration::from_millis(100));
        Some(pb)
    };
    let (mut ws_stream, _) = connect_async(ws_url).await?;
    if let Some(ref pb) = spinner {
        pb.set_message("Connected. Authenticating...");
    } else {
        eprintln!(
            "{}",
            serde_json::json!({"event": "connected", "status": "authenticating"})
        );
    }

    let auth_msg = serde_json::json!({
        "params": { "token": token },
        "id": 1
    });
    ws_stream
        .send(Message::Text(auth_msg.to_string()))
        .await?;

    let mut authed = false;

    loop {
        tokio::select! {
            _ = tokio::signal::ctrl_c() => {
                if let Some(ref pb) = spinner {
                    pb.finish_and_clear();
                    eprintln!("Interrupted by user. Closing connection...");
                } else {
                    eprintln!("{}", serde_json::json!({"event": "interrupted", "reason": "user_ctrl_c"}));
                }
                let _ = ws_stream.send(Message::Close(None)).await;
                break;
            }
            msg = ws_stream.next() => {
                let msg = match msg {
                    Some(m) => m,
                    None => break,
                };

                match msg {
                    Ok(Message::Text(text)) => {
                        let val = match serde_json::from_str::<serde_json::Value>(&text) {
                            Ok(v) => v,
                            Err(e) => {
                                tracing::warn!("WebSocket JSON parse error: {} (text: {})", e, text);
                                continue;
                            }
                        };

                        if !authed {
                            if val.get("id").and_then(|v| v.as_i64()) == Some(1)
                                && val.get("result").is_some()
                            {
                                authed = true;
                                if let Some(ref pb) = spinner {
                                    pb.finish_and_clear();
                                    eprintln!("Authenticated. Subscribing to channel: {}", channel);
                                    eprintln!();
                                } else {
                                    eprintln!("{}", serde_json::json!({"event": "authenticated", "channel": channel}));
                                }
                                let sub_msg = serde_json::json!({
                                    "method": 1,
                                    "params": { "channel": channel },
                                    "id": 2
                                });
                                ws_stream
                                    .send(Message::Text(sub_msg.to_string()))
                                    .await?;
                            }
                            continue;
                        }

                        if val.get("id").and_then(|v| v.as_i64()) == Some(2) {
                            // subscription confirmation, skip
                        } else if val.get("result").is_some() {
                            handler(val);
                        }
                    }
                    Ok(Message::Ping(_)) => {
                        let _ = ws_stream.send(Message::Pong(vec![])).await;
                    }
                    Ok(Message::Close(_)) => {
                        if let Some(ref pb) = spinner {
                            pb.finish_and_clear();
                            eprintln!("Connection closed by server.");
                        } else {
                            eprintln!("{}", serde_json::json!({"event": "disconnected", "reason": "server_close"}));
                        }
                        break;
                    }
                    Err(e) => {
                        if let Some(ref pb) = spinner {
                            pb.finish_and_clear();
                            eprintln!("WebSocket error: {}", e);
                        } else {
                            eprintln!("{}", serde_json::json!({"event": "error", "message": e.to_string()}));
                        }
                        break;
                    }
                    _ => {}
                }
            }
        }
    }

    Ok(CommandOutput::json(serde_json::json!({"status": "disconnected"})))
}

fn format_ws_price(val: &serde_json::Value) -> Option<String> {
    val.as_u64()
        .or_else(|| val.as_f64().map(|f| f as u64))
        .or_else(|| val.as_str().and_then(|s| s.parse().ok()))
        .map(|v| format!("{}", v))
}

async fn ws_ticker(client: &IndodaxClient, pair: &str, output_format: OutputFormat) -> Result<CommandOutput> {
    let channel = format!("chart:tick-{}", pair);
    let token = fetch_public_ws_token(client.http_client()).await?;
    ws_connect_and_listen(PUBLIC_WS_URL, &token, &channel, |val| {
        let rows = &val["result"]["data"]["data"];
        if let serde_json::Value::Array(arr) = rows {
            for row in arr {
                if let serde_json::Value::Array(fields) = row {
                    if fields.len() >= 4 {
                        let ts = fields[0].as_u64().unwrap_or(0);
                        let price = format_ws_price(&fields[2]).unwrap_or_default();
                        let time_str = chrono::DateTime::from_timestamp(ts as i64, 0)
                            .map(|d| d.format("%H:%M:%S").to_string())
                            .unwrap_or_default();
                        if output_format == OutputFormat::Json {
                            println!("{}", serde_json::json!({
                                "event": "ticker", "pair": pair, "time": time_str, "price": price
                            }));
                        } else {
                            println!("[{}] {}  {}", time_str, pair, price);
                        }
                    }
                }
            }
        }
    }, output_format)
    .await
}

async fn ws_trades(client: &IndodaxClient, pair: &str, output_format: OutputFormat) -> Result<CommandOutput> {
    let channel = format!("market:trade-activity-{}", pair);
    let token = fetch_public_ws_token(client.http_client()).await?;
    ws_connect_and_listen(PUBLIC_WS_URL, &token, &channel, |val| {
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
                        if output_format == OutputFormat::Json {
                            println!("{}", serde_json::json!({
                                "event": "trade", "pair": pair, "time": time_str,
                                "side": side, "price": price, "volume": volume
                            }));
                        } else {
                            println!("[{}] {} {} @ {} vol: {}", time_str, side, pair, price, volume);
                        }
                    }
                }
            }
        }
    }, output_format)
    .await
}

async fn ws_book(client: &IndodaxClient, pair: &str, output_format: OutputFormat) -> Result<CommandOutput> {
    let channel = format!("market:order-book-{}", pair);
    let token = fetch_public_ws_token(client.http_client()).await?;
    ws_connect_and_listen(PUBLIC_WS_URL, &token, &channel, |val| {
        let data = &val["result"]["data"]["data"];
        let ask_price = data["ask"].as_array().and_then(|asks| {
            asks.last().and_then(|best| {
                let p = helpers::value_to_string(best.get("price").unwrap_or(&serde_json::Value::Null));
                let a = helpers::value_to_string(
                    best.get("btc_volume")
                        .or_else(|| best.get("volume"))
                        .or_else(|| best.get("amount"))
                        .unwrap_or(&serde_json::Value::Null),
                );
                Some((p, a))
            })
        });
        let bid_price = data["bid"].as_array().and_then(|bids| {
            bids.first().and_then(|best| {
                let p = helpers::value_to_string(best.get("price").unwrap_or(&serde_json::Value::Null));
                let a = helpers::value_to_string(
                    best.get("btc_volume")
                        .or_else(|| best.get("volume"))
                        .or_else(|| best.get("amount"))
                        .unwrap_or(&serde_json::Value::Null),
                );
                Some((p, a))
            })
        });
        if output_format == OutputFormat::Json {
            println!("{}", serde_json::json!({
                "event": "orderbook", "pair": pair,
                "ask": ask_price.map(|(p, a)| serde_json::json!({"price": p, "amount": a})),
                "bid": bid_price.map(|(p, a)| serde_json::json!({"price": p, "amount": a})),
            }));
        } else if std::io::stdout().is_terminal() {
            if let Some((price, amount)) = ask_price {
                print!("\r\x1b[KAsk: {} @ {} | ", price, amount);
            }
            if let Some((price, amount)) = bid_price {
                println!("Bid: {} @ {}", price, amount);
            }
        } else {
            if let Some((price, amount)) = ask_price {
                println!("Ask: {} @ {}", price, amount);
            }
            if let Some((price, amount)) = bid_price {
                println!("Bid: {} @ {}", price, amount);
            }
        }
    }, output_format)
    .await
}

async fn ws_summary(client: &IndodaxClient, output_format: OutputFormat) -> Result<CommandOutput> {
    let token = fetch_public_ws_token(client.http_client()).await?;
    ws_connect_and_listen(PUBLIC_WS_URL, &token, "market:summary-24h", |val| {
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
                        if output_format == OutputFormat::Json {
                            println!("{}", serde_json::json!({
                                "event": "summary", "pair": pair, "last": last,
                                "high": high, "low": low, "change": change
                            }));
                        } else if std::io::stdout().is_terminal() {
                            println!("\x1b[K{:15}  last: {:>15}  high: {:>15}  low: {:>15}  change: {}", pair, last, high, low, change);
                        } else {
                            println!("{:15}  last: {:>15}  high: {:>15}  low: {:>15}  change: {}", pair, last, high, low, change);
                        }
                    }
                }
            }
        }
    }, output_format)
    .await
}

async fn ws_orders(client: &IndodaxClient, output_format: OutputFormat) -> Result<CommandOutput> {
    if client.signer().is_none() {
        return Err(anyhow::anyhow!(
            "Private WebSocket requires API credentials. Use 'indodax auth set' or set INDODAX_API_KEY and INDODAX_API_SECRET environment variables."
        ));
    }

    eprintln!("Generating WebSocket token...");
    let token = client.generate_ws_token().await.map_err(|e| {
        anyhow::anyhow!("WebSocket token generation failed: {}. Check that your API credentials are valid and have the correct permissions.", e)
    })?;
    eprintln!("Token generated. Connecting to private WebSocket...");

    let channel = "private:orders";
    ws_connect_and_listen(PRIVATE_WS_URL, &token, channel, |val| {
        let data = &val["result"]["data"];
        if let Some(order_id) = data.get("order_id").and_then(|v| v.as_u64()) {
            let pair = data.get("pair").and_then(|v| v.as_str()).unwrap_or("?");
            let side = data.get("side").and_then(|v| v.as_str()).unwrap_or("?");
            let status = data.get("status").and_then(|v| v.as_str()).unwrap_or("?");
            let price = helpers::value_to_string(data.get("price").unwrap_or(&serde_json::Value::Null));
            let amount = helpers::value_to_string(data.get("amount").unwrap_or(&serde_json::Value::Null));
            if output_format == OutputFormat::Json {
                println!("{}", serde_json::json!({
                    "event": "order_update", "order_id": order_id, "pair": pair,
                    "side": side, "status": status, "price": price, "amount": amount
                }));
            } else {
                println!("ID={} Pair={} Side={} Status={} Price={} Amount={}",
                    order_id, pair, side, status, price, amount);
            }
        } else {
            if output_format == OutputFormat::Json {
                println!("{}", serde_json::json!({"event": "order_update_raw", "data": &val["result"]}));
            } else {
                println!("{}", serde_json::to_string_pretty(&val).unwrap_or_default());
            }
        }
    }, output_format)
    .await
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
            _ => assert!(false, "Expected Ticker command, got {:?}", cmd),
        }
    }

    #[test]
    fn test_websocket_command_trades() {
        let cmd = WebSocketCommand::Trades { pair: "doge_idr".into() };
        match cmd {
            WebSocketCommand::Trades { pair } => {
                assert_eq!(pair, "doge_idr");
            }
            _ => assert!(false, "Expected Trades command, got {:?}", cmd),
        }
    }

    #[test]
    fn test_websocket_command_book() {
        let cmd = WebSocketCommand::Book { pair: "eth_idr".into() };
        match cmd {
            WebSocketCommand::Book { pair } => {
                assert_eq!(pair, "eth_idr");
            }
            _ => assert!(false, "Expected Book command, got {:?}", cmd),
        }
    }

    #[test]
    fn test_websocket_command_summary() {
        let cmd = WebSocketCommand::Summary;
        match cmd {
            WebSocketCommand::Summary => (),
            _ => assert!(false, "Expected Summary command, got {:?}", cmd),
        }
    }

    #[test]
    fn test_websocket_command_orders() {
        let cmd = WebSocketCommand::Orders;
        match cmd {
            WebSocketCommand::Orders => (),
            _ => assert!(false, "Expected Orders command, got {:?}", cmd),
        }
    }

    #[test]
    fn test_format_ws_price_u64() {
        let val = serde_json::json!(123456);
        assert_eq!(format_ws_price(&val).as_deref(), Some("123456"));
    }

    #[test]
    fn test_format_ws_price_f64() {
        let val = serde_json::json!(123.456);
        let result = format_ws_price(&val);
        assert!(result.is_some());
    }

    #[test]
    fn test_format_ws_price_str() {
        let val = serde_json::json!("789");
        assert_eq!(format_ws_price(&val).as_deref(), Some("789"));
    }

    #[test]
    fn test_format_ws_price_null() {
        let val = serde_json::json!(null);
        assert!(format_ws_price(&val).is_none());
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
    fn test_public_ws_token_url() {
        assert!(PUBLIC_WS_TOKEN_URL.contains("indodax.com"));
    }
}
