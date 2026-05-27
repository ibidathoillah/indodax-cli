use std::collections::HashMap;

use rmcp::model::{CallToolResult, Tool};
use serde_json::Value;

use super::IndodaxMcp;
use crate::commands::helpers;

pub fn account_tools() -> Vec<Tool> {
    vec![
        IndodaxMcp::tool_def(
            "account_info",
            "Retrieve comprehensive information about your Indodax account. This tool provides a complete snapshot of your identity verification status, transaction permissions, and all asset balances, including both available funds and those currently locked in open orders. Use this to get a high-level overview of your account state.",
            serde_json::json!({}),
            vec![],
        ),
        IndodaxMcp::tool_def(
            "balance",
            "Get a clean list of all non-zero asset balances for your authenticated Indodax account. This tool simplifies portfolio tracking by filtering out inactive assets, showing you exactly what you hold and in what quantities (e.g., IDR, BTC, ETH).",
            serde_json::json!({}),
            vec![],
        ),
        IndodaxMcp::tool_def(
            "open_orders",
            "Fetch a real-time list of all currently active and unfilled orders for your account. This is essential for monitoring your active market commitments. You can optionally filter by a specific trading pair (e.g., 'btc_idr') to focus on a particular market segment.",
            serde_json::json!({
                "pair": IndodaxMcp::str_param("Optional: The trading pair to filter results by (e.g., 'btc_idr', 'eth_idr'). If omitted, all open orders across all pairs are returned.", false, None)
            }),
            vec![],
        ),
        IndodaxMcp::tool_def(
            "order_history",
            "Retrieve a detailed historical record of all orders (both filled and cancelled) placed on a specific trading pair. This tool is vital for auditing your trading performance, calculating cost basis, and reconciling trade activity over time.",
            serde_json::json!({
                "symbol": IndodaxMcp::str_param(
                    "The trading pair symbol to query (e.g., 'btc_idr', 'eth_idr'). Supports both underscore and compact formats.",
                    false,
                    Some("btc_idr"),
                ),
                "limit": IndodaxMcp::num_param("The maximum number of historical records to return. Default is 100. Use this to control the data volume for high-frequency accounts.", false),
            }),
            vec![],
        ),
        IndodaxMcp::tool_def(
            "trade_history",
            "Get a precise history of executed trade fills for your account. Unlike order history, this tool focuses on actual transactions, providing specific execution prices, quantities, and the trading fees paid for each fill on a given pair.",
            serde_json::json!({
                "symbol": IndodaxMcp::str_param(
                    "The trading pair symbol to query (e.g., 'btc_idr', 'eth_idr').",
                    false,
                    Some("btc_idr"),
                ),
                "limit": IndodaxMcp::num_param("The maximum number of execution records to return. Default is 100.", false),
            }),
            vec![],
        ),
        IndodaxMcp::tool_def(
            "get_order",
            "Retrieve detailed, granular information for a specific order using its unique Order ID. This is the preferred tool for checking the final status of a recently placed order or investigating a specific historical transaction.",
            serde_json::json!({
                "order_id": IndodaxMcp::num_param("The unique numerical ID assigned to the order by the exchange.", true),
                "pair": IndodaxMcp::str_param("The trading pair the order was placed on (e.g., 'btc_idr'). This is required for efficient lookup.", true, None),
            }),
            vec!["order_id", "pair"],
        ),
        IndodaxMcp::tool_def(
            "trans_history",
            "Retrieve the complete history of all deposit and withdrawal transactions (both fiat IDR and cryptocurrency) for your account. Includes transaction IDs, destination addresses, fees, and current processing status.",
            serde_json::json!({}),
            vec![],
        ),
    ]
}

fn validate_limit(limit: Option<f64>) -> Result<u32, String> {
    match limit {
        Some(v) if v.fract() != 0.0 || v <= 0.0 => {
            Err(format!("limit must be a positive whole number, got {}", v))
        }
        Some(v) => Ok(v as u32),
        None => Ok(100),
    }
}

impl IndodaxMcp {
    pub async fn handle_account_info(&self) -> CallToolResult {
        match self.get_account_info().await {
            Ok(data) => Self::json_result(data),
            Err(e) => Self::error_from_indodax(&e),
        }
    }

    pub async fn handle_balance(&self) -> CallToolResult {
        match self.get_account_info().await {
            Ok(data) => match data.get("balance") {
                Some(balance) => Self::json_result(balance.clone()),
                None => Self::error_result("API response missing 'balance' field".into()),
            },
            Err(e) => Self::error_from_indodax(&e),
        }
    }

    pub async fn handle_open_orders(&self, pair: Option<&str>) -> CallToolResult {
        let mut params = HashMap::new();
        if let Some(p) = pair {
            params.insert("pair".to_string(), p.to_string());
        }
        match self
            .client
            .private_post_v1::<Value>("openOrders", &params)
            .await
        {
            Ok(data) => Self::json_result(data),
            Err(e) => Self::error_from_indodax(&e),
        }
    }

    pub async fn handle_order_history(&self, symbol: &str, limit: Option<f64>) -> CallToolResult {
        let now = helpers::now_millis();
        let start = now - helpers::ONE_DAY_MS;
        let limit_val = match validate_limit(limit) {
            Ok(v) => v,
            Err(e) => return Self::validation_error_result(e),
        };

        let mut params = HashMap::new();
        params.insert("symbol".to_string(), helpers::normalize_pair_v2(symbol));
        params.insert("limit".to_string(), limit_val.max(10).to_string());
        params.insert("startTime".to_string(), start.to_string());
        params.insert("endTime".to_string(), now.to_string());

        match self
            .client
            .private_get_v2::<Value>("/api/v2/order/histories", &params)
            .await
        {
            Ok(data) => Self::json_result(data),
            Err(e) => Self::error_from_indodax(&e),
        }
    }

    pub async fn handle_trade_history(&self, symbol: &str, limit: Option<f64>) -> CallToolResult {
        let now = helpers::now_millis();
        let start = now - helpers::ONE_DAY_MS;
        let limit_val = match validate_limit(limit) {
            Ok(v) => v,
            Err(e) => return Self::validation_error_result(e),
        };

        let mut params = HashMap::new();
        params.insert("symbol".to_string(), helpers::normalize_pair_v2(symbol));
        params.insert("limit".to_string(), limit_val.max(10).to_string());
        params.insert("startTime".to_string(), start.to_string());
        params.insert("endTime".to_string(), now.to_string());

        match self
            .client
            .private_get_v2::<Value>("/api/v2/myTrades", &params)
            .await
        {
            Ok(data) => Self::json_result(data),
            Err(e) => Self::error_from_indodax(&e),
        }
    }

    pub async fn handle_get_order(&self, order_id: f64, pair: &str) -> CallToolResult {
        let mut params = HashMap::new();
        params.insert("order_id".to_string(), (order_id as u64).to_string());
        params.insert("pair".to_string(), pair.to_string());

        match self
            .client
            .private_post_v1::<Value>("getOrder", &params)
            .await
        {
            Ok(data) => Self::json_result(data),
            Err(e) => Self::error_from_indodax(&e),
        }
    }

    pub async fn handle_trans_history(&self) -> CallToolResult {
        match self
            .client
            .private_post_v1::<Value>("transHistory", &HashMap::new())
            .await
        {
            Ok(data) => Self::json_result(data),
            Err(e) => Self::error_from_indodax(&e),
        }
    }
}
