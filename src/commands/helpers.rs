use chrono::DateTime;

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
            let first = &arr[0];
            if let serde_json::Value::Object(first_map) = first {
                let mut headers: Vec<String> = first_map.keys().cloned().collect();
                headers.sort();
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
