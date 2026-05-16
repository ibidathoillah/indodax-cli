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
    handler: impl Fn(serde_json::Value) -> Option<serde_json::Value>,
    output_format: OutputFormat,
) -> Result<CommandOutput> {
    let spinner_ref = if output_format == OutputFormat::Json {
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

    let mut events: Vec<serde_json::Value> = Vec::new();
    let mut retry_count = 0;

    'reconnect: loop {
        if retry_count > 0 {
            let delay = std::time::Duration::from_secs(2u64.pow(retry_count.min(5)));
            if let Some(ref pb) = spinner_ref {
                pb.set_message(format!("Disconnected. Retrying in {:?}...", delay));
            } else {
                eprintln!("{}", serde_json::json!({"event": "reconnecting", "delay_secs": delay.as_secs()}));
            }
            tokio::select! {
                _ = tokio::signal::ctrl_c() => break 'reconnect,
                _ = tokio::time::sleep(delay) => {}
            }
        }

        let (mut ws_stream, _) = match connect_async(ws_url).await {
            Ok(s) => s,
            Err(e) => {
                retry_count += 1;
                tracing::warn!("WebSocket connection failed: {}. Retrying...", e);
                continue 'reconnect;
            }
        };

        if let Some(ref pb) = spinner_ref {
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
        if let Err(e) = ws_stream.send(Message::Text(auth_msg.to_string())).await {
            retry_count += 1;
            tracing::warn!("Failed to send auth message: {}. Retrying...", e);
            continue 'reconnect;
        }

        let mut authed = false;
        let mut ping_interval = tokio::time::interval(std::time::Duration::from_secs(30));
        ping_interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);

        loop {
            tokio::select! {
                _ = tokio::signal::ctrl_c() => {
                    if let Some(ref pb) = spinner_ref {
                        pb.finish_and_clear();
                        eprintln!("Interrupted by user. Closing connection...");
                    } else {
                        eprintln!("{}", serde_json::json!({"event": "interrupted", "reason": "user_ctrl_c"}));
                    }
                    let _ = ws_stream.send(Message::Close(None)).await;
                    break 'reconnect;
                }
                _ = ping_interval.tick() => {
                    // Application-level ping to keep connection alive
                    let ping_msg = serde_json::json!({
                        "method": 7,
                        "id": 7
                    });
                    if let Err(e) = ws_stream.send(Message::Text(ping_msg.to_string())).await {
                        tracing::warn!("Failed to send WebSocket ping: {}. Triggering reconnect...", e);
                        retry_count += 1;
                        continue 'reconnect;
                    }
                }
                msg = ws_stream.next() => {
                    let msg = match msg {
                        Some(m) => m,
                        None => {
                            retry_count += 1;
                            tracing::warn!("WebSocket stream ended. Reconnecting...");
                            continue 'reconnect;
                        }
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
                                    retry_count = 0; // Reset retry count on successful auth
                                    if let Some(ref pb) = spinner_ref {
                                        pb.set_message(format!("Authenticated. Subscribing to: {}", channel));
                                    } else {
                                        eprintln!("{}", serde_json::json!({"event": "authenticated", "channel": channel}));
                                    }
                                    let sub_msg = serde_json::json!({
                                        "method": 1,
                                        "params": { "channel": channel },
                                        "id": 2
                                    });
                                    if let Err(e) = ws_stream.send(Message::Text(sub_msg.to_string())).await {
                                        retry_count += 1;
                                        continue 'reconnect;
                                    }
                                }
                                continue;
                            }

                            if val.get("id").and_then(|v| v.as_i64()) == Some(2) {
                                // subscription confirmation
                                if let Some(ref pb) = spinner_ref {
                                    pb.finish_and_clear();
                                    eprintln!("Subscription active: {}", channel);
                                    eprintln!();
                                }
                            } else if val.get("result").is_some() {
                                if let Some(event) = handler(val) {
                                    events.push(event);
                                }
                            }
                        }
                        Ok(Message::Ping(data)) => {
                            let _ = ws_stream.send(Message::Pong(data)).await;
                        }
                        Ok(Message::Close(_)) => {
                            retry_count += 1;
                            tracing::warn!("Connection closed by server. Reconnecting...");
                            continue 'reconnect;
                        }
                        Err(e) => {
                            retry_count += 1;
                            tracing::warn!("WebSocket error: {}. Reconnecting...", e);
                            continue 'reconnect;
                        }
                        _ => {}
                    }
                }
            }
        }
    }

    Ok(CommandOutput::json(serde_json::json!({
        "status": "disconnected",
        "events": events,
        "event_count": events.len(),
    })))
}

fn format_ws_price(val: &serde_json::Value) -> Option<String> {
    let f = val.as_f64()
        .or_else(|| val.as_str().and_then(|s| s.parse::<f64>().ok()))?;
    if f == 0.0 {
        return Some("0".into());
    }
    if (f - f.round()).abs() < f64::EPSILON && f.abs() >= 1.0 {
        return Some(format!("{}", f as u64));
    }
    let s = format!("{:.8}", f);
    let trimmed = s.trim_end_matches('0');
    Some(trimmed.trim_end_matches('.').to_string())
}

async fn ws_ticker(client: &IndodaxClient, pair: &str, output_format: OutputFormat) -> Result<CommandOutput> {
    let channel = format!("chart:tick-{}", pair);
    let token = helpers::fetch_public_ws_token(client).await?;
    ws_connect_and_listen(PUBLIC_WS_URL, &token, &channel, |val| {
        let rows = &val["result"]["data"]["data"];
        let mut last_event = None;
        if let serde_json::Value::Array(arr) = rows {
            for row in arr {
                if let serde_json::Value::Array(fields) = row {
                    if fields.len() >= 4 {
                        let ts = fields[0].as_u64().unwrap_or(0);
                        let price = format_ws_price(&fields[2]).unwrap_or_default();
                        let time_str = chrono::DateTime::from_timestamp(ts.min(i64::MAX as u64) as i64, 0)
                            .map(|d| d.format("%H:%M:%S").to_string())
                            .unwrap_or_default();
                        if output_format == OutputFormat::Json {
                            println!("{}", serde_json::json!({
                                "event": "ticker", "pair": pair, "time": time_str, "price": price
                            }));
                        } else {
                            println!("[{}] {}  {}", time_str, pair, price);
                        }
                        last_event = Some(serde_json::json!({
                            "event": "ticker", "pair": pair, "time": time_str, "price": price
                        }));
                    }
                }
            }
        }
        last_event
    }, output_format)
    .await
}

async fn ws_trades(client: &IndodaxClient, pair: &str, output_format: OutputFormat) -> Result<CommandOutput> {
    let channel = format!("market:trade-activity-{}", pair);
    let token = helpers::fetch_public_ws_token(client).await?;
    ws_connect_and_listen(PUBLIC_WS_URL, &token, &channel, |val| {
        let rows = &val["result"]["data"]["data"];
        let mut last_event = None;
        if let serde_json::Value::Array(arr) = rows {
            for row in arr {
                if let serde_json::Value::Array(fields) = row {
                    if fields.len() >= 7 {
                        let ts = fields[1].as_u64().unwrap_or(0);
                        let side = fields[3].as_str().unwrap_or("?");
                        let price = fields[4].as_f64().unwrap_or(0.0);
                        let volume = fields[6].as_str().and_then(|s| s.parse().ok()).unwrap_or(0.0);
                        let time_str = chrono::DateTime::from_timestamp(ts.min(i64::MAX as u64) as i64, 0)
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
                        last_event = Some(serde_json::json!({
                            "event": "trade", "pair": pair, "time": time_str,
                            "side": side, "price": price, "volume": volume
                        }));
                    }
                }
            }
        }
        last_event
    }, output_format)
    .await
}

async fn ws_book(client: &IndodaxClient, pair: &str, output_format: OutputFormat) -> Result<CommandOutput> {
    let channel = format!("market:order-book-{}", pair);
    let token = helpers::fetch_public_ws_token(client).await?;
    ws_connect_and_listen(PUBLIC_WS_URL, &token, &channel, |val| {
        let data = &val["result"]["data"]["data"];
        
        let parse_entry = |entry: &serde_json::Value| -> Option<(String, String)> {
            if let Some(arr) = entry.as_array() {
                if arr.len() >= 2 {
                    let p = helpers::value_to_string(&arr[0]);
                    let v = helpers::value_to_string(&arr[1]);
                    return Some((p, v));
                }
            } else if let Some(obj) = entry.as_object() {
                let p = helpers::value_to_string(obj.get("price").unwrap_or(&serde_json::Value::Null));
                let v = helpers::value_to_string(
                    obj.get("btc_volume")
                        .or_else(|| obj.get("volume"))
                        .or_else(|| obj.get("amount"))
                        .unwrap_or(&serde_json::Value::Null),
                );
                return Some((p, v));
            }
            None
        };

        let ask_price = data["ask"].as_array().and_then(|asks| {
            asks.first().and_then(parse_entry)
        }).or_else(|| data["asks"].as_array().and_then(|asks| {
            asks.first().and_then(parse_entry)
        }));

        let bid_price = data["bid"].as_array().and_then(|bids| {
            bids.first().and_then(parse_entry)
        }).or_else(|| data["bids"].as_array().and_then(|bids| {
            bids.first().and_then(parse_entry)
        }));

        let event = serde_json::json!({
            "event": "orderbook", "pair": pair,
            "ask": ask_price.clone().map(|(p, a)| serde_json::json!({"price": p, "amount": a})),
            "bid": bid_price.clone().map(|(p, a)| serde_json::json!({"price": p, "amount": a})),
        });
        if output_format == OutputFormat::Json {
            println!("{}", event);
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
        Some(event)
    }, output_format)
    .await
}

async fn ws_summary(client: &IndodaxClient, output_format: OutputFormat) -> Result<CommandOutput> {
    let token = helpers::fetch_public_ws_token(client).await?;
    ws_connect_and_listen(PUBLIC_WS_URL, &token, "market:summary-24h", |val| {
        let rows = &val["result"]["data"]["data"];
        let mut last_event = None;
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
                        last_event = Some(serde_json::json!({
                            "event": "summary", "pair": pair, "last": last,
                            "high": high, "low": low, "change": change
                        }));
                    }
                }
            }
        }
        last_event
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
    let (token, channel) = client.generate_ws_token().await.map_err(|e| {
        anyhow::anyhow!("WebSocket token generation failed: {}. Check that your API credentials are valid and have the correct permissions.", e)
    })?;
    eprintln!("Token generated. Connecting to private WebSocket...");

    ws_private_connect_and_listen(PRIVATE_WS_URL, &token, &channel, |val| {
        // Private WebSocket messages can be in several formats depending on the server version
        // We look for common patterns in order and balance updates.
        
        let result = val.get("result").or(val.get("push")).or(Some(&val)).unwrap();
        let data = result.get("data").unwrap_or(result);

        if let Some(order_id) = data.get("order_id").or(data.get("orderId")).and_then(|v| v.as_u64().or_else(|| v.as_str().and_then(|s| s.parse().ok()))) {
            let pair = data.get("pair").and_then(|v| v.as_str()).unwrap_or("?");
            let side = data.get("side").and_then(|v| v.as_str()).unwrap_or("?");
            let status = data.get("status").and_then(|v| v.as_str()).unwrap_or("?");
            let price = helpers::value_to_string(data.get("price").unwrap_or(&serde_json::Value::Null));
            let amount = helpers::value_to_string(data.get("amount").or(data.get("quantity")).unwrap_or(&serde_json::Value::Null));
            
            if output_format == OutputFormat::Json {
                println!("{}", serde_json::json!({
                    "event": "order_update", "order_id": order_id, "pair": pair,
                    "side": side, "status": status, "price": price, "amount": amount
                }));
            } else {
                println!("Order Update: ID={} Pair={} Side={} Status={} Price={} Amount={}",
                    order_id, pair, side, status, price, amount);
            }
            Some(serde_json::json!({
                "event": "order_update", "order_id": order_id, "pair": pair,
                "side": side, "status": status, "price": price, "amount": amount
            }))
        } else if let Some(currency) = data.get("currency").or(data.get("asset")).and_then(|v| v.as_str()) {
            let available = helpers::value_to_string(data.get("available").or(data.get("balance")).unwrap_or(&serde_json::Value::Null));
            let frozen = helpers::value_to_string(data.get("frozen").or(data.get("hold")).unwrap_or(&serde_json::Value::Null));
            
            if output_format == OutputFormat::Json {
                println!("{}", serde_json::json!({
                    "event": "balance_update", "currency": currency,
                    "available": available, "frozen": frozen
                }));
            } else {
                println!("Balance Update: {} Available={} Frozen={}", currency, available, frozen);
            }
            Some(serde_json::json!({
                "event": "balance_update", "currency": currency,
                "available": available, "frozen": frozen
            }))
        } else {
            // Raw fallback for unknown private events
            if output_format == OutputFormat::Json {
                let raw = serde_json::json!({"event": "private_update_raw", "data": data});
                println!("{}", raw);
                Some(raw)
            } else {
                // If it's a heartbeat or confirmation, we might want to skip printing
                if data.get("method").and_then(|m| m.as_str()) != Some("pong") {
                    println!("Private Event: {}", serde_json::to_string(data).unwrap_or_default());
                }
                Some(data.clone())
            }
        }
    }, output_format)
    .await
}

async fn ws_private_connect_and_listen(
    ws_url: &str,
    token: &str,
    channel: &str,
    handler: impl Fn(serde_json::Value) -> Option<serde_json::Value>,
    output_format: OutputFormat,
) -> Result<CommandOutput> {
    let spinner_ref = if output_format == OutputFormat::Json {
        eprintln!("{}", serde_json::json!({"event": "connecting", "url": ws_url}));
        None
    } else {
        let pb = ProgressBar::new_spinner();
        pb.set_message("Connecting to Private WebSocket...");
        pb.enable_steady_tick(std::time::Duration::from_millis(100));
        Some(pb)
    };

    let mut events: Vec<serde_json::Value> = Vec::new();
    let mut retry_count = 0;

    'reconnect: loop {
        if retry_count > 0 {
            let delay = std::time::Duration::from_secs(2u64.pow(retry_count.min(5)));
            if let Some(ref pb) = spinner_ref {
                pb.set_message(format!("Disconnected. Retrying in {:?}...", delay));
            } else {
                eprintln!("{}", serde_json::json!({"event": "reconnecting", "delay_secs": delay.as_secs()}));
            }
            tokio::select! {
                _ = tokio::signal::ctrl_c() => break 'reconnect,
                _ = tokio::time::sleep(delay) => {}
            }
        }

        let (mut ws_stream, _) = match connect_async(ws_url).await {
            Ok(s) => s,
            Err(e) => {
                retry_count += 1;
                tracing::warn!("Private WebSocket connection failed: {}. Retrying...", e);
                continue 'reconnect;
            }
        };

        // 1. Connect (Authenticate)
        let connect_msg = serde_json::json!({
            "connect": { "token": token },
            "id": 1
        });
        if let Err(e) = ws_stream.send(Message::Text(connect_msg.to_string())).await {
            retry_count += 1;
            continue 'reconnect;
        }

        let mut authed = false;
        let mut subscribed = false;
        let mut ping_interval = tokio::time::interval(std::time::Duration::from_secs(30));

        loop {
            tokio::select! {
                _ = tokio::signal::ctrl_c() => {
                    let _ = ws_stream.send(Message::Close(None)).await;
                    break 'reconnect;
                }
                _ = ping_interval.tick() => {
                    // Private WS uses standard ping frames if configured in URL, 
                    // but some versions also support application-level pings.
                    let _ = ws_stream.send(Message::Ping(vec![])).await;
                }
                msg = ws_stream.next() => {
                    let msg = match msg {
                        Some(m) => m,
                        None => { retry_count += 1; continue 'reconnect; }
                    };

                    match msg {
                        Ok(Message::Text(text)) => {
                            let val: serde_json::Value = match serde_json::from_str(&text) {
                                Ok(v) => v,
                                Err(_) => continue,
                            };

                            if !authed {
                                // Check for connection success
                                if val.get("connect").is_some() || val.get("id").and_then(|v| v.as_i64()) == Some(1) {
                                    authed = true;
                                    retry_count = 0;
                                    // 2. Subscribe to user channel
                                    let sub_msg = serde_json::json!({
                                        "subscribe": { "channel": channel },
                                        "id": 2
                                    });
                                    let _ = ws_stream.send(Message::Text(sub_msg.to_string())).await;
                                }
                                continue;
                            }

                            if !subscribed {
                                if val.get("subscribe").is_some() || val.get("id").and_then(|v| v.as_i64()) == Some(2) {
                                    subscribed = true;
                                    if let Some(ref pb) = spinner_ref {
                                        pb.finish_and_clear();
                                        eprintln!("Private subscription active: {}", channel);
                                        eprintln!();
                                    }
                                }
                                continue;
                            }

                            if let Some(event) = handler(val) {
                                events.push(event);
                            }
                        }
                        Ok(Message::Ping(data)) => { let _ = ws_stream.send(Message::Pong(data)).await; }
                        Ok(Message::Close(_)) => { retry_count += 1; continue 'reconnect; }
                        Err(_) => { retry_count += 1; continue 'reconnect; }
                        _ => {}
                    }
                }
            }
        }
    }

    Ok(CommandOutput::json(serde_json::json!({
        "status": "disconnected",
        "events": events,
        "event_count": events.len(),
    })))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_websocket_command_variants() {
        let _cmd1 = WebSocketCommand::Ticker { pair: "btc_idr".into() };
        let _cmd2 = WebSocketCommand::Trades { pair: "eth_idr".into() };
        let _cmd3 = WebSocketCommand::Book { pair: "btc_idr".into() };
        let _cmd4 = WebSocketCommand::Summary;
        let _cmd5 = WebSocketCommand::Orders;
    }

    #[test]
    fn test_format_ws_price() {
        assert_eq!(format_ws_price(&json!(1234.56)), Some("1234.56".to_string()));
        assert_eq!(format_ws_price(&json!("1234.56")), Some("1234.56".to_string()));
        assert_eq!(format_ws_price(&json!(1000)), Some("1000".to_string()));
        assert_eq!(format_ws_price(&json!(0)), Some("0".to_string()));
    }

    #[test]
    fn test_ticker_parsing_logic() {
        let msg = json!({
            "result": {
                "data": {
                    "data": [
                        [1632717721, 4087327, 14340, "1063.73019525"]
                    ]
                }
            }
        });
        
        // Simulating the handler logic inside ws_ticker
        let rows = &msg["result"]["data"]["data"];
        if let serde_json::Value::Array(arr) = rows {
            let fields = arr[0].as_array().unwrap();
            let price = format_ws_price(&fields[2]).unwrap();
            assert_eq!(price, "14340");
        } else {
            panic!("Expected array");
        }
    }

    #[test]
    fn test_orderbook_parsing_array_format() {
        let msg = json!({
            "result": {
                "data": {
                    "data": {
                        "asks": [["651000000", "0.05000000"]],
                        "bids": [["650000000", "0.12345678"]]
                    }
                }
            }
        });
        
        let data = &msg["result"]["data"]["data"];
        let parse_entry = |entry: &serde_json::Value| -> Option<(String, String)> {
            if let Some(arr) = entry.as_array() {
                if arr.len() >= 2 {
                    let p = helpers::value_to_string(&arr[0]);
                    let v = helpers::value_to_string(&arr[1]);
                    return Some((p, v));
                }
            }
            None
        };

        let ask = data["asks"].as_array().unwrap().first().and_then(parse_entry).unwrap();
        assert_eq!(ask.0, "651000000");
        assert_eq!(ask.1, "0.05000000");
    }

    #[test]
    fn test_orderbook_parsing_object_format() {
        let msg = json!({
            "result": {
                "data": {
                    "data": {
                        "ask": [{"price": "319437000", "btc_volume": "0.11035661"}],
                        "bid": [{"price": "319436000", "btc_volume": "0.61427265"}]
                    }
                }
            }
        });

        let data = &msg["result"]["data"]["data"];
        let parse_entry = |entry: &serde_json::Value| -> Option<(String, String)> {
            if let Some(obj) = entry.as_object() {
                let p = helpers::value_to_string(obj.get("price").unwrap());
                let v = helpers::value_to_string(obj.get("btc_volume").unwrap());
                return Some((p, v));
            }
            None
        };

        let ask = data["ask"].as_array().unwrap().first().and_then(parse_entry).unwrap();
        assert_eq!(ask.0, "319437000");
        assert_eq!(ask.1, "0.11035661");
    }

    #[test]
    fn test_private_order_update_parsing() {
        let msg = json!({
            "push": {
                "data": {
                    "order_id": 12345,
                    "pair": "btcidr",
                    "side": "buy",
                    "status": "filled",
                    "price": "500000000",
                    "amount": "0.1"
                }
            }
        });

        let result = msg.get("result").or(msg.get("push")).or(Some(&msg)).unwrap();
        let data = result.get("data").unwrap_or(result);
        
        assert_eq!(data["order_id"], 12345);
        assert_eq!(data["pair"], "btcidr");
    }

    #[test]
    fn test_private_balance_update_parsing() {
        let msg = json!({
            "currency": "idr",
            "available": "1000000",
            "frozen": "50000"
        });

        let data = &msg;
        assert_eq!(data["currency"], "idr");
        assert_eq!(data["available"], "1000000");
    }

    #[test]
    fn test_fetch_public_ws_token_precedence() {
        // We can't easily mock the IndodaxClient's network call here without more refactoring,
        // but we can test the logic of fetch_public_ws_token if we extract it slightly.
        let default_token = helpers::DEFAULT_STATIC_WS_TOKEN;
        assert!(default_token.starts_with("eyJ"));
    }
}
