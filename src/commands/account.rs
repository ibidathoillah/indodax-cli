use crate::auth::Signer;
use crate::client::IndodaxClient;
use crate::commands::helpers;
use crate::config::IndodaxConfig;
use crate::output::CommandOutput;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, clap::Subcommand)]
pub enum AccountCommand {
    #[command(name = "info", about = "Get account information and balances")]
    Info,

    #[command(name = "balance", about = "Show account balances")]
    Balance,

    #[command(name = "open-orders", about = "List open orders")]
    OpenOrders {
        #[arg(short, long, help = "Filter by trading pair")]
        pair: Option<String>,
    },

    #[command(name = "order-history", about = "Get order history (v2 API)")]
    OrderHistory {
        #[arg(short, long, default_value = "btc_idr")]
        symbol: String,
        #[arg(short, long, default_value = "100")]
        limit: u32,
    },

    #[command(name = "trade-history", about = "Get trade fill history (v2 API)")]
    TradeHistory {
        #[arg(short, long, default_value = "btc_idr")]
        symbol: String,
        #[arg(short, long, default_value = "100")]
        limit: u32,
    },

    #[command(name = "trans-history", about = "Get deposit and withdrawal history")]
    TransHistory,

    #[command(name = "get-order", about = "Get order details by order ID")]
    GetOrder {
        #[arg(long)]
        order_id: u64,
        #[arg(long)]
        pair: String,
    },

    #[command(name = "equity-snap", about = "Record a portfolio equity snapshot")]
    EquitySnap,

    #[command(name = "equity-history", about = "View equity snapshot history")]
    EquityHistory {
        #[arg(short, long, default_value = "20", help = "Number of snapshots to show")]
        limit: usize,
        #[arg(long, help = "Show all snapshots")]
        all: bool,
    },
}

pub async fn execute(
    client: &IndodaxClient,
    cmd: &AccountCommand,
) -> Result<CommandOutput> {
    match cmd {
        AccountCommand::Info => info(client).await,
        AccountCommand::Balance => balance(client).await,
        AccountCommand::OpenOrders { pair } => {
            let pair = pair.as_ref().map(|p| helpers::normalize_pair(p));
            open_orders(client, pair.as_deref()).await
        }
        AccountCommand::OrderHistory { symbol, limit } => {
            let symbol = helpers::normalize_pair(symbol);
            order_history(client, &symbol, *limit).await
        }
        AccountCommand::TradeHistory { symbol, limit } => {
            let symbol = helpers::normalize_pair(symbol);
            trade_history(client, &symbol, *limit).await
        }
        AccountCommand::TransHistory => trans_history(client).await,
        AccountCommand::GetOrder { order_id, pair } => {
            let pair = helpers::normalize_pair(pair);
            get_order(client, *order_id, &pair).await
        }
        AccountCommand::EquitySnap => equity_snap(client).await,
        AccountCommand::EquityHistory { limit, all } => {
            equity_history(*limit, *all)
        }
    }
}

async fn info(client: &IndodaxClient) -> Result<CommandOutput> {
    let data: serde_json::Value =
        client.private_post_v1("getInfo", &HashMap::new()).await?;

    let headers = vec![
        "Field".into(), "Value".into(),
    ];
    let mut rows: Vec<Vec<String>> = vec![
        vec!["Name".into(), helpers::value_to_string(data.get("name").unwrap_or(&serde_json::Value::Null))],
        vec!["User ID".into(), helpers::value_to_string(data.get("user_id").unwrap_or(&serde_json::Value::Null))],
        vec!["Server Time".into(), helpers::format_timestamp(data["server_time"].as_u64().unwrap_or(0), true)],
        vec!["Vip Level".into(), helpers::value_to_string(data.get("vip_level").unwrap_or(&serde_json::Value::Null))],
        vec!["Verified".into(), helpers::value_to_string(data.get("verified_user").unwrap_or(&serde_json::Value::Null))],
    ];

    let balance = &data["balance"];
    if let serde_json::Value::Object(bal_map) = balance {
        let mut entries: Vec<(&String, f64)> = bal_map
            .iter()
            .map(|(k, v)| {
                let val = v.as_str().and_then(|s| s.parse::<f64>().ok())
                    .or_else(|| v.as_f64())
                    .unwrap_or(0.0);
                (k, val)
            })
            .collect();
        entries.sort_by(|a, b| a.0.cmp(b.0));
        for (k, amount) in entries {
            let formatted = helpers::format_balance(k, amount);
            rows.push(vec![k.clone(), formatted]);
        }
    }

    Ok(CommandOutput::new(data, headers, rows))
}

async fn balance(client: &IndodaxClient) -> Result<CommandOutput> {
    let data: serde_json::Value =
        client.private_post_v1("getInfo", &HashMap::new()).await?;

    let balance = &data["balance"];
    let headers = vec!["Currency".into(), "Balance".into()];
    let mut rows: Vec<Vec<String>> = Vec::new();

    if let serde_json::Value::Object(bal_map) = balance {
        let mut entries: Vec<(String, f64)> = bal_map
            .iter()
            .map(|(k, v)| {
                let val = v.as_str().and_then(|s| s.parse::<f64>().ok())
                    .or_else(|| v.as_f64())
                    .unwrap_or(0.0);
                (k.clone(), val)
            })
            .collect();
        entries.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        for (currency, amount) in entries {
            let formatted = helpers::format_balance(&currency, amount);
            rows.push(vec![currency, formatted]);
        }
    }

    Ok(CommandOutput::new(data, headers, rows))
}

async fn open_orders(
    client: &IndodaxClient,
    pair: Option<&str>,
) -> Result<CommandOutput> {
    let mut params = HashMap::new();
    if let Some(p) = pair {
        params.insert("pair".into(), p.to_string());
    }
    let data: serde_json::Value =
        client.private_post_v1("openOrders", &params).await?;

    let orders = &data["orders"];
    let headers = vec![
        "Order ID".into(), "Pair".into(), "Type".into(), "Side".into(),
        "Price".into(), "Amount".into(), "Remaining".into(), "Time".into(),
    ];
    let mut rows: Vec<Vec<String>> = Vec::new();

    if let serde_json::Value::Object(orders_map) = orders {
        for (order_id, order_val) in orders_map {
            let pair = helpers::value_to_string(
                priv_get(order_val, &["pair", "market", "symbol"]),
            );
            let order_type = helpers::value_to_string(
                priv_get(order_val, &["type", "order_type"]),
            );
            let side = if order_type.to_lowercase().contains("sell") {
                "SELL"
            } else {
                "BUY"
            };

            let remaining = helpers::value_to_string(
                priv_get(order_val, &["remaining", "remain_volume", "remaining_volume"]),
            );
            let base_amount = order_val.get("order_btc")
                .or_else(|| order_val.get("order_base"))
                .or_else(|| order_val.get("amount"))
                .map(helpers::value_to_string)
                .unwrap_or_default();

            let time_val = order_val.get("submit_time")
                .or_else(|| order_val.get("created_at"))
                .or_else(|| order_val.get("time"))
                .map(|v| {
                    let ts = v.as_u64().unwrap_or(0);
                    if ts > 1_000_000_000_000 {
                        helpers::format_timestamp(ts, true)
                    } else {
                        helpers::format_timestamp(ts, false)
                    }
                })
                .unwrap_or_default();

            rows.push(vec![
                order_id.to_string(),
                pair,
                order_type,
                side.into(),
                helpers::value_to_string(
                    priv_get(order_val, &["price", "order_price"]),
                ),
                base_amount,
                remaining,
                time_val,
            ]);
        }
    }

    rows.sort_by(|a, b| {
        match (b[0].parse::<u64>().ok(), a[0].parse::<u64>().ok()) {
            (Some(bv), Some(av)) => bv.cmp(&av),
            _ => b[0].cmp(&a[0]),
        }
    });
    let count = rows.len();
    Ok(CommandOutput::new(data, headers, rows)
        .with_addendum(format!("{} open orders", count)))
}

async fn order_history(
    client: &IndodaxClient,
    symbol: &str,
    limit: u32,
) -> Result<CommandOutput> {
    let now = Signer::now_millis();
    let start = now - crate::commands::helpers::ONE_DAY_MS;

    let mut params = HashMap::new();
    params.insert("symbol".into(), crate::commands::helpers::normalize_pair_v2(symbol));
    params.insert("limit".into(), limit.max(10).to_string());
    params.insert("startTime".into(), start.to_string());
    params.insert("endTime".into(), now.to_string());

    let data: serde_json::Value =
        client.private_get_v2("/api/v2/order/histories", &params).await?;

    let headers = vec![
        "Order ID".into(), "Symbol".into(), "Side".into(), "Type".into(),
        "Price".into(), "Qty".into(), "Status".into(), "Time".into(),
    ];
    let mut rows: Vec<Vec<String>> = Vec::new();

    if let serde_json::Value::Array(arr) = &data {
        for order in arr.iter().take(limit as usize) {
            rows.push(vec![
                helpers::value_to_string(priv_get(order, &["orderId", "order_id"])),
                helpers::value_to_string(priv_get(order, &["symbol", "pair"])),
                helpers::value_to_string(priv_get(order, &["side", "order_side"])),
                helpers::value_to_string(priv_get(order, &["type", "order_type"])),
                helpers::value_to_string(priv_get(order, &["price", "order_price"])),
                helpers::value_to_string(priv_get(order, &["origQty", "orig_qty", "qty"])),
                helpers::value_to_string(priv_get(order, &["status", "order_status"])),
                helpers::value_to_string(priv_get(order, &["time", "created_at"])),
            ]);
        }
    }

    Ok(CommandOutput::new(data, headers, rows))
}

async fn trade_history(
    client: &IndodaxClient,
    symbol: &str,
    limit: u32,
) -> Result<CommandOutput> {
    let now = Signer::now_millis();
    let start = now - crate::commands::helpers::ONE_DAY_MS;

    let mut params = HashMap::new();
    params.insert("symbol".into(), crate::commands::helpers::normalize_pair_v2(symbol));
    params.insert("limit".into(), limit.max(10).to_string());
    params.insert("startTime".into(), start.to_string());
    params.insert("endTime".into(), now.to_string());

    let data: serde_json::Value =
        client.private_get_v2("/api/v2/myTrades", &params).await?;

    let headers = vec![
        "Trade ID".into(), "Order ID".into(), "Symbol".into(), "Side".into(),
        "Price".into(), "Qty".into(), "Fee".into(), "Time".into(),
    ];
    let mut rows: Vec<Vec<String>> = Vec::new();

    if let serde_json::Value::Array(arr) = &data {
        for trade in arr.iter().take(limit as usize) {
            rows.push(vec![
                helpers::value_to_string(priv_get(trade, &["id", "tradeId", "trade_id"])),
                helpers::value_to_string(priv_get(trade, &["orderId", "order_id"])),
                helpers::value_to_string(priv_get(trade, &["symbol", "pair"])),
                helpers::value_to_string(priv_get(trade, &["side"])),
                helpers::value_to_string(priv_get(trade, &["price"])),
                helpers::value_to_string(priv_get(trade, &["qty", "quantity"])),
                helpers::value_to_string(priv_get(trade, &["commission", "fee"])),
                helpers::value_to_string(priv_get(trade, &["time", "timestamp"])),
            ]);
        }
    }

    Ok(CommandOutput::new(data, headers, rows))
}

async fn trans_history(client: &IndodaxClient) -> Result<CommandOutput> {
    let data: serde_json::Value =
        client.private_post_v1("transHistory", &HashMap::new()).await?;

    let headers = vec![
        "ID".into(), "Type".into(), "Currency".into(), "Amount".into(),
        "Fee".into(), "Status".into(), "Time".into(),
    ];
    let mut rows: Vec<Vec<String>> = Vec::new();

    let mut all_trans = Vec::new();
    
    if let Some(obj) = data.get("withdraw").and_then(|v| v.as_object()) {
        for (id, val) in obj {
            all_trans.push((id, "WITHDRAW", val));
        }
    }
    if let Some(obj) = data.get("deposit").and_then(|v| v.as_object()) {
        for (id, val) in obj {
            all_trans.push((id, "DEPOSIT", val));
        }
    }
    if let Some(obj) = data.get("transactions").and_then(|v| v.as_object()) {
        for (id, val) in obj {
            let tx_type = if id.contains("withdraw") {
                "WITHDRAW"
            } else if id.contains("deposit") {
                "DEPOSIT"
            } else {
                "TRANS"
            };
            all_trans.push((id, tx_type, val));
        }
    }

    for (id, tx_type, entry) in all_trans {
        rows.push(vec![
            id.clone(),
            tx_type.into(),
            helpers::value_to_string(
                priv_get(entry, &["currency", "asset", "coin"]),
            ),
            helpers::value_to_string(
                priv_get(entry, &["amount", "value"]),
            ),
            helpers::value_to_string(
                priv_get(entry, &["fee", "withdraw_fee"]),
            ),
            helpers::value_to_string(
                priv_get(entry, &["status", "state"]),
            ),
            helpers::value_to_string(
                priv_get(entry, &["submit_time", "timestamp", "time", "submitted"]),
            ),
        ]);
    }

    rows.sort_by(|a, b| b[0].cmp(&a[0]));
    Ok(CommandOutput::new(data, headers, rows))
}

async fn get_order(
    client: &IndodaxClient,
    order_id: u64,
    pair: &str,
) -> Result<CommandOutput> {
    let mut params = HashMap::new();
    params.insert("order_id".into(), order_id.to_string());
    params.insert("pair".into(), pair.to_string());

    let data: serde_json::Value =
        client.private_post_v1("getOrder", &params).await?;

    let (headers, rows) = helpers::flatten_json_to_table(&data);
    Ok(CommandOutput::new(data, headers, rows))
}

fn priv_get<'a>(val: &'a serde_json::Value, keys: &[&str]) -> &'a serde_json::Value {
    helpers::first_of(val, keys)
}

// ---------------------------------------------------------------------------
// Equity snapshot history
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize, Deserialize)]
struct EquitySnapshot {
    timestamp: u64,
    equity: f64,
}

#[derive(Debug, Serialize, Deserialize)]
struct EquityHistoryData {
    snapshots: Vec<EquitySnapshot>,
}

fn equity_history_path() -> std::path::PathBuf {
    IndodaxConfig::config_dir().join("equity_history.json")
}

fn load_equity_history() -> EquityHistoryData {
    let path = equity_history_path();
    if path.exists() {
        std::fs::read_to_string(&path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or(EquityHistoryData { snapshots: vec![] })
    } else {
        EquityHistoryData { snapshots: vec![] }
    }
}

fn save_equity_history(data: &EquityHistoryData) -> Result<()> {
    let dir = IndodaxConfig::config_dir();
    std::fs::create_dir_all(&dir)?;
    let content = serde_json::to_string_pretty(data)?;
    #[cfg(unix)]
    {
        use std::io::Write;
        use std::os::unix::fs::OpenOptionsExt;
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .mode(0o600)
            .open(equity_history_path())?;
        file.write_all(content.as_bytes())?;
    }
    #[cfg(not(unix))]
    {
        std::fs::write(equity_history_path(), content)?;
    }
    Ok(())
}

async fn calculate_equity(client: &IndodaxClient) -> Result<f64> {
    let info: serde_json::Value = client.private_post_v1("getInfo", &HashMap::new()).await?;

    let mut balances: HashMap<String, f64> = HashMap::new();
    if let Some(bal_map) = info["balance"].as_object() {
        for (k, v) in bal_map {
            let val = v
                .as_str()
                .and_then(|s| s.parse::<f64>().ok())
                .or_else(|| v.as_f64())
                .unwrap_or(0.0);
            if val > 0.0 {
                balances.insert(k.clone(), val);
            }
        }
    }

    let tickers: serde_json::Value = client.public_get("/api/ticker_all").await?;
    let mut prices: HashMap<String, f64> = HashMap::new();
    if let Some(t) = tickers["tickers"].as_object() {
        for (k, v) in t {
            let last = v["last"]
                .as_str()
                .and_then(|s| s.parse::<f64>().ok())
                .or_else(|| v["last"].as_f64())
                .unwrap_or(0.0);
            prices.insert(k.clone(), last);
        }
    }

    let mut total = 0.0;
    let btc_idr = prices.get("btc_idr").copied().unwrap_or(0.0);
    let usdt_idr = prices.get("usdt_idr").copied().unwrap_or(0.0);
    let eth_idr = prices.get("eth_idr").copied().unwrap_or(0.0);

    for (currency, amount) in &balances {
        if currency == "idr" {
            total += amount;
        } else if currency == "btc" {
            total += amount * btc_idr;
        } else if currency == "usdt" {
            total += amount * usdt_idr;
        } else {
            let pair_idr = format!("{}_{}", currency, "idr");
            let pair_btc = format!("{}_{}", currency, "btc");
            let pair_usdt = format!("{}_{}", currency, "usdt");
            let pair_eth = format!("{}_{}", currency, "eth");

            if let Some(price) = prices.get(&pair_idr) {
                total += amount * price;
            } else if let Some(price) = prices.get(&pair_btc) {
                total += amount * price * btc_idr;
            } else if let Some(price) = prices.get(&pair_usdt) {
                total += amount * price * usdt_idr;
            } else if let Some(price) = prices.get(&pair_eth) {
                total += amount * price * eth_idr;
            } else {
                eprintln!("[EQUITY] Warning: No known price pair for {} (value: {}). Contribution set to 0.", currency.to_uppercase(), amount);
            }
        }
    }

    Ok(total)
}

async fn equity_snap(client: &IndodaxClient) -> Result<CommandOutput> {
    let equity = calculate_equity(client).await?;
    let timestamp = Signer::now_millis();

    let snap = EquitySnapshot { timestamp, equity };
    let mut history = load_equity_history();
    history.snapshots.push(snap);

    if history.snapshots.len() > 1000 {
        let keep = history.snapshots.split_off(history.snapshots.len() - 1000);
        history.snapshots = keep;
    }

    save_equity_history(&history)?;

    let count = history.snapshots.len();
    let first_equity = history.snapshots.first().map(|s| s.equity).unwrap_or(equity);
    let peak = history.snapshots.iter().map(|s| s.equity).fold(0.0_f64, f64::max);
    let change = equity - first_equity;
    let change_pct = if first_equity > 0.0 { (change / first_equity) * 100.0 } else { 0.0 };
    let dd_pct = if peak > 0.0 { ((equity / peak) - 1.0) * 100.0 } else { 0.0 };

    let headers = vec!["Metric".into(), "Value".into()];
    let formatted_time = helpers::format_timestamp(timestamp, true);
    let rows = vec![
        vec!["Time".into(), formatted_time],
        vec!["Equity (IDR)".into(), format_equity(equity)],
        vec!["Change".into(), format_change(change)],
        vec!["Change %".into(), format_change_pct(change_pct)],
        vec!["Peak (IDR)".into(), format_equity(peak)],
        vec!["Drawdown %".into(), format_change_pct(dd_pct)],
        vec!["Total Snapshots".into(), count.to_string()],
    ];

    let data = serde_json::json!({
        "timestamp": timestamp,
        "equity": equity,
        "change": change,
        "change_pct": change_pct,
        "peak": peak,
        "drawdown_pct": dd_pct,
        "total_snapshots": count,
    });

    Ok(CommandOutput::new(data, headers, rows))
}

fn equity_history(limit: usize, all: bool) -> Result<CommandOutput> {
    let history = load_equity_history();

    if history.snapshots.is_empty() {
        return Ok(CommandOutput::json(serde_json::json!({
            "status": "ok",
            "message": "No equity snapshots. Use `indodax account equity-snap` to record one.",
            "snapshots": [],
        })));
    }

    let first_equity = history.snapshots.first().map(|s| s.equity).unwrap_or(0.0);

    let headers = vec![
        "Time".into(),
        "Equity (IDR)".into(),
        "Change".into(),
        "Change %".into(),
        "Peak (IDR)".into(),
        "DD %".into(),
    ];

    let snapshots_to_show: Vec<&EquitySnapshot> = if all {
        history.snapshots.iter().collect()
    } else {
        let take = limit.min(history.snapshots.len());
        history.snapshots[history.snapshots.len() - take..]
            .iter()
            .collect()
    };

    let mut rows: Vec<Vec<String>> = Vec::new();
    let mut peak = 0.0_f64;

    for snap in &snapshots_to_show {
        if snap.equity > peak {
            peak = snap.equity;
        }
        let change = snap.equity - first_equity;
        let change_pct = if first_equity > 0.0 {
            (change / first_equity) * 100.0
        } else {
            0.0
        };
        let dd_pct = if peak > 0.0 {
            ((snap.equity / peak) - 1.0) * 100.0
        } else {
            0.0
        };

        rows.push(vec![
            format_timestamp_short(snap.timestamp),
            format_equity(snap.equity),
            format_change(change),
            format_change_pct(change_pct),
            format_equity(peak),
            format_change_pct(dd_pct),
        ]);
    }

    let data = serde_json::json!({
        "count": history.snapshots.len(),
        "first_equity": first_equity,
        "snapshots": history.snapshots.iter().map(|s| serde_json::json!({
            "timestamp": s.timestamp,
            "equity": s.equity,
        })).collect::<Vec<_>>(),
    });

    let count = history.snapshots.len();
    Ok(CommandOutput::new(data, headers, rows)
        .with_addendum(format!("[EQUITY] {} snapshot(s) total", count)))
}

fn format_equity(val: f64) -> String {
    format!("{:>14.2}", val)
}

fn format_change(val: f64) -> String {
    if val >= 0.0 {
        format!("+{:>10.2}", val)
    } else {
        format!("{:>11.2}", val)
    }
}

fn format_change_pct(val: f64) -> String {
    if val >= 0.0 {
        format!("+{:>7.2}%", val)
    } else {
        format!("{:>8.2}%", val)
    }
}

fn format_timestamp_short(ts: u64) -> String {
    let ts_sec = ts / 1000;
    chrono::DateTime::from_timestamp(ts_sec.min(i64::MAX as u64) as i64, 0)
        .map(|dt| dt.format("%b %d  %H:%M:%S").to_string())
        .unwrap_or_else(|| ts.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    #[test]
    fn test_priv_get_existing_key() {
        let val = json!({"name": "Alice", "age": 30});
        let result = priv_get(&val, &["name"]);
        assert_eq!(result, &json!("Alice"));
    }

    #[test]
    fn test_priv_get_first_key_exists() {
        let val = json!({"a": 1, "b": 2});
        let result = priv_get(&val, &["a", "b"]);
        assert_eq!(result, &json!(1));
    }

    #[test]
    fn test_priv_get_second_key_exists() {
        let val = json!({"a": null, "b": "2"});
        let result = priv_get(&val, &["a", "b"]);
        // priv_get delegates to first_of which skips null values
        assert_eq!(result, &json!("2"));
    }

    #[test]
    fn test_priv_get_no_keys_exist() {
        let val = json!({"a": 1});
        let result = priv_get(&val, &["x", "y", "z"]);
        assert_eq!(result, &serde_json::Value::Null);
    }

    #[test]
    fn test_priv_get_with_json_null() {
        let val = json!(null);
        let result = priv_get(&val, &["key"]);
        assert_eq!(result, &serde_json::Value::Null);
    }

    #[test]
    fn test_priv_get_empty_keys() {
        let val = json!({"a": 1});
        let result = priv_get(&val, &[]);
        assert_eq!(result, &serde_json::Value::Null);
    }

    #[test]
    fn test_priv_get_nested_value() {
        let val = json!({"data": {"name": "Bob"}});
        let result = priv_get(&val, &["data"]);
        assert_eq!(result, &json!({"name": "Bob"}));
    }

    #[test]
    fn test_account_command_variants() {
        let _cmd1 = AccountCommand::Info;
        let _cmd2 = AccountCommand::Balance;
        let _cmd3 = AccountCommand::OpenOrders { pair: Some("btc_idr".into()) };
        let _cmd4 = AccountCommand::OrderHistory { symbol: "btc_idr".into(), limit: 100 };
        let _cmd5 = AccountCommand::TradeHistory { symbol: "btc_idr".into(), limit: 100 };
        let _cmd6 = AccountCommand::TransHistory;
        let _cmd7 = AccountCommand::GetOrder { order_id: 123, pair: "btc_idr".into() };
        let _cmd8 = AccountCommand::EquitySnap;
        let _cmd9 = AccountCommand::EquityHistory { limit: 10, all: false };
    }

    #[test]
    fn test_priv_get_with_null_first() {
        let val = json!({"first": null, "second": "value"});
        let result = priv_get(&val, &["first", "second"]);
        // priv_get delegates to first_of which skips null values
        assert_eq!(result, &json!("value"));
    }

    #[test]
    fn test_priv_get_array_value() {
        let val = json!({"arr": [1, 2, 3]});
        let result = priv_get(&val, &["arr"]);
        assert_eq!(result, &json!([1, 2, 3]));
    }

    #[test]
    fn test_priv_get_number_value() {
        let val = json!({"num": 42.5});
        let result = priv_get(&val, &["num"]);
        assert_eq!(result, &json!(42.5));
    }

    #[test]
    fn test_priv_get_bool_value() {
        let val = json!({"flag": true});
        let result = priv_get(&val, &["flag"]);
        assert_eq!(result, &json!(true));
    }
}
