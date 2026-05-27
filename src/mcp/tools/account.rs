use std::collections::HashMap;

use rmcp::model::{CallToolResult, Tool};
use serde_json::Value;

use super::IndodaxMcp;
use crate::commands::helpers;

pub fn account_tools() -> Vec<Tool> {
    vec![
        IndodaxMcp::tool_def(
            "account_info",
            "[REQUIRES AUTH] Get account information including balances",
            serde_json::json!({}),
            vec![],
        ),
        IndodaxMcp::tool_def(
            "balance",
            "[REQUIRES AUTH] Get wallet balances (non-zero only)",
            serde_json::json!({}),
            vec![],
        ),
        IndodaxMcp::tool_def(
            "open_orders",
            "[REQUIRES AUTH] List open orders",
            serde_json::json!({
                "pair": IndodaxMcp::str_param("Filter by trading pair (optional)", false, None)
            }),
            vec![],
        ),
        IndodaxMcp::tool_def(
            "order_history",
            "[REQUIRES AUTH] Get order history",
            serde_json::json!({
                "symbol": IndodaxMcp::str_param(
                    "Trading pair symbol, e.g. btc_idr",
                    false,
                    Some("btc_idr"),
                ),
                "limit": IndodaxMcp::num_param("Maximum number of orders to return", false),
            }),
            vec![],
        ),
        IndodaxMcp::tool_def(
            "trade_history",
            "[REQUIRES AUTH] Get trade fill history",
            serde_json::json!({
                "symbol": IndodaxMcp::str_param(
                    "Trading pair symbol, e.g. btc_idr",
                    false,
                    Some("btc_idr"),
                ),
                "limit": IndodaxMcp::num_param("Maximum number of trades to return", false),
            }),
            vec![],
        ),
        IndodaxMcp::tool_def(
            "get_order",
            "[REQUIRES AUTH] Get order details by order ID",
            serde_json::json!({
                "order_id": IndodaxMcp::num_param("Order ID", true),
                "pair": IndodaxMcp::str_param("Trading pair, e.g. btc_idr", true, None),
            }),
            vec!["order_id", "pair"],
        ),
        IndodaxMcp::tool_def(
            "trans_history",
            "[REQUIRES AUTH] Get deposit and withdrawal transaction history",
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
