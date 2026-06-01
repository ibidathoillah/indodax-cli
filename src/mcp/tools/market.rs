use rmcp::model::{CallToolResult, Tool};
use serde_json::Value;

use super::IndodaxMcp;
use crate::commands::helpers;

pub fn market_tools() -> Vec<Tool> {
    vec![
        IndodaxMcp::tool_def(
            "server_time",
            "Retrieve the current Indodax exchange server time in Unix timestamp format (milliseconds). This tool is essential for accurately calculating request signatures, synchronizing local trade logs, and verifying that your system time is aligned with the exchange's matching engine.",
            serde_json::json!({}),
            vec![],
        ),
        IndodaxMcp::tool_def(
            "ticker",
            "[PUBLIC READ-ONLY] Obtain the current REST ticker snapshot for one Indodax trading pair. Returns JSON text with last price, buy/sell prices, 24h high/low, and volume. Use this for normal price checks; use ticker_all for all pairs or ws_snapshot_ticker when low-latency WebSocket data matters.",
            serde_json::json!({
                "pair": IndodaxMcp::str_param("The specific trading pair to query (e.g., 'btc_idr', 'eth_idr', 'usdt_idr'). The standard format is base_quote in lowercase.", false, Some("btc_idr"))
            }),
            vec![],
        ),
        IndodaxMcp::tool_def(
            "ticker_all",
            "[PUBLIC READ-ONLY] Get current REST ticker snapshots for every supported Indodax trading pair in one call. Returns JSON text keyed by pair with price and 24h statistics. Use this for exchange-wide scans; use ticker when you only need one pair.",
            serde_json::json!({}),
            vec![],
        ),
        IndodaxMcp::tool_def(
            "pairs",
            "List every trading pair currently active on Indodax along with their detailed metadata. Includes crucial information such as asset symbols, full names, minimum trade amounts, and fee structures. Use this tool to discover new markets and ensure your order parameters comply with exchange minimums.",
            serde_json::json!({}),
            vec![],
        ),
        IndodaxMcp::tool_def(
            "summaries",
            "Retrieve a high-level statistical summary for all trading pairs over both 24-hour and 7-day windows. Includes price change percentages and aggregate volume data. Ideal for identifying long-term trends and broader market sentiment across the entire exchange.",
            serde_json::json!({}),
            vec![],
        ),
        IndodaxMcp::tool_def(
            "orderbook",
            "[PUBLIC READ-ONLY] Fetch the current REST order book depth for one trading pair. Returns JSON text containing bid and ask price levels with volumes, sorted by price. Use this for resting liquidity and slippage checks; use trades for recent executions or ws_snapshot_book for a one-shot WebSocket depth snapshot.",
            serde_json::json!({
                "pair": IndodaxMcp::str_param("The trading pair for which to retrieve order book depth (e.g., 'btc_idr').", false, Some("btc_idr"))
            }),
            vec![],
        ),
        IndodaxMcp::tool_def(
            "trades",
            "[PUBLIC READ-ONLY] Retrieve recent public executions for one Indodax trading pair. Returns JSON text with trade price, amount, timestamp, and side. Use this to inspect tape activity; use orderbook for resting bids/asks or trade_history for authenticated fills from your own account.",
            serde_json::json!({
                "pair": IndodaxMcp::str_param("The trading pair to retrieve recent trade history for (e.g., 'btc_idr').", false, Some("btc_idr"))
            }),
            vec![],
        ),
        IndodaxMcp::tool_def(
            "ohlc",
            "[PUBLIC READ-ONLY] Retrieve historical OHLCV candles from the Indodax TradingView v2 endpoint. Returns JSON text with candle arrays for charting and technical analysis. If from/to are omitted the handler uses the last 24 hours; Unix timestamps must be seconds, though millisecond-looking values are normalized. Use ticker for the latest quote instead of historical candles.",
            serde_json::json!({
                "symbol": IndodaxMcp::str_param("The trading pair symbol in v2 format (e.g., 'BTCIDR' or 'BTC_IDR').", false, Some("BTCIDR")),
                "timeframe": IndodaxMcp::str_param(
                    "The interval for each candle in minutes. Common values include '1', '5', '15', '30', '60' (hourly), and '1440' (daily).",
                    false,
                    Some("60"),
                ),
                "from": IndodaxMcp::num_param("The start of the historical range as a Unix timestamp in seconds.", false),
                "to": IndodaxMcp::num_param("The end of the historical range as a Unix timestamp in seconds.", false),
            }),
            vec![],
        ),
        IndodaxMcp::tool_def(
            "price_increments",
            "Retrieve the current minimum price increments (tick sizes) allowed for every trading pair. This metadata is essential for placing limit orders that satisfy the exchange's precision requirements and avoiding 'invalid price' errors.",
            serde_json::json!({}),
            vec![],
        ),
        IndodaxMcp::tool_def(
            "orderbook_grouped",
            "[PUBLIC READ-ONLY] Fetch the order book and group price levels by a specified interval. This is useful for identifying significant support and resistance zones (walls) without being distracted by granular noise. Returns total volume per clustered price bracket.",
            serde_json::json!({
                "pair": IndodaxMcp::str_param("The trading pair to query (e.g., 'btc_idr').", true, Some("btc_idr")),
                "grouping": IndodaxMcp::num_param("The price interval to group by (e.g., 100000 for 100k IDR steps).", false),
                "depth": IndodaxMcp::num_param("The number of grouped levels to show per side. Default is 10.", false)
            }),
            vec!["pair"],
        ),
        IndodaxMcp::tool_def(
            "spreads",
            "[PUBLIC READ-ONLY] Calculate the current bid/ask spread for a trading pair. Returns absolute spread value and percentage. High spreads indicate low liquidity or high volatility.",
            serde_json::json!({
                "pair": IndodaxMcp::str_param("The trading pair to query (e.g., 'btc_idr').", true, Some("btc_idr"))
            }),
            vec!["pair"],
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

    pub async fn handle_orderbook_grouped(&self, pair: &str, grouping: Option<f64>, depth: Option<f64>) -> CallToolResult {
        let grouping = grouping.unwrap_or(100000.0);
        let depth = depth.unwrap_or(10.0) as usize;
        match crate::commands::market::orderbook_grouped(&self.client, pair, grouping, depth).await {
            Ok(output) => Self::json_result(output.data),
            Err(e) => Self::error_result(format!("Failed to get grouped orderbook: {}", e)),
        }
    }

    pub async fn handle_spreads(&self, pair: &str) -> CallToolResult {
        match crate::commands::market::spreads(&self.client, pair).await {
            Ok(output) => Self::json_result(output.data),
            Err(e) => Self::error_result(format!("Failed to calculate spreads: {}", e)),
        }
    }
}
