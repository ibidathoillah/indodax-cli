use crate::client::IndodaxClient;
use crate::commands::helpers;
use crate::output::CommandOutput;
use anyhow::Result;
use serde_json::Value;

fn first_of<'a>(val: &'a Value, keys: &[&str]) -> &'a Value {
    for k in keys {
        let v = &val[k];
        if !v.is_null() {
            let s = helpers::value_to_string(v);
            if !s.is_empty() && s != "null" {
                return v;
            }
        }
    }
    &Value::Null
}

#[derive(Debug, clap::Subcommand)]
pub enum MarketCommand {
    #[command(name = "server-time", about = "Get server time")]
    ServerTime,

    #[command(name = "pairs", about = "List available trading pairs")]
    Pairs,

    #[command(name = "ticker", about = "Get ticker for a pair")]
    Ticker {
        #[arg(default_value = "btc_idr")]
        pair: String,
    },

    #[command(name = "ticker-all", about = "Get tickers for all pairs")]
    TickerAll,

    #[command(name = "summaries", about = "Get 24h and 7d summaries for all pairs")]
    Summaries,

    #[command(name = "orderbook", about = "Get order book for a pair")]
    Orderbook {
        #[arg(default_value = "btcidr")]
        pair: String,
    },

    #[command(name = "trades", about = "Get recent trades for a pair")]
    Trades {
        #[arg(default_value = "btcidr")]
        pair: String,
    },

    #[command(name = "ohlc", about = "Get OHLCV candle data")]
    Ohlc {
        #[arg(short, long, default_value = "BTCIDR")]
        symbol: String,
        #[arg(long, default_value = "60")]
        timeframe: String,
        #[arg(short, long)]
        from: Option<u64>,
        #[arg(long)]
        to: Option<u64>,
    },

    #[command(name = "price-increments", about = "Get price increments (tick sizes)")]
    PriceIncrements,
}

pub async fn execute(
    client: &IndodaxClient,
    cmd: &MarketCommand,
) -> Result<CommandOutput> {
    match cmd {
        MarketCommand::ServerTime => server_time(client).await,
        MarketCommand::Pairs => pairs(client).await,
        MarketCommand::Ticker { pair } => ticker(client, pair).await,
        MarketCommand::TickerAll => ticker_all(client).await,
        MarketCommand::Summaries => summaries(client).await,
        MarketCommand::Orderbook { pair } => orderbook(client, pair).await,
        MarketCommand::Trades { pair } => trades(client, pair).await,
        MarketCommand::Ohlc { symbol, timeframe, from, to } => {
            ohlc(client, symbol, timeframe, *from, *to).await
        }
        MarketCommand::PriceIncrements => price_increments(client).await,
    }
}

async fn server_time(client: &IndodaxClient) -> Result<CommandOutput> {
    let data: Value = client.public_get("/api/server_time").await?;
    let (headers, rows) = helpers::flatten_json_to_table(&data);
    Ok(CommandOutput::new(data, headers, rows))
}

async fn pairs(client: &IndodaxClient) -> Result<CommandOutput> {
    let data: Value = client.public_get("/api/pairs").await?;
    let pairs_info = helpers::extract_pairs(&data);
    let headers = vec!["Pair ID".into(), "Info".into()];
    let rows: Vec<Vec<String>> = pairs_info
        .into_iter()
        .map(|(id, info)| vec![id, info])
        .collect();
    Ok(CommandOutput::new(data, headers, rows))
}

async fn ticker(client: &IndodaxClient, pair: &str) -> Result<CommandOutput> {
    let data: Value = client.public_get(&format!("/api/ticker/{}", pair)).await?;
    let ticker = &data["ticker"];
    if ticker.is_object() {
        let (headers, rows) = helpers::flatten_json_to_table(ticker);
        Ok(CommandOutput::new(data, headers, rows))
    } else {
        let (headers, rows) = helpers::flatten_json_to_table(&data);
        Ok(CommandOutput::new(data, headers, rows))
    }
}

async fn ticker_all(client: &IndodaxClient) -> Result<CommandOutput> {
    let data: Value = client.public_get("/api/ticker_all").await?;
    let tickers = &data["tickers"];
    if tickers.is_object() {
        let headers = vec![
            "Pair".into(), "Last".into(), "High".into(), "Low".into(),
            "Buy".into(), "Sell".into(), "Vol (base)".into(), "Vol (quote)".into(),
        ];
        let mut rows: Vec<Vec<String>> = Vec::new();
        if let Value::Object(map) = tickers {
            for (key, val) in map {
                rows.push(vec![
                    key.clone(),
                    helpers::value_to_string(&val["last"]),
                    helpers::value_to_string(&val["high"]),
                    helpers::value_to_string(&val["low"]),
                    helpers::value_to_string(&val["buy"]),
                    helpers::value_to_string(&val["sell"]),
                    helpers::value_to_string(first_of(val, &["vol_btc", "vol_base"])),
                    helpers::value_to_string(first_of(val, &["vol_idr", "vol_traded"])),
                ]);
            }
        }
        rows.sort_by(|a, b| a[0].cmp(&b[0]));
        Ok(CommandOutput::new(data, headers, rows))
    } else {
        let (headers, rows) = helpers::flatten_json_to_table(&data);
        Ok(CommandOutput::new(data, headers, rows))
    }
}

async fn summaries(client: &IndodaxClient) -> Result<CommandOutput> {
    let data: Value = client.public_get("/api/summaries").await?;
    let summaries = &data["summaries"];
    if summaries.is_object() {
        let headers = vec![
            "Pair".into(), "Last".into(), "High".into(), "Low".into(),
            "Vol (base)".into(), "Vol (quote)".into(),
        ];
        let mut rows: Vec<Vec<String>> = Vec::new();
        if let Value::Object(map) = summaries {
            for (key, val) in map {
                rows.push(vec![
                    key.clone(),
                    helpers::value_to_string(&val["last"]),
                    helpers::value_to_string(&val["high"]),
                    helpers::value_to_string(&val["low"]),
                    helpers::value_to_string(first_of(val, &["vol_btc", "vol_base"])),
                    helpers::value_to_string(first_of(val, &["vol_idr", "vol_traded"])),
                ]);
            }
        }
        rows.sort_by(|a, b| a[0].cmp(&b[0]));
        Ok(CommandOutput::new(data, headers, rows))
    } else {
        let (headers, rows) = helpers::flatten_json_to_table(&data);
        Ok(CommandOutput::new(data, headers, rows))
    }
}

async fn orderbook(client: &IndodaxClient, pair: &str) -> Result<CommandOutput> {
    let data: Value = client.public_get(&format!("/api/depth/{}", pair)).await?;
    let headers = vec!["Side".into(), "Price".into(), "Amount".into()];
    let mut rows: Vec<Vec<String>> = Vec::new();
    let buys = &data["buy"];
    let sells = &data["sell"];
    if let Value::Array(arr) = buys {
        for entry in arr.iter().take(20) {
            if let Some(row_arr) = entry.as_array().filter(|a| a.len() >= 2) {
                rows.push(vec!["BUY".into(), helpers::value_to_string(&row_arr[0]), helpers::value_to_string(&row_arr[1])]);
            }
        }
    }
    if let Value::Array(arr) = sells {
        for entry in arr.iter().rev().take(20) {
            if let Some(row_arr) = entry.as_array().filter(|a| a.len() >= 2) {
                rows.push(vec!["SELL".into(), helpers::value_to_string(&row_arr[0]), helpers::value_to_string(&row_arr[1])]);
            }
        }
    }
    let level_count = rows.len() / 2;
    Ok(CommandOutput::new(data, headers, rows)
        .with_addendum(format!("Showing {} bid/ask levels", level_count)))
}

async fn trades(client: &IndodaxClient, pair: &str) -> Result<CommandOutput> {
    let data: Value = client.public_get(&format!("/api/trades/{}", pair)).await?;
    let headers = vec!["TID".into(), "Date".into(), "Price".into(), "Amount".into(), "Type".into()];
    let mut rows: Vec<Vec<String>> = Vec::new();
    if let Value::Array(arr) = &data {
        for trade in arr.iter().take(50) {
            let ts = trade["date"].as_str()
                .and_then(|s| s.parse::<u64>().ok())
                .or_else(|| trade["date"].as_u64())
                .unwrap_or(0);
            rows.push(vec![
                helpers::value_to_string(&trade["tid"]),
                helpers::format_timestamp(ts, false),
                helpers::value_to_string(&trade["price"]),
                helpers::value_to_string(&trade["amount"]),
                helpers::value_to_string(&trade["type"]),
            ]);
        }
    }
    Ok(CommandOutput::new(data, headers, rows))
}

async fn ohlc(
    client: &IndodaxClient,
    symbol: &str,
    timeframe: &str,
    from: Option<u64>,
    to: Option<u64>,
) -> Result<CommandOutput> {
    let now_secs = crate::auth::Signer::now_millis() / 1000;
    let from_val = from.map(|v| v.to_string()).unwrap_or_else(|| {
        (now_secs - 24 * 60 * 60).to_string()
    });
    let to_val = to.map(|v| v.to_string()).unwrap_or_else(|| {
        now_secs.to_string()
    });

    let data: Value = client.public_get_v2(
        "/tradingview/history_v2",
        &[
            ("symbol", symbol),
            ("tf", timeframe),
            ("from", &from_val),
            ("to", &to_val),
        ],
    ).await?;

    let headers = vec![
        "Time".into(), "Open".into(), "High".into(), "Low".into(),
        "Close".into(), "Volume".into(),
    ];
    let mut rows: Vec<Vec<String>> = Vec::new();

    if let Value::Object(ref map) = data {
        let times = map.get("t").and_then(|v| v.as_array());
        let opens = map.get("o").and_then(|v| v.as_array());
        let highs = map.get("h").and_then(|v| v.as_array());
        let lows = map.get("l").and_then(|v| v.as_array());
        let closes = map.get("c").and_then(|v| v.as_array());
        let volumes = map.get("v").and_then(|v| v.as_array());

        if let (Some(t), Some(o), Some(h), Some(l), Some(c), Some(vol)) =
            (times, opens, highs, lows, closes, volumes)
        {
            let len = t.len().min(o.len()).min(h.len()).min(l.len()).min(c.len()).min(vol.len());
            for i in 0..len {
                rows.push(vec![
                    helpers::format_timestamp(t[i].as_u64().unwrap_or(0), false),
                    helpers::value_to_string(&o[i]),
                    helpers::value_to_string(&h[i]),
                    helpers::value_to_string(&l[i]),
                    helpers::value_to_string(&c[i]),
                    helpers::value_to_string(&vol[i]),
                ]);
            }
        }
    }

    Ok(CommandOutput::new(data, headers, rows))
}

async fn price_increments(client: &IndodaxClient) -> Result<CommandOutput> {
    let data: Value = client.public_get("/api/price_increments").await?;
    if data.is_object() {
        let headers = vec!["Pair".into(), "Increment".into()];
        let mut rows: Vec<Vec<String>> = Vec::new();
        if let Value::Object(map) = &data["increments"] {
            for (key, val) in map {
                rows.push(vec![key.clone(), helpers::value_to_string(val)]);
            }
        }
        rows.sort_by(|a, b| a[0].cmp(&b[0]));
        Ok(CommandOutput::new(data, headers, rows))
    } else {
        let (headers, rows) = helpers::flatten_json_to_table(&data);
        Ok(CommandOutput::new(data, headers, rows))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_first_of_first_key_exists() {
        let val = json!({"a": "1", "b": "2"});
        let result = first_of(&val, &["a", "b"]);
        assert_eq!(result, &json!("1"));
    }

    #[test]
    fn test_first_of_second_key_exists() {
        let val = json!({"a": null, "b": "2"});
        let result = first_of(&val, &["a", "b"]);
        assert_eq!(result, &json!("2"));
    }

    #[test]
    fn test_first_of_no_keys_exist() {
        let val = json!({"a": null, "b": null});
        let result = first_of(&val, &["a", "b"]);
        assert_eq!(result, &serde_json::Value::Null);
    }

    #[test]
    fn test_first_of_empty_value() {
        let val = json!({"a": "", "b": "value"});
        let result = first_of(&val, &["a", "b"]);
        // first_of skips empty strings
        assert_eq!(result, &json!("value"));
    }

    #[test]
    fn test_first_of_null_value() {
        let val = json!({"a": "null", "b": "value"});
        let result = first_of(&val, &["a", "b"]);
        // first_of skips values where string representation is "null"
        assert_eq!(result, &json!("value"));
    }

    #[test]
    fn test_market_command_variants() {
        let _cmd1 = MarketCommand::ServerTime;
        let _cmd2 = MarketCommand::Pairs;
        let _cmd3 = MarketCommand::Ticker { pair: "btc_idr".into() };
        let _cmd4 = MarketCommand::TickerAll;
        let _cmd5 = MarketCommand::Summaries;
        let _cmd6 = MarketCommand::Orderbook { pair: "btcidr".into() };
        let _cmd7 = MarketCommand::Trades { pair: "btcidr".into() };
        let _cmd8 = MarketCommand::Ohlc { 
            symbol: "BTCIDR".into(), 
            timeframe: "60".into(), 
            from: None, 
            to: None 
        };
        let _cmd9 = MarketCommand::PriceIncrements;
    }

    #[test]
    fn test_first_of_with_json_null() {
        let val = json!(null);
        let result = first_of(&val, &["key"]);
        assert_eq!(result, &serde_json::Value::Null);
    }

    #[test]
    fn test_first_of_empty_keys() {
        let val = json!({"a": 1});
        let result = first_of(&val, &[]);
        assert_eq!(result, &serde_json::Value::Null);
    }
}
