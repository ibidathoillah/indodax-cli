use std::collections::HashMap;

use rmcp::model::{CallToolResult, Tool};
use serde_json::Value;

use super::IndodaxMcp;
use crate::commands::helpers;

pub fn account_tools() -> Vec<Tool> {
    vec![
        IndodaxMcp::tool_def(
            "account_info",
            "[REQUIRES AUTH, READ-ONLY] Retrieve the full private account snapshot from Indodax. Returns JSON text with identity/status fields, API permissions, balances, locked funds, and server timestamp when provided by the exchange. Use this for a complete account overview; use balance when you only need non-zero wallet balances.",
            serde_json::json!({}),
            vec![],
        ),
        IndodaxMcp::tool_def(
            "balance",
            "[REQUIRES AUTH, READ-ONLY] Return only the non-zero wallet balances from your Indodax account as JSON text. This filters the broader account_info response for portfolio tracking. Use account_info instead when you need permissions, locked balances, or account metadata.",
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
            "[REQUIRES AUTH, READ-ONLY] Retrieve historical orders for one trading pair, including filled, cancelled, and other exchange-reported order states. Returns JSON text from the private orderHistory API. Use this for order lifecycle auditing; use trade_history when you only need executed fills and fees.",
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
            "[REQUIRES AUTH, READ-ONLY] Retrieve executed trade fills for your account on one trading pair. Returns JSON text with execution prices, quantities, timestamps, and exchange fee fields when available. Use this for realized trading activity; use order_history when you need cancelled or still-open order records too.",
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
        IndodaxMcp::tool_def(
            "equity_snap",
            "Record a current portfolio equity snapshot in Indonesian Rupiah (IDR). This calculates the total value of all your holdings based on current market prices and saves it to a local history file. This is useful for tracking your portfolio growth over time.",
            serde_json::json!({}),
            vec![],
        ),
        IndodaxMcp::tool_def(
            "equity_history",
            "Retrieve the history of recorded portfolio equity snapshots. This allows you to see how your total account value has changed over time. You can limit the number of snapshots returned.",
            serde_json::json!({
                "limit": IndodaxMcp::num_param("The maximum number of historical snapshots to return. Default is 20.", false),
            }),
            vec![],
        ),
        IndodaxMcp::tool_def(
            "list_downline",
            "List all downline users associated with your Indodax account (Affiliate program). Returns user IDs and registration dates for your referrals.",
            serde_json::json!({}),
            vec![],
        ),
        IndodaxMcp::tool_def(
            "check_downline",
            "Check if a specific email address exists in your affiliate downline. This helps you verify if a user has registered using your referral link.",
            serde_json::json!({
                "email": IndodaxMcp::str_param("The email address to check.", true, None),
            }),
            vec!["email"],
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

    pub async fn handle_equity_snap(&self) -> CallToolResult {
        match crate::commands::account::equity_snap(&self.client).await {
            Ok(output) => Self::json_result(output.data),
            Err(e) => Self::error_result(format!("Failed to record equity snapshot: {}", e)),
        }
    }

    pub async fn handle_equity_history(&self, limit: Option<f64>) -> CallToolResult {
        let limit_val = match validate_limit(limit) {
            Ok(v) => v,
            Err(e) => return Self::validation_error_result(e),
        };

        match crate::commands::account::equity_history(limit_val as usize, false) {
            Ok(output) => Self::json_result(output.data),
            Err(e) => Self::error_result(format!("Failed to retrieve equity history: {}", e)),
        }
    }

    pub async fn handle_list_downline(&self) -> CallToolResult {
        match self
            .client
            .private_post_v1::<Value>("listDownline", &HashMap::new())
            .await
        {
            Ok(data) => Self::json_result(data),
            Err(e) => Self::error_from_indodax(&e),
        }
    }

    pub async fn handle_check_downline(&self, email: &str) -> CallToolResult {
        let mut params = HashMap::new();
        params.insert("email".into(), email.to_string());

        match self
            .client
            .private_post_v1::<Value>("checkDownline", &params)
            .await
        {
            Ok(data) => Self::json_result(data),
            Err(e) => Self::error_from_indodax(&e),
        }
    }
}
