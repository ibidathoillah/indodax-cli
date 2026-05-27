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
            "Obtain a real-time market data snapshot for a specific trading pair. Returns critical price action metrics including the last traded price, current best bid and ask (buy/sell) prices, 24-hour high and low, and total 24-hour trading volume. This is the primary tool for price discovery and basic market monitoring.",
            serde_json::json!({
                "pair": IndodaxMcp::str_param("The specific trading pair to query (e.g., 'btc_idr', 'eth_idr', 'usdt_idr'). The standard format is base_quote in lowercase.", false, Some("btc_idr"))
            }),
            vec![],
        ),
        IndodaxMcp::tool_def(
            "ticker_all",
            "Get real-time ticker data for every supported trading pair on the Indodax exchange in a single request. This provides a comprehensive view of global market activity, allowing you to quickly identify volume spikes, significant price moves, and arbitrage opportunities across all listed assets.",
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
            "Fetch the current order book depth for a specific trading pair. Returns a detailed list of open buy orders (bids) and sell orders (asks) sorted by price. This tool is fundamental for analyzing market liquidity, identifying support/resistance levels, and estimating the slippage for large orders.",
            serde_json::json!({
                "pair": IndodaxMcp::str_param("The trading pair for which to retrieve order book depth (e.g., 'btc_idr').", false, Some("btc_idr"))
            }),
            vec![],
        ),
        IndodaxMcp::tool_def(
            "trades",
            "Retrieve the list of the most recent public trades executed for a specific trading pair. For each trade, it provides the execution price, the amount of the asset traded, the exact timestamp, and the trade side. Essential for verifying real-time market activity and trade flow.",
            serde_json::json!({
                "pair": IndodaxMcp::str_param("The trading pair to retrieve recent trade history for (e.g., 'btc_idr').", false, Some("btc_idr"))
            }),
            vec![],
        ),
        IndodaxMcp::tool_def(
            "ohlc",
            "Retrieve historical Open, High, Low, Close, and Volume (OHLCV) candle data. This tool is designed for technical analysis, allowing you to fetch historical price action over custom timeframes and date ranges to power charting and algorithmic strategies.",
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
