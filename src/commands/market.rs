use crate::client::IndodaxClient;
use crate::commands::helpers;
use crate::output::CommandOutput;
use anyhow::Result;
use serde_json::Value;

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
        #[arg(default_value = "btc_idr")]
        pair: String,
        #[arg(long, default_value = "20", help = "Number of bid/ask levels to show")]
        levels: usize,
    },

    #[command(name = "orderbook-grouped", about = "Get grouped order book for a pair")]
    OrderbookGrouped {
        #[arg(default_value = "btc_idr")]
        pair: String,
        #[arg(long, default_value = "100000", help = "Price grouping interval")]
        grouping: f64,
        #[arg(long, default_value = "10", help = "Number of grouped levels to show")]
        depth: usize,
    },

    #[command(name = "spreads", about = "Get current bid/ask spread for a pair")]
    Spreads {
        #[arg(default_value = "btc_idr")]
        pair: String,
    },

    #[command(name = "trades", about = "Get recent trades for a pair")]
    Trades {
        #[arg(default_value = "btc_idr")]
        pair: String,
    },

    #[command(
        name = "ohlc",
        about = "Get OHLCV candle data (default --from is 24h ago)"
    )]
    Ohlc {
        #[arg(short, long, default_value = "btc_idr")]
        symbol: String,
        #[arg(long, default_value = "60")]
        timeframe: String,
        #[arg(short, long, help = "Start timestamp in seconds (default: 24h ago)")]
        from: Option<u64>,
        #[arg(long, help = "End timestamp in seconds (default: now)")]
        to: Option<u64>,
    },

    #[command(name = "price-increments", about = "Get price increments (tick sizes)")]
    PriceIncrements,

    #[command(name = "webdata", about = "Get market webdata for a pair")]
    WebData {
        #[arg(default_value = "btc_idr")]
        pair: String,
    },

    #[command(name = "chatroom-history", about = "Get chatroom history")]
    ChatHistory,

    #[command(name = "pairs-v2", about = "Get detailed pairs info (V2)")]
    PairsV2 {
        #[arg(short, long)]
        pair: Option<String>,
    },

    #[command(name = "search-v2", about = "Search markets (TradingView Search V2)")]
    SearchV2,

    #[command(name = "terminal-trade", about = "Get terminal trading data")]
    TerminalTrade {
        #[arg(default_value = "btc_idr")]
        pair: String,
    },

    #[command(name = "terminal-market", about = "Get terminal market data")]
    TerminalMarket {
        #[arg(default_value = "btc_idr")]
        pair: String,
    },

    #[command(name = "terminal-categories", about = "Get terminal market categories")]
    TerminalCategories,

    #[command(name = "onramp-config", about = "Get onramp config for a pair")]
    OnrampConfig {
        #[arg(default_value = "usdt_idr")]
        pair: String,
    },

    #[command(name = "news", about = "Get news for an asset")]
    News {
        #[arg(default_value = "btc")]
        asset: String,
        #[arg(short, long, default_value = "1")]
        page: u32,
    },
}

pub async fn execute(client: &IndodaxClient, cmd: &MarketCommand) -> Result<CommandOutput> {
    match cmd {
        MarketCommand::ServerTime => server_time(client).await,
        MarketCommand::Pairs => pairs(client).await,
        MarketCommand::Ticker { pair: p } => {
            let pair = helpers::normalize_pair(p);
            ticker(client, &pair).await
        }
        MarketCommand::TickerAll => ticker_all(client).await,
        MarketCommand::Summaries => summaries(client).await,
        MarketCommand::Orderbook { pair, levels } => {
            let pair = helpers::normalize_pair(pair);
            orderbook(client, &pair, *levels).await
        }
        MarketCommand::OrderbookGrouped { pair, grouping, depth } => {
            let pair = helpers::normalize_pair(pair);
            orderbook_grouped(client, &pair, *grouping, *depth).await
        }
        MarketCommand::Spreads { pair } => {
            let pair = helpers::normalize_pair(pair);
            spreads(client, &pair).await
        }
        MarketCommand::Trades { pair: p } => {
            let pair = helpers::normalize_pair(p);
            trades(client, &pair).await
        }
        MarketCommand::Ohlc {
            symbol,
            timeframe,
            from,
            to,
        } => {
            // Indodax history API requires symbols like BTCIDR (no underscore, uppercase)
            let sym = helpers::normalize_pair_v2(symbol).to_uppercase();
            ohlc(client, &sym, timeframe, *from, *to).await
        }
        MarketCommand::PriceIncrements => price_increments(client).await,
        MarketCommand::WebData { pair } => {
            let sym = helpers::normalize_pair_v2(pair).to_uppercase();
            webdata(client, &sym).await
        }
        MarketCommand::ChatHistory => chat_history(client).await,
        MarketCommand::PairsV2 { pair } => pairs_v2(client, pair.as_deref()).await,
        MarketCommand::SearchV2 => search_v2(client).await,
        MarketCommand::TerminalTrade { pair } => {
            let sym = helpers::normalize_pair_v2(pair).to_lowercase();
            terminal_trade(client, &sym).await
        }
        MarketCommand::TerminalMarket { pair } => {
            let sym = helpers::normalize_pair_v2(pair).to_lowercase();
            terminal_market(client, &sym).await
        }
        MarketCommand::TerminalCategories => terminal_categories(client).await,
        MarketCommand::OnrampConfig { pair } => {
            let sym = helpers::normalize_pair_v2(pair).to_lowercase();
            onramp_config(client, &sym).await
        }
        MarketCommand::News { asset, page } => news(client, asset, *page).await,
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
            "Pair".into(),
            "Last".into(),
            "High".into(),
            "Low".into(),
            "Buy".into(),
            "Sell".into(),
            "Vol (base)".into(),
            "Vol (quote)".into(),
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
                    helpers::value_to_string(helpers::first_of(val, &["vol_btc", "vol_base"])),
                    helpers::value_to_string(helpers::first_of(val, &["vol_idr", "vol_traded"])),
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
            "Pair".into(),
            "Last".into(),
            "High".into(),
            "Low".into(),
            "Vol (base)".into(),
            "Vol (quote)".into(),
        ];
        let mut rows: Vec<Vec<String>> = Vec::new();
        if let Value::Object(map) = summaries {
            for (key, val) in map {
                rows.push(vec![
                    key.clone(),
                    helpers::value_to_string(&val["last"]),
                    helpers::value_to_string(&val["high"]),
                    helpers::value_to_string(&val["low"]),
                    helpers::value_to_string(helpers::first_of(val, &["vol_btc", "vol_base"])),
                    helpers::value_to_string(helpers::first_of(val, &["vol_idr", "vol_traded"])),
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

async fn orderbook(client: &IndodaxClient, pair: &str, levels: usize) -> Result<CommandOutput> {
    let data: Value = client.public_get(&format!("/api/depth/{}", pair)).await?;
    let headers = vec!["Side".into(), "Price".into(), "Amount".into()];
    let mut rows: Vec<Vec<String>> = Vec::new();
    let buys = &data["buy"];
    let sells = &data["sell"];
    if let Value::Array(arr) = buys {
        for entry in arr.iter().take(levels) {
            if let Some(row_arr) = entry.as_array().filter(|a| a.len() >= 2) {
                rows.push(vec![
                    "BUY".into(),
                    helpers::value_to_string(&row_arr[0]),
                    helpers::value_to_string(&row_arr[1]),
                ]);
            }
        }
    }
    if let Value::Array(arr) = sells {
        for entry in arr.iter().rev().take(levels) {
            if let Some(row_arr) = entry.as_array().filter(|a| a.len() >= 2) {
                rows.push(vec![
                    "SELL".into(),
                    helpers::value_to_string(&row_arr[0]),
                    helpers::value_to_string(&row_arr[1]),
                ]);
            }
        }
    }
    let level_count = rows.len() / 2;
    Ok(CommandOutput::new(data, headers, rows)
        .with_addendum(format!("Showing {} bid/ask levels", level_count)))
}

pub(crate) async fn orderbook_grouped(
    client: &IndodaxClient,
    pair: &str,
    grouping: f64,
    depth: usize,
) -> Result<CommandOutput> {
    let data: Value = client.public_get(&format!("/api/depth/{}", pair)).await?;
    let headers = vec!["Side".into(), "Price (Group)".into(), "Total Amount".into()];
    let mut rows: Vec<Vec<String>> = Vec::new();

    let mut buy_groups: std::collections::BTreeMap<i64, f64> = std::collections::BTreeMap::new();
    let mut sell_groups: std::collections::BTreeMap<i64, f64> = std::collections::BTreeMap::new();

    if let Some(buys) = data["buy"].as_array() {
        for entry in buys {
            if let Some(row_arr) = entry.as_array().filter(|a| a.len() >= 2) {
                let price = row_arr[0].as_str().and_then(|s| s.parse::<f64>().ok()).unwrap_or(0.0);
                let amount = row_arr[1].as_str().and_then(|s| s.parse::<f64>().ok()).unwrap_or(0.0);
                let group = (price / grouping).floor() as i64;
                *buy_groups.entry(group).or_insert(0.0) += amount;
            }
        }
    }

    if let Some(sells) = data["sell"].as_array() {
        for entry in sells {
            if let Some(row_arr) = entry.as_array().filter(|a| a.len() >= 2) {
                let price = row_arr[0].as_str().and_then(|s| s.parse::<f64>().ok()).unwrap_or(0.0);
                let amount = row_arr[1].as_str().and_then(|s| s.parse::<f64>().ok()).unwrap_or(0.0);
                let group = (price / grouping).ceil() as i64;
                *sell_groups.entry(group).or_insert(0.0) += amount;
            }
        }
    }

    // Process buys (highest group first)
    for (&group, &amount) in buy_groups.iter().rev().take(depth) {
        rows.push(vec![
            "BUY".into(),
            format!("{:.0}", group as f64 * grouping),
            format!("{:.8}", amount),
        ]);
    }

    // Process sells (lowest group first)
    for (&group, &amount) in sell_groups.iter().take(depth) {
        rows.push(vec![
            "SELL".into(),
            format!("{:.0}", group as f64 * grouping),
            format!("{:.8}", amount),
        ]);
    }

    Ok(CommandOutput::new(data, headers, rows).with_addendum(format!(
        "Grouped by {} units, showing top {} levels per side",
        grouping, depth
    )))
}

pub(crate) async fn spreads(client: &IndodaxClient, pair: &str) -> Result<CommandOutput> {
    let data: Value = client.public_get(&format!("/api/ticker/{}", pair)).await?;
    let ticker = &data["ticker"];

    let last = ticker["last"].as_str().and_then(|s| s.parse::<f64>().ok()).unwrap_or(0.0);
    let buy = ticker["buy"].as_str().and_then(|s| s.parse::<f64>().ok()).unwrap_or(0.0);
    let sell = ticker["sell"].as_str().and_then(|s| s.parse::<f64>().ok()).unwrap_or(0.0);

    let spread_abs = (sell - buy).abs();
    let spread_pct = if buy > 0.0 {
        (spread_abs / buy) * 100.0
    } else {
        0.0
    };

    let headers = vec!["Field".into(), "Value".into()];
    let rows = vec![
        vec!["Pair".into(), pair.to_uppercase()],
        vec!["Last Price".into(), format!("{:.0}", last)],
        vec!["Best Bid (Buy)".into(), format!("{:.0}", buy)],
        vec!["Best Ask (Sell)".into(), format!("{:.0}", sell)],
        vec!["Spread (Absolute)".into(), format!("{:.0}", spread_abs)],
        vec!["Spread (%)".into(), format!("{:.4}%", spread_pct)],
    ];

    let json_data = serde_json::json!({
        "pair": pair,
        "last": last,
        "buy": buy,
        "sell": sell,
        "spread_abs": spread_abs,
        "spread_pct": spread_pct,
    });

    Ok(CommandOutput::new(json_data, headers, rows))
}

async fn trades(client: &IndodaxClient, pair: &str) -> Result<CommandOutput> {
    let pair_v2 = pair.replace('_', "");
    let data: Value = client
        .public_get(&format!("/api/trades/{}", pair_v2))
        .await?;
    let headers = vec![
        "TID".into(),
        "Date".into(),
        "Price".into(),
        "Amount".into(),
        "Type".into(),
    ];
    let mut rows: Vec<Vec<String>> = Vec::new();
    if let Value::Array(arr) = &data {
        for trade in arr.iter().take(50) {
            let ts = trade["date"]
                .as_str()
                .and_then(|s| s.parse::<u64>().ok())
                .or_else(|| trade["date"].as_u64())
                .unwrap_or(0);
            let ts = if ts > 1_000_000_000_000 {
                ts / 1000
            } else {
                ts
            };
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
    let mut ohlc_warnings: Vec<String> = Vec::new();
    fn normalize_ohlc_ts(ts: u64, label: &str, warnings: &mut Vec<String>) -> u64 {
        let mut ts = ts;
        if ts > 1_000_000_000_000 {
            warnings.push(format!("[MARKET] Warning: {} timestamp ({}) looks like milliseconds. Converting to seconds.", label, ts));
            ts /= 1000;
        }
        ts
    }

    let now_secs = crate::commands::helpers::now_millis() / 1000;
    let from = from.map(|v| normalize_ohlc_ts(v, "--from", &mut ohlc_warnings));
    let to = to.map(|v| normalize_ohlc_ts(v, "--to", &mut ohlc_warnings));
    let from_val = from
        .map(|v| v.to_string())
        .unwrap_or_else(|| (now_secs - crate::commands::helpers::ONE_DAY_SECS).to_string());
    let to_val = to
        .map(|v| v.to_string())
        .unwrap_or_else(|| now_secs.to_string());

    let data: Value = client
        .public_get_v2(
            "/tradingview/history_v2",
            &[
                ("symbol", symbol),
                ("tf", timeframe),
                ("from", &from_val),
                ("to", &to_val),
            ],
        )
        .await?;

    let headers = vec![
        "Time".into(),
        "Open".into(),
        "High".into(),
        "Low".into(),
        "Close".into(),
        "Volume".into(),
    ];
    let mut rows: Vec<Vec<String>> = Vec::new();

    if let Value::Array(ref arr) = data {
        // Handle array of objects format (modern)
        for item in arr {
            rows.push(vec![
                helpers::format_timestamp(
                    item.get("Time")
                        .or(item.get("t"))
                        .and_then(|v| v.as_u64())
                        .unwrap_or(0),
                    false,
                ),
                helpers::value_to_string(
                    item.get("Open").or(item.get("o")).unwrap_or(&Value::Null),
                ),
                helpers::value_to_string(
                    item.get("High").or(item.get("h")).unwrap_or(&Value::Null),
                ),
                helpers::value_to_string(item.get("Low").or(item.get("l")).unwrap_or(&Value::Null)),
                helpers::value_to_string(
                    item.get("Close").or(item.get("c")).unwrap_or(&Value::Null),
                ),
                helpers::value_to_string(
                    item.get("Volume").or(item.get("v")).unwrap_or(&Value::Null),
                ),
            ]);
        }
    } else if let Value::Object(ref map) = data {
        // Handle parallel arrays format (legacy)
        let times = map.get("t").and_then(|v| v.as_array());
        let opens = map.get("o").and_then(|v| v.as_array());
        let highs = map.get("h").and_then(|v| v.as_array());
        let lows = map.get("l").and_then(|v| v.as_array());
        let closes = map.get("c").and_then(|v| v.as_array());
        let volumes = map.get("v").and_then(|v| v.as_array());

        if let (Some(t), Some(o), Some(h), Some(l), Some(c), Some(vol)) =
            (times, opens, highs, lows, closes, volumes)
        {
            let len = t
                .len()
                .min(o.len())
                .min(h.len())
                .min(l.len())
                .min(c.len())
                .min(vol.len());
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

    let mut output = CommandOutput::new(data, headers, rows);
    for w in ohlc_warnings {
        output = output.with_warning(w);
    }
    Ok(output)
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

async fn webdata(client: &IndodaxClient, pair: &str) -> Result<CommandOutput> {
    let data = client.get_webdata(pair).await?;
    let (headers, rows) = helpers::flatten_json_to_table(&data);
    Ok(CommandOutput::new(data, headers, rows))
}

async fn chat_history(client: &IndodaxClient) -> Result<CommandOutput> {
    let data = client.get_chatroom_history().await?;
    let headers = vec!["User".into(), "Message".into(), "Time".into()];
    let mut rows: Vec<Vec<String>> = Vec::new();
    if let Some(arr) = data.as_array() {
        for msg in arr {
            rows.push(vec![
                helpers::value_to_string(&msg["username"]),
                helpers::value_to_string(&msg["message"]),
                helpers::value_to_string(&msg["time"]),
            ]);
        }
    }
    Ok(CommandOutput::new(data, headers, rows))
}

async fn pairs_v2(client: &IndodaxClient, pair: Option<&str>) -> Result<CommandOutput> {
    let data = client.get_pairs_v2(pair).await?;
    let (headers, rows) = helpers::flatten_json_to_table(&data);
    Ok(CommandOutput::new(data, headers, rows))
}

async fn search_v2(client: &IndodaxClient) -> Result<CommandOutput> {
    let data = client.get_tv_search().await?;
    let (headers, rows) = helpers::flatten_json_to_table(&data);
    Ok(CommandOutput::new(data, headers, rows))
}

async fn terminal_trade(client: &IndodaxClient, pair: &str) -> Result<CommandOutput> {
    let data = client.get_terminal_trade(pair).await?;
    let (headers, rows) = helpers::flatten_json_to_table(&data);
    Ok(CommandOutput::new(data, headers, rows))
}

async fn terminal_market(client: &IndodaxClient, pair: &str) -> Result<CommandOutput> {
    let data = client.get_terminal_market_data(pair).await?;
    let (headers, rows) = helpers::flatten_json_to_table(&data);
    Ok(CommandOutput::new(data, headers, rows))
}

async fn terminal_categories(client: &IndodaxClient) -> Result<CommandOutput> {
    let data = client.get_terminal_market_category().await?;
    let (headers, rows) = helpers::flatten_json_to_table(&data);
    Ok(CommandOutput::new(data, headers, rows))
}

async fn onramp_config(client: &IndodaxClient, pair: &str) -> Result<CommandOutput> {
    let data = client.get_onramp_config(pair).await?;
    let (headers, rows) = helpers::flatten_json_to_table(&data);
    Ok(CommandOutput::new(data, headers, rows))
}

async fn news(client: &IndodaxClient, asset: &str, page: u32) -> Result<CommandOutput> {
    let html = client.get_news(asset, page).await?;
    let data =
        serde_json::json!({ "html_summary": html.chars().take(200).collect::<String>() + "..." });
    let headers = vec!["News Content".into()];
    let rows = vec![vec![html]];
    Ok(CommandOutput::new(data, headers, rows))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_market_command_variants() {
        let _cmd1 = MarketCommand::ServerTime;
        let _cmd2 = MarketCommand::Pairs;
        let _cmd3 = MarketCommand::Ticker {
            pair: "btc_idr".into(),
        };
        let _cmd4 = MarketCommand::TickerAll;
        let _cmd5 = MarketCommand::Summaries;
        let _cmd6 = MarketCommand::Orderbook {
            pair: "btcidr".into(),
            levels: 20,
        };
        let _cmd7 = MarketCommand::Trades {
            pair: "btcidr".into(),
        };
        let _cmd8 = MarketCommand::Ohlc {
            symbol: "BTCIDR".into(),
            timeframe: "60".into(),
            from: None,
            to: None,
        };
        let _cmd9 = MarketCommand::PriceIncrements;
    }

    #[test]
    fn test_first_of_with_json_null() {
        let val = json!(null);
        let result = helpers::first_of(&val, &["key"]);
        assert_eq!(result, &serde_json::Value::Null);
    }

    #[test]
    fn test_first_of_empty_keys() {
        let val = json!({"a": 1});
        let result = helpers::first_of(&val, &[]);
        assert_eq!(result, &serde_json::Value::Null);
    }
}
