use rmcp::model::{CallToolResult, Tool};
use serde_json::Value;

use super::IndodaxMcp;
use crate::commands::helpers;

pub fn market_tools() -> Vec<Tool> {
    vec![
        IndodaxMcp::tool_def(
            "server_time",
            "Get the current Indodax server time",
            serde_json::json!({}),
            vec![],
        ),
        IndodaxMcp::tool_def(
            "ticker",
            "Get ticker for a trading pair",
            serde_json::json!({
                "pair": IndodaxMcp::str_param("Trading pair, e.g. btc_idr", false, Some("btc_idr"))
            }),
            vec![],
        ),
        IndodaxMcp::tool_def(
            "ticker_all",
            "Get tickers for all trading pairs",
            serde_json::json!({}),
            vec![],
        ),
        IndodaxMcp::tool_def(
            "pairs",
            "List all available trading pairs",
            serde_json::json!({}),
            vec![],
        ),
        IndodaxMcp::tool_def(
            "summaries",
            "Get 24h and 7d market summaries for all pairs",
            serde_json::json!({}),
            vec![],
        ),
        IndodaxMcp::tool_def(
            "orderbook",
            "Get order book depth for a trading pair",
            serde_json::json!({
                "pair": IndodaxMcp::str_param("Trading pair, e.g. btc_idr", false, Some("btc_idr"))
            }),
            vec![],
        ),
        IndodaxMcp::tool_def(
            "trades",
            "Get recent trades for a trading pair",
            serde_json::json!({
                "pair": IndodaxMcp::str_param("Trading pair, e.g. btc_idr", false, Some("btc_idr"))
            }),
            vec![],
        ),
        IndodaxMcp::tool_def(
            "ohlc",
            "Get OHLCV candle data",
            serde_json::json!({
                "symbol": IndodaxMcp::str_param("Trading pair symbol, e.g. BTCIDR", false, Some("BTCIDR")),
                "timeframe": IndodaxMcp::str_param(
                    "Candle timeframe in minutes, e.g. 60",
                    false,
                    Some("60"),
                ),
                "from": IndodaxMcp::num_param("Start timestamp (seconds)", false),
                "to": IndodaxMcp::num_param("End timestamp (seconds)", false),
            }),
            vec![],
        ),
        IndodaxMcp::tool_def(
            "price_increments",
            "Get price increments (tick sizes) for all pairs",
            serde_json::json!({}),
            vec![],
        ),
    ]
}

impl IndodaxMcp {
    pub async fn handle_server_time(&self) -> CallToolResult {
        match self.client.public_get::<Value>("/api/server_time").await {
            Ok(data) => Self::json_result(data),
            Err(e) => Self::error_from_indodax(&e),
        }
    }

    pub async fn handle_ticker(&self, pair: &str) -> CallToolResult {
        let path = format!("/api/ticker/{}", pair);
        match self.client.public_get::<Value>(&path).await {
            Ok(data) => Self::json_result(data),
            Err(e) => Self::error_from_indodax(&e),
        }
    }

    pub async fn handle_ticker_all(&self) -> CallToolResult {
        match self.client.public_get::<Value>("/api/ticker_all").await {
            Ok(data) => Self::json_result(data),
            Err(e) => Self::error_from_indodax(&e),
        }
    }

    pub async fn handle_pairs(&self) -> CallToolResult {
        match self.client.public_get::<Value>("/api/pairs").await {
            Ok(data) => Self::json_result(data),
            Err(e) => Self::error_from_indodax(&e),
        }
    }

    pub async fn handle_summaries(&self) -> CallToolResult {
        match self.client.public_get::<Value>("/api/summaries").await {
            Ok(data) => Self::json_result(data),
            Err(e) => Self::error_from_indodax(&e),
        }
    }

    pub async fn handle_orderbook(&self, pair: &str) -> CallToolResult {
        let path = format!("/api/depth/{}", pair);
        match self.client.public_get::<Value>(&path).await {
            Ok(data) => Self::json_result(data),
            Err(e) => Self::error_from_indodax(&e),
        }
    }

    pub async fn handle_trades(&self, pair: &str) -> CallToolResult {
        let path = format!("/api/trades/{}", pair);
        match self.client.public_get::<Value>(&path).await {
            Ok(data) => Self::json_result(data),
            Err(e) => Self::error_from_indodax(&e),
        }
    }

    pub async fn handle_ohlc(
        &self,
        symbol: &str,
        timeframe: &str,
        from: Option<f64>,
        to: Option<f64>,
    ) -> CallToolResult {
        fn normalize_ts(v: f64, label: &str) -> u64 {
            let mut ts = v as u64;
            if ts > 1_000_000_000_000 {
                eprintln!("[MCP] Warning: {} timestamp ({}) looks like milliseconds. Converting to seconds.", label, ts);
                ts /= 1000;
            }
            ts
        }

        // Normalize symbol to lowercase for v2 API consistency
        let symbol = symbol.to_lowercase();
        let now_secs = crate::commands::helpers::now_millis() / 1000;
        let from_val = from
            .map(|v| normalize_ts(v, "from").to_string())
            .unwrap_or_else(|| (now_secs.saturating_sub(helpers::ONE_DAY_SECS)).to_string());
        let to_val = to
            .map(|v| normalize_ts(v, "to").to_string())
            .unwrap_or_else(|| now_secs.to_string());

        match self
            .client
            .public_get_v2::<Value>(
                "/tradingview/history_v2",
                &[
                    ("symbol", &symbol),
                    ("tf", timeframe),
                    ("from", &from_val),
                    ("to", &to_val),
                ],
            )
            .await
        {
            Ok(data) => Self::json_result(data),
            Err(e) => Self::error_from_indodax(&e),
        }
    }

    pub async fn handle_price_increments(&self) -> CallToolResult {
        match self
            .client
            .public_get::<Value>("/api/price_increments")
            .await
        {
            Ok(data) => Self::json_result(data),
            Err(e) => Self::error_from_indodax(&e),
        }
    }
}
