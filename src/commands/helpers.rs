use chrono::DateTime;
use serde_json::Value;

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
    if let Some(dt) = DateTime::from_timestamp(ts_sec as i64, 0) {
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
    for quote in &["idr", "usdt", "btc"] {
        if let Some(base) = pair.strip_suffix(quote) {
            if !base.is_empty() {
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
