use chrono::DateTime;
use serde_json::Value;
use crate::client::IndodaxClient;
use crate::errors::IndodaxError;

pub const PUBLIC_WS_TOKEN_URL: &str = "https://indodax.com/api/ws/v1/generate_token";

pub async fn fetch_public_ws_token(client: &IndodaxClient) -> Result<String, anyhow::Error> {
    let resp = client.http_client().get(PUBLIC_WS_TOKEN_URL).send().await
        .map_err(|e| anyhow::anyhow!("Failed to fetch WebSocket token: {}", e))?;
    let text = resp.text().await
        .map_err(|e| anyhow::anyhow!("Failed to read token response: {}", e))?;
    let val: serde_json::Value = serde_json::from_str(&text)
        .map_err(|e| anyhow::anyhow!("Invalid token response: {}", e))?;
    val.get("token").and_then(|t| t.as_str()).map(|t| t.to_string())
        .or_else(|| val.get("data").and_then(|d| d.get("token")).and_then(|t| t.as_str()).map(|t| t.to_string()))
        .ok_or_else(|| anyhow::anyhow!("No token in response: {}", text))
}

pub const ONE_DAY_MS: u64 = 24 * 60 * 60 * 1000;
pub const ONE_DAY_SECS: u64 = 24 * 60 * 60;

pub fn flatten_json_to_table(json: &serde_json::Value) -> (Vec<String>, Vec<Vec<String>>) {
    match json {
        serde_json::Value::Object(map) => {
            let mut headers: Vec<String> = map.keys().cloned().collect();
            headers.sort();
            let row: Vec<String> = headers
                .iter()
                .map(|k| value_to_string(&map[k]))
                .collect();
            (headers, vec![row])
        }
        serde_json::Value::Array(arr) if !arr.is_empty() => {
            // Collect all unique keys from all array elements
            let mut all_keys: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
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
                    .map(|item| {
                        headers
                            .iter()
                            .map(|k| value_to_string(&item[k]))
                            .collect()
                    })
                    .collect();
                (headers, rows)
            } else {
                (vec!["Value".into()], arr.iter().map(|v| vec![value_to_string(v)]).collect())
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
        serde_json::Value::Object(_) => serde_json::to_string(v).unwrap_or_default(),
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
    let pair = pair.to_lowercase().replace('-', "_");
    if pair.contains('_') || pair.is_empty() {
        return pair;
    }
    let quote_currencies = ["usdt", "idr", "btc"];
    for quote in &quote_currencies {
        if let Some(base) = pair.strip_suffix(quote) {
            if !base.is_empty() && !quote_currencies.contains(&base) {
                return format!("{}_{}", base, quote);
            }
        }
    }
    pair
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
                    .or_else(|| order_val.get("market"))
                    .or_else(|| order_val.get("symbol"))
                    .unwrap_or(&serde_json::Value::Null),
            );
            let order_type = order_val
                .get("type")
                .or_else(|| order_val.get("order_type"))
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();

            let mut cancel_params = HashMap::new();
            cancel_params.insert("order_id".to_string(), order_id.clone());
            cancel_params.insert("pair".to_string(), order_pair);
            cancel_params.insert("type".to_string(), order_type);
            match client
                .private_post_v1::<serde_json::Value>("cancelOrder", &cancel_params)
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
                let base = obj.get("traded_currency")
                    .or_else(|| obj.get("tradedCurrency"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let quote = obj.get("base_currency")
                    .or_else(|| obj.get("baseCurrency"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let symbol = obj.get("symbol").or_else(|| obj.get("ticker_id")).and_then(|v| v.as_str()).unwrap_or("");
                pairs.push((key.clone(), format!("{}/{} ({})", base, quote, symbol)));
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
        assert_eq!(normalize_pair("btc_idr"), "btcidr");
        assert_eq!(normalize_pair("eth_btc"), "ethbtc");
        assert_eq!(normalize_pair("usdt_idr"), "usdtidr");
    }

    #[test]
    fn test_normalize_pair_no_underscore() {
        assert_eq!(normalize_pair("btcidr"), "btcidr");
        assert_eq!(normalize_pair("ethidr"), "ethidr");
        assert_eq!(normalize_pair("ethbtc"), "ethbtc");
        assert_eq!(normalize_pair("solusdt"), "solusdt");
    }

    #[test]
    fn test_normalize_pair_uppercase() {
        assert_eq!(normalize_pair("BTC_IDR"), "btcidr");
        assert_eq!(normalize_pair("BTCIDR"), "btcidr");
        assert_eq!(normalize_pair("ETH_BTC"), "ethbtc");
    }

    #[test]
    fn test_normalize_pair_dash_separator() {
        assert_eq!(normalize_pair("btc-idr"), "btcidr");
        assert_eq!(normalize_pair("ETH-IDR"), "ethidr");
        assert_eq!(normalize_pair("sol-usdt"), "solusdt");
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
        // idrbtc should NOT produce idr_btc since idr is never a base
        assert_eq!(normalize_pair("idrbtc"), "idrbtc");
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
        let data = json!({
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
        
        let pairs = extract_pairs(&data);
        assert_eq!(pairs.len(), 2);
        assert!(pairs[0].0 == "btcidr" || pairs[0].0 == "ethidr");
        assert!(pairs.iter().any(|(k, _)| k == "btcidr"));
        assert!(pairs.iter().any(|(k, _)| k == "ethidr"));
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
}
