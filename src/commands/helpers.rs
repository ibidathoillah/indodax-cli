use crate::client::IndodaxClient;
use crate::errors::IndodaxError;
use chrono::DateTime;
use serde_json::Value;

pub const PUBLIC_WS_TOKEN_URL: &str = "https://indodax.com/api/ws/v1/generate_token";

/// Default static token from official Indodax Market Data WebSocket documentation.
/// Used for authenticating with the Public Market Data WebSocket (ws3).
pub const DEFAULT_STATIC_WS_TOKEN: &str = "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJleHAiOjE5NDY2MTg0MTV9.UR1lBM6Eqh0yWz-PVirw1uPCxe60FdchR8eNVdsskeo";

/// Fetch a public WebSocket token, with fallback to user configuration and then a hardcoded default.
pub async fn fetch_public_ws_token(client: &IndodaxClient) -> Result<String, anyhow::Error> {
    // 1. Try to fetch dynamically (some Indodax environments support this)
    let resp_res = client.http_client().get(PUBLIC_WS_TOKEN_URL).send().await;
    if let Ok(resp) = resp_res {
        if let Ok(text) = resp.text().await {
            if let Ok(val) = serde_json::from_str::<serde_json::Value>(&text) {
                if let Some(token) = val.get("token").and_then(|t| t.as_str()).or_else(|| {
                    val.get("data")
                        .and_then(|d| d.get("token"))
                        .and_then(|t| t.as_str())
                }) {
                    return Ok(token.to_string());
                }
            }
        }
    }

    // 2. Try to use user-configured token from config/env
    if let Some(token) = client.ws_token() {
        return Ok(token.to_string());
    }

    // 3. Try INDODAX_WS_TOKEN env var
    if let Ok(token) = std::env::var("INDODAX_WS_TOKEN") {
        if !token.is_empty() {
            return Ok(token);
        }
    }

    // 4. Fallback to hardcoded default
    eprintln!("[WS] Warning: Could not fetch dynamic WebSocket token and no configured token found. Using built-in fallback token (may expire). Set INDODAX_WS_TOKEN env var to override.");
    Ok(DEFAULT_STATIC_WS_TOKEN.to_string())
}

pub const ONE_DAY_MS: u64 = 24 * 60 * 60 * 1000;
pub const ONE_DAY_SECS: u64 = 24 * 60 * 60;
pub const BALANCE_EPSILON: f64 = 1e-8;

use web_time::SystemTime;

pub fn now_millis() -> u64 {
    #[cfg(target_arch = "wasm32")]
    {
        js_sys::Date::now() as u64
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64
    }
}

pub fn now_seconds() -> u64 {
    #[cfg(target_arch = "wasm32")]
    {
        (js_sys::Date::now() / 1000.0) as u64
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
    }
}

pub fn get_system_time() -> SystemTime {
    SystemTime::now()
}

pub fn flatten_json_to_table(json: &serde_json::Value) -> (Vec<String>, Vec<Vec<String>>) {
    match json {
        serde_json::Value::Object(map) => {
            let mut headers: Vec<String> = map.keys().cloned().collect();
            headers.sort();
            let row: Vec<String> = headers.iter().map(|k| value_to_string(&map[k])).collect();
            (headers, vec![row])
        }
        serde_json::Value::Array(arr) if !arr.is_empty() => {
            // Collect all unique keys from all array elements
            let mut all_keys: std::collections::BTreeSet<String> =
                std::collections::BTreeSet::new();
            let mut is_obj = false;
            for item in arr {
                if let serde_json::Value::Object(map) = item {
                    is_obj = true;
                    for k in map.keys() {
                        all_keys.insert(k.clone());
                    }
                }
            }
            if is_obj {
                let headers: Vec<String> = all_keys.into_iter().collect();
                let rows: Vec<Vec<String>> = arr
                    .iter()
                    .map(|item| headers.iter().map(|k| value_to_string(&item[k])).collect())
                    .collect();
                (headers, rows)
            } else {
                (
                    vec!["Value".into()],
                    arr.iter().map(|v| vec![value_to_string(v)]).collect(),
                )
            }
        }
        _ => (vec!["Value".into()], vec![vec![value_to_string(json)]]),
    }
}

pub fn value_to_string(v: &serde_json::Value) -> String {
    match v {
        serde_json::Value::Null => String::new(),
        serde_json::Value::Bool(b) => b.to_string(),
        serde_json::Value::Number(n) => n.to_string(),
        serde_json::Value::String(s) => s.clone(),
        serde_json::Value::Array(arr) => {
            let items: Vec<String> = arr.iter().map(value_to_string).collect();
            items.join(", ")
        }
        serde_json::Value::Object(_) => {
            serde_json::to_string(v).unwrap_or_else(|_| "<serialization_error>".to_string())
        }
    }
}

pub fn value_to_string_opt(v: &serde_json::Value) -> Option<String> {
    match v {
        serde_json::Value::Null => None,
        _ => Some(value_to_string(v)),
    }
}

pub fn format_timestamp(ts: u64, millis: bool) -> String {
    let ts_sec = if millis { ts / 1000 } else { ts };
    if let Some(dt) = DateTime::from_timestamp(ts_sec.min(i64::MAX as u64) as i64, 0) {
        dt.format("%Y-%m-%d %H:%M:%S").to_string()
    } else {
        ts.to_string()
    }
}

pub fn normalize_pair(pair: &str) -> String {
    let pair = pair.to_lowercase().replace(['-', '/'], "_");
    if pair.contains('_') || pair.is_empty() {
        return pair;
    }
    let quote_currencies = [
        "usdt", "idr", "btc", "usdc", "eth", "sol", "bnb", "xrp", "ada",
    ];
    for quote in &quote_currencies {
        if let Some(base) = pair.strip_suffix(quote) {
            if !base.is_empty() {
                return format!("{}_{}", base, quote);
            }
        }
    }
    pair
}

pub fn normalize_pair_v2(pair: &str) -> String {
    normalize_pair(pair).replace('_', "")
}

pub fn first_of<'a>(val: &'a Value, keys: &[&str]) -> &'a Value {
    for k in keys {
        if let Some(v) = val.get(*k) {
            match v {
                Value::Null => continue,
                Value::String(s) if s.is_empty() || s == "null" => continue,
                _ => return v,
            }
        }
    }
    &Value::Null
}

/// Check if a currency is fiat (IDR) or a stablecoin.
pub fn is_fiat_or_stable(currency: &str) -> bool {
    matches!(
        currency.to_lowercase().as_str(),
        "idr" | "usdt" | "usdc" | "dai" | "busd" | "pax" | "usde" | "gusd" | "tusd"
    )
}

/// Currency-aware balance formatting: 2 decimals for IDR/fiat/stablecoins, 8 for crypto.
/// Handles extreme values gracefully (no scientific notation).
pub fn format_balance(currency: &str, value: f64) -> String {
    if is_fiat_or_stable(currency) {
        if value.abs() < 0.01 && value != 0.0 {
            "<0.01".to_string()
        } else {
            format!("{:.2}", value)
        }
    } else {
        if value.abs() < 1e-8 && value != 0.0 {
            "<1e-8".to_string()
        } else {
            format!("{:.8}", value)
        }
    }
}

/// Parse a balance value for a given currency from API account info response.
pub fn parse_balance(info: &serde_json::Value, currency: &str) -> f64 {
    info["balance"][currency]
        .as_str()
        .and_then(|s| s.parse::<f64>().ok())
        .or_else(|| info["balance"][currency].as_f64())
        .unwrap_or(0.0)
}

/// Build withdrawal parameters HashMap shared between CLI and MCP.
pub fn build_withdraw_params(
    currency: &str,
    amount: f64,
    address: &str,
    to_username: bool,
    memo: Option<&str>,
    network: Option<&str>,
    callback_url: Option<&str>,
) -> std::collections::HashMap<String, String> {
    let mut params = std::collections::HashMap::new();
    params.insert("currency".into(), currency.to_string());
    params.insert("amount".into(), amount.to_string());

    if to_username {
        params.insert(
            "request_id".into(),
            chrono::Utc::now().timestamp_millis().to_string(),
        );
        params.insert("withdraw_to".into(), address.to_string());
    } else {
        params.insert("address".into(), address.to_string());
    }

    if let Some(m) = memo {
        params.insert("memo".into(), m.to_string());
    }
    if let Some(n) = network {
        params.insert("network".into(), n.to_string());
    }
    if let Some(u) = callback_url {
        params.insert("callback_url".into(), u.to_string());
    }
    params
}

/// Validate a price against the price increment (tick size) for a pair.
/// Returns a warning string if validation fails, or None if the price is valid or can't be checked.
pub async fn validate_tick_size(client: &IndodaxClient, pair: &str, price: f64) -> Option<String> {
    let Ok(data) = client
        .public_get::<serde_json::Value>("/api/price_increments")
        .await
    else {
        return None;
    };
    let increments = data.get("increments").and_then(|v| v.as_object())?;
    let normalized_pair = pair.to_lowercase().replace('-', "_");
    let inc_entry = increments
        .get(&normalized_pair)
        .or_else(|| increments.get(&normalized_pair.replace('_', "")))
        .or_else(|| {
            let alt = normalized_pair.replace('_', "");
            increments
                .keys()
                .find(|k| k.replace('_', "") == alt)
                .and_then(|k| increments.get(k))
        })?;
    let inc_str = inc_entry.as_str()?;
    let inc: f64 = inc_str.parse().ok()?;
    if inc <= 0.0 {
        return None;
    }
    let price_rounded = (price / inc).round() * inc;
    if (price_rounded - price).abs() <= inc * 1e-6 {
        return None;
    }
    Some(format!(
        "[TRADE] Warning: Price {} does not conform to the tick size (increment: {}) for pair {}. The API may reject this order.",
        price, inc_str, pair
    ))
}

/// Fetch all open orders and cancel them one by one.
/// Returns (cancelled_ids, failed_ids).
pub async fn cancel_all_open_orders(
    client: &IndodaxClient,
    pair: Option<&str>,
) -> Result<(Vec<String>, Vec<String>), IndodaxError> {
    use std::collections::HashMap;
    let mut params = HashMap::new();
    if let Some(p) = pair {
        params.insert("pair".to_string(), p.to_string());
    }
    let data: serde_json::Value = client.private_post_v1("openOrders", &params).await?;
    let orders = &data["orders"];
    let mut cancelled_ids: Vec<String> = Vec::new();
    let mut failed_ids: Vec<String> = Vec::new();

    if let serde_json::Value::Object(orders_map) = orders {
        for (order_id, order_val) in orders_map {
            let order_pair = value_to_string(
                order_val
                    .get("pair")
                    .or_else(|| order_val.get("symbol"))
                    .unwrap_or(&serde_json::Value::Null),
            );

            let mut cancel_params = HashMap::new();
            cancel_params.insert("symbol".to_string(), normalize_pair_v2(&order_pair));
            
            let path = format!("/api/v2/order/{}", order_id);
            match client
                .private_delete_v2::<serde_json::Value>(&path, &cancel_params)
                .await
            {
                Ok(_) => cancelled_ids.push(order_id.clone()),
                Err(e) => failed_ids.push(format!("{} ({})", order_id, e)),
            }
        }
    }

    Ok((cancelled_ids, failed_ids))
}

pub fn extract_pairs(data: &serde_json::Value) -> Vec<(String, String)> {
    let mut pairs: Vec<(String, String)> = Vec::new();
    if let serde_json::Value::Object(map) = data {
        for (key, value) in map {
            if let Some(obj) = value.as_object() {
                let base = obj
                    .get("traded_currency")
                    .or_else(|| obj.get("tradedCurrency"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let quote = obj
                    .get("base_currency")
                    .or_else(|| obj.get("baseCurrency"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let symbol = obj
                    .get("symbol")
                    .or_else(|| obj.get("ticker_id"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                pairs.push((key.clone(), format!("{}/{} ({})", base, quote, symbol)));
            }
        }
    } else if let serde_json::Value::Array(arr) = data {
        for item in arr {
            if let Some(obj) = item.as_object() {
                let id = obj
                    .get("id")
                    .or_else(|| obj.get("ticker_id"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let base = obj
                    .get("traded_currency")
                    .or_else(|| obj.get("tradedCurrency"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let quote = obj
                    .get("base_currency")
                    .or_else(|| obj.get("baseCurrency"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let symbol = obj
                    .get("symbol")
                    .or_else(|| obj.get("ticker_id"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                if !id.is_empty() {
                    pairs.push((id.to_string(), format!("{}/{} ({})", base, quote, symbol)));
                }
            }
        }
    }
    pairs.sort_by(|a, b| a.0.cmp(&b.0));
    pairs
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_normalize_pair_already_normalized() {
        assert_eq!(normalize_pair("btc_idr"), "btc_idr");
        assert_eq!(normalize_pair("eth_btc"), "eth_btc");
        assert_eq!(normalize_pair("usdt_idr"), "usdt_idr");
    }

    #[test]
    fn test_normalize_pair_no_underscore() {
        assert_eq!(normalize_pair("btcidr"), "btc_idr");
        assert_eq!(normalize_pair("ethidr"), "eth_idr");
        assert_eq!(normalize_pair("ethbtc"), "eth_btc");
        assert_eq!(normalize_pair("solusdt"), "sol_usdt");
    }

    #[test]
    fn test_normalize_pair_uppercase() {
        assert_eq!(normalize_pair("BTC_IDR"), "btc_idr");
        assert_eq!(normalize_pair("BTCIDR"), "btc_idr");
        assert_eq!(normalize_pair("ETH_BTC"), "eth_btc");
    }

    #[test]
    fn test_normalize_pair_dash_separator() {
        assert_eq!(normalize_pair("btc-idr"), "btc_idr");
        assert_eq!(normalize_pair("ETH-IDR"), "eth_idr");
        assert_eq!(normalize_pair("sol-usdt"), "sol_usdt");
    }

    #[test]
    fn test_normalize_pair_slash_separator() {
        assert_eq!(normalize_pair("btc/idr"), "btc_idr");
        assert_eq!(normalize_pair("ETH/BTC"), "eth_btc");
        assert_eq!(normalize_pair("sol/usdt"), "sol_usdt");
    }

    #[test]
    fn test_normalize_pair_v2() {
        assert_eq!(normalize_pair_v2("btc_idr"), "btcidr");
        assert_eq!(normalize_pair_v2("BTCIDR"), "btcidr");
        assert_eq!(normalize_pair_v2("sol-usdt"), "solusdt");
    }

    #[test]
    fn test_normalize_pair_empty() {
        assert_eq!(normalize_pair(""), "");
    }

    #[test]
    fn test_normalize_pair_single_token() {
        // Pairs that don't match known quote currencies pass through
        assert_eq!(normalize_pair("foobar"), "foobar");
    }

    #[test]
    fn test_normalize_pair_btc_as_quote() {
        // ethbtc -> eth_btc (btc as suffix)
        assert_eq!(normalize_pair("ethbtc"), "eth_btc");
        // btc alone -> stays as btc (base would be empty, so skipped)
        assert_eq!(normalize_pair("btc"), "btc");
    }

    #[test]
    fn test_normalize_pair_idr_not_treated_as_base() {
        // idrbtc -> idr_btc (btc as suffix)
        assert_eq!(normalize_pair("idrbtc"), "idr_btc");
    }

    #[test]
    fn test_flatten_json_to_table_object() {
        let json = json!({"name": "Alice", "age": 30, "city": "NYC"});
        let (headers, rows) = flatten_json_to_table(&json);

        assert_eq!(headers.len(), 3);
        assert_eq!(rows.len(), 1);
        assert!(headers.contains(&"name".into()));
        assert!(headers.contains(&"age".into()));
        assert!(headers.contains(&"city".into()));
    }

    #[test]
    fn test_flatten_json_to_table_array() {
        let json = json!([
            {"id": 1, "val": 100},
            {"id": 2, "val": 200}
        ]);
        let (headers, rows) = flatten_json_to_table(&json);

        assert_eq!(headers.len(), 2);
        assert_eq!(rows.len(), 2);
        assert!(headers.contains(&"id".into()));
        assert!(headers.contains(&"val".into()));
    }

    #[test]
    fn test_flatten_json_to_table_empty_array() {
        let json = json!([]);
        let (headers, rows) = flatten_json_to_table(&json);

        // For empty array, returns single "Value" header and one row
        assert_eq!(headers.len(), 1);
        assert_eq!(rows.len(), 1);
    }

    #[test]
    fn test_flatten_json_to_table_primitive() {
        let json = json!("hello");
        let (headers, rows) = flatten_json_to_table(&json);

        assert_eq!(headers.len(), 1);
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0][0], "hello");
    }

    #[test]
    fn test_flatten_json_to_table_number() {
        let json = json!(42);
        let (_headers, rows) = flatten_json_to_table(&json);

        assert_eq!(rows[0][0], "42");
    }

    #[test]
    fn test_flatten_json_to_table_bool() {
        let json = json!(true);
        let (_headers, rows) = flatten_json_to_table(&json);

        assert_eq!(rows[0][0], "true");
    }

    #[test]
    fn test_first_of_first_key() {
        let val = json!({"a": "1", "b": "2"});
        assert_eq!(first_of(&val, &["a", "b"]), &json!("1"));
    }

    #[test]
    fn test_first_of_second_key() {
        let val = json!({"a": null, "b": "2"});
        assert_eq!(first_of(&val, &["a", "b"]), &json!("2"));
    }

    #[test]
    fn test_first_of_skips_null() {
        let val = json!({"a": null, "b": null});
        assert_eq!(first_of(&val, &["a", "b"]), &serde_json::Value::Null);
    }

    #[test]
    fn test_first_of_skips_empty() {
        let val = json!({"a": "", "b": "value"});
        assert_eq!(first_of(&val, &["a", "b"]), &json!("value"));
    }

    #[test]
    fn test_first_of_skips_null_string() {
        let val = json!({"a": "null", "b": "value"});
        assert_eq!(first_of(&val, &["a", "b"]), &json!("value"));
    }

    #[test]
    fn test_flatten_json_to_table_null() {
        let json = json!(null);
        let (_headers, rows) = flatten_json_to_table(&json);

        assert_eq!(rows[0][0], "");
    }

    #[test]
    fn test_value_to_string_string() {
        let v = json!("hello");
        assert_eq!(value_to_string(&v), "hello");
    }

    #[test]
    fn test_value_to_string_number() {
        let v = json!(42);
        assert_eq!(value_to_string(&v), "42");
    }

    #[test]
    fn test_value_to_string_bool() {
        let v = json!(true);
        assert_eq!(value_to_string(&v), "true");
    }

    #[test]
    fn test_value_to_string_null() {
        let v = json!(null);
        assert_eq!(value_to_string(&v), "");
    }

    #[test]
    fn test_value_to_string_array() {
        let v = json!([1, 2, 3]);
        let result = value_to_string(&v);
        assert!(result.contains("1"));
        assert!(result.contains("2"));
        assert!(result.contains("3"));
    }

    #[test]
    fn test_value_to_string_object() {
        let v = json!({"a": 1});
        let result = value_to_string(&v);
        assert!(result.contains("a") || result.contains("1"));
    }

    #[test]
    fn test_format_timestamp_millis() {
        // 2024-01-01 00:00:00 UTC in millis
        let ts = 1704067200000u64;
        let result = format_timestamp(ts, true);
        assert!(result.contains("2024") || result.contains("01-01"));
    }

    #[test]
    fn test_format_timestamp_seconds() {
        // 2024-01-01 00:00:00 UTC in seconds
        let ts = 1704067200u64;
        let result = format_timestamp(ts, false);
        assert!(result.contains("2024") || result.contains("01-01"));
    }

    #[test]
    fn test_format_timestamp_invalid() {
        let result = format_timestamp(0, false);
        // Timestamp 0 is valid (1970-01-01), so it returns a formatted date
        assert!(result.contains("1970") || result.contains("01-01"));
    }

    #[test]
    fn test_extract_pairs() {
        // Test object format
        let data_obj = json!({
            "btcidr": {
                "traded_currency": "btc",
                "base_currency": "idr",
                "symbol": "BTC/IDR"
            },
            "ethidr": {
                "traded_currency": "eth",
                "base_currency": "idr",
                "symbol": "ETH/IDR"
            }
        });

        let pairs_obj = extract_pairs(&data_obj);
        assert_eq!(pairs_obj.len(), 2);
        assert!(pairs_obj.iter().any(|(k, _)| k == "btcidr"));
        assert!(pairs_obj.iter().any(|(k, _)| k == "ethidr"));

        // Test array format
        let data_arr = json!([
            {
                "id": "btcidr",
                "traded_currency": "btc",
                "base_currency": "idr",
                "symbol": "BTCIDR"
            },
            {
                "id": "ethidr",
                "traded_currency": "eth",
                "base_currency": "idr",
                "symbol": "ETHIDR"
            }
        ]);
        let pairs_arr = extract_pairs(&data_arr);
        assert_eq!(pairs_arr.len(), 2);
        assert!(pairs_arr.iter().any(|(k, _)| k == "btcidr"));
        assert!(pairs_arr.iter().any(|(k, _)| k == "ethidr"));
    }

    #[test]
    fn test_extract_pairs_with_base_currency() {
        let data = json!({
            "btcidr": {
                "tradedCurrency": "btc",
                "baseCurrency": "idr"
            }
        });

        let pairs = extract_pairs(&data);
        assert_eq!(pairs.len(), 1);
        assert!(pairs[0].1.contains("btc"));
        assert!(pairs[0].1.contains("idr"));
    }

    #[test]
    fn test_extract_pairs_empty() {
        let data = json!({});
        let pairs = extract_pairs(&data);
        assert!(pairs.is_empty());
    }

    #[test]
    fn test_extract_pairs_not_object() {
        let data = json!([]);
        let pairs = extract_pairs(&data);
        assert!(pairs.is_empty());
    }

    #[tokio::test]
    async fn test_fetch_public_ws_token_default() {
        let client = IndodaxClient::new(None).unwrap();
        let token = fetch_public_ws_token(&client).await.unwrap();
        // Since we are not in an environment where the dynamic URL necessarily works,
        // it should at least return the default token if it fails.
        assert!(!token.is_empty());
    }
}
