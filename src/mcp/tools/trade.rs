use std::collections::HashMap;

use rmcp::model::{CallToolResult, Tool};
use serde_json::Value;

use super::IndodaxMcp;

pub fn trade_tools() -> Vec<Tool> {
    vec![
        IndodaxMcp::tool_def(
            "buy_order",
            "Place a new buy order on the Indodax exchange. This tool supports Limit orders (specify price), Market orders (omit price), and Stop-Limit orders (specify price and stop_price). It is a 'dangerous' operation that requires the 'acknowledged' parameter to be set to true.",
            serde_json::json!({
                "pair": IndodaxMcp::str_param("The trading pair you wish to buy (e.g., 'btc_idr', 'eth_idr').", true, None),
                "idr": IndodaxMcp::num_param("The total amount of Indonesian Rupiah (IDR) you want to spend.", true),
                "price": IndodaxMcp::num_param("The limit price per unit you are willing to pay. Required for limit and stoplimit orders.", false),
                "stop_price": IndodaxMcp::num_param("The trigger price for stop-limit orders. If provided, the order_type will be 'stoplimit'.", false),
                "client_order_id": IndodaxMcp::str_param("Optional: A custom unique identifier for the order.", false, None),
                "acknowledged":
                    IndodaxMcp::bool_param("Security confirmation: This must be explicitly set to true."),
            }),
            vec!["pair", "idr", "acknowledged"],
        ),
        IndodaxMcp::tool_def(
            "sell_order",
            "Place a new sell order on the Indodax exchange. Supports Limit, Market, and Stop-Limit orders. This tool requires the 'acknowledged' parameter for safety.",
            serde_json::json!({
                "pair": IndodaxMcp::str_param("The trading pair you wish to sell (e.g., 'btc_idr').", true, None),
                "price": IndodaxMcp::num_param("The limit price per unit you want to receive. Required for limit and stoplimit orders.", false),
                "amount": IndodaxMcp::num_param("The exact quantity of the base asset (e.g., 0.005 for BTC) you wish to sell.", true),
                "stop_price": IndodaxMcp::num_param("The trigger price for stop-limit orders. If provided, the order_type will be 'stoplimit'.", false),
                "client_order_id": IndodaxMcp::str_param("Optional: A custom unique identifier for the order.", false, None),
                "order_type":
                    IndodaxMcp::str_param("The execution strategy: 'limit', 'market', or 'stoplimit'. Inferred from stop_price if omitted.", false, Some("limit")),
                "acknowledged":
                    IndodaxMcp::bool_param("Security confirmation: This must be set to true."),
            }),
            vec!["pair", "amount", "acknowledged"],
        ),
        IndodaxMcp::tool_def(
            "cancel_order",
            "Cancel an existing open order on the Indodax exchange. This tool requires the specific Order ID, the trading pair it was placed on, and the order side (buy or sell). Once an order is successfully cancelled, any remaining locked funds will be returned to your available balance.",
            serde_json::json!({
                "order_id": IndodaxMcp::num_param("The unique numerical identifier for the order you wish to cancel.", true),
                "pair": IndodaxMcp::str_param("The trading pair associated with the order (e.g., 'btc_idr').", true, None),
                "order_type": IndodaxMcp::str_param("The side of the order you are cancelling: 'buy' or 'sell'.", true, None),
                "acknowledged":
                    IndodaxMcp::bool_param("Security confirmation: Must be set to true to authorize the cancellation of an active order."),
            }),
            vec!["order_id", "pair", "order_type", "acknowledged"],
        ),
        IndodaxMcp::tool_def(
            "cancel_all_orders",
            "Bulk cancel all currently open orders for your account. You can optionally restrict this operation to a specific trading pair. This is a high-impact operation used for quickly clearing your order book or resetting your trading positions.",
            serde_json::json!({
                "pair": IndodaxMcp::str_param("Optional: Only cancel orders for this specific pair (e.g., 'btc_idr'). If omitted, ALL open orders across ALL pairs will be cancelled.", false, None),
                "acknowledged":
                    IndodaxMcp::bool_param("Security confirmation: Must be set to true to authorize the bulk cancellation of all open orders."),
            }),
            vec!["acknowledged"],
        ),
        IndodaxMcp::tool_def(
            "get_order_by_client_id",
            "Retrieve detailed information for a specific order using its client-assigned Order ID (client_order_id). This is useful for tracking orders that you have tagged with your own identifiers.",
            serde_json::json!({
                "client_order_id": IndodaxMcp::str_param("The client-assigned unique identifier for the order.", true, None),
            }),
            vec!["client_order_id"],
        ),
        IndodaxMcp::tool_def(
            "buy_preview",
            "[REQUIRES AUTH, READ-ONLY] Preview a buy order without executing it. Returns a detailed summary of the intended trade, including pair, amount, price, and any validation warnings (e.g., tick size). Use this to verify trade parameters with the user before calling buy_order.",
            serde_json::json!({
                "pair": IndodaxMcp::str_param("The trading pair you wish to preview (e.g., 'btc_idr').", true, None),
                "idr": IndodaxMcp::num_param("The total amount of IDR to spend.", true),
                "price": IndodaxMcp::num_param("The limit price per unit.", false),
                "stop_price": IndodaxMcp::num_param("The trigger price for stop-limit orders.", false),
                "client_order_id": IndodaxMcp::str_param("Optional: A custom unique identifier.", false, None),
            }),
            vec!["pair", "idr"],
        ),
        IndodaxMcp::tool_def(
            "sell_preview",
            "[REQUIRES AUTH, READ-ONLY] Preview a sell order without executing it. Returns a detailed summary of the intended trade and validation checks. Recommended for safety before executing a live sell_order.",
            serde_json::json!({
                "pair": IndodaxMcp::str_param("The trading pair you wish to preview (e.g., 'btc_idr').", true, None),
                "amount": IndodaxMcp::num_param("The amount of base asset to sell.", true),
                "price": IndodaxMcp::num_param("The limit price per unit.", false),
                "stop_price": IndodaxMcp::num_param("The trigger price for stop-limit orders.", false),
                "client_order_id": IndodaxMcp::str_param("Optional: A custom unique identifier.", false, None),
            }),
            vec!["pair", "amount"],
        ),
    ]
}

const BALANCE_EPSILON: f64 = 1e-8;

impl IndodaxMcp {
    pub async fn handle_buy_preview(
        &self,
        pair: &str,
        idr: f64,
        price: Option<f64>,
        stop_price: Option<f64>,
        client_order_id: Option<&str>,
    ) -> CallToolResult {
        self.handle_buy_order_internal(pair, idr, price, stop_price, client_order_id, true).await
    }

    pub async fn handle_sell_preview(
        &self,
        pair: &str,
        price: Option<f64>,
        amount: f64,
        stop_price: Option<f64>,
        client_order_id: Option<&str>,
    ) -> CallToolResult {
        self.handle_sell_order_internal(pair, price, amount, "limit", stop_price, client_order_id, true).await
    }

    pub async fn handle_buy_order(
        &self,
        pair: &str,
        idr: f64,
        price: Option<f64>,
        stop_price: Option<f64>,
        client_order_id: Option<&str>,
    ) -> CallToolResult {
        self.handle_buy_order_internal(pair, idr, price, stop_price, client_order_id, false).await
    }

    async fn handle_buy_order_internal(
        &self,
        pair: &str,
        idr: f64,
        price: Option<f64>,
        stop_price: Option<f64>,
        client_order_id: Option<&str>,
        validate: bool,
    ) -> CallToolResult {
        if idr <= 0.0 || !idr.is_finite() {
            return Self::validation_error_result(format!(
                "IDR amount must be positive and finite, got {}",
                idr
            ));
        }

        let info = match self.get_account_info().await {
            Ok(data) => data,
            Err(e) => return Self::error_from_indodax(&e),
        };

        let idr_balance = crate::commands::helpers::parse_balance(&info, "idr");

        if idr_balance + BALANCE_EPSILON < idr {
            return Self::error_result(format!(
                "Insufficient IDR balance. Need {:.2}, have {:.2}",
                idr, idr_balance
            ));
        }

        match crate::commands::trade::place_buy_order(&self.client, pair, idr, price, None, stop_price, client_order_id, validate).await {
            Ok(output) => Self::json_result(output.data),
            Err(e) => Self::error_result(format!("Trade failed: {}", e)),
        }
    }

    pub async fn handle_sell_order(
        &self,
        pair: &str,
        price: Option<f64>,
        amount: f64,
        order_type: &str,
        stop_price: Option<f64>,
        client_order_id: Option<&str>,
    ) -> CallToolResult {
        self.handle_sell_order_internal(pair, price, amount, order_type, stop_price, client_order_id, false).await
    }

    async fn handle_sell_order_internal(
        &self,
        pair: &str,
        price: Option<f64>,
        amount: f64,
        order_type: &str,
        stop_price: Option<f64>,
        client_order_id: Option<&str>,
        validate: bool,
    ) -> CallToolResult {
        if amount <= 0.0 || !amount.is_finite() {
            return Self::validation_error_result(format!(
                "Amount must be positive and finite, got {}",
                amount
            ));
        }

        let base_currency = pair.split('_').next().unwrap_or_default();
        if base_currency.is_empty() {
            return Self::validation_error_result(format!("Invalid pair format: {}", pair));
        }

        let info = match self.get_account_info().await {
            Ok(data) => data,
            Err(e) => return Self::error_from_indodax(&e),
        };

        let base_balance = crate::commands::helpers::parse_balance(&info, base_currency);

        if base_balance + BALANCE_EPSILON < amount {
            return Self::error_result(format!(
                "Insufficient {} balance. Need {:.8}, have {:.8}",
                base_currency.to_uppercase(),
                amount,
                base_balance
            ));
        }

        match crate::commands::trade::place_sell_order(&self.client, pair, price, amount, Some(order_type), stop_price, client_order_id, validate).await {
            Ok(output) => Self::json_result(output.data),
            Err(e) => Self::error_result(format!("Trade failed: {}", e)),
        }
    }

    pub async fn handle_cancel_order(
        &self,
        order_id: f64,
        pair: &str,
        order_type: &str,
    ) -> CallToolResult {
        if order_type != "buy" && order_type != "sell" {
            return Self::validation_error_result(format!(
                "Invalid order_type '{}'. Must be 'buy' or 'sell'.",
                order_type
            ));
        }
        let mut params = HashMap::new();
        params.insert("symbol".to_string(), crate::commands::helpers::normalize_pair_v2(pair));

        let path = format!("/api/v2/order/{}", order_id as u64);
        match self
            .client
            .private_delete_v2::<Value>(&path, &params)
            .await
        {
            Ok(data) => Self::json_result(data),
            Err(e) => Self::error_from_indodax(&e),
        }
    }

    pub async fn handle_cancel_all_orders(&self, pair: Option<&str>) -> CallToolResult {
        let scope_warning = if pair.is_none() {
            Some("[WARN] No pair filter specified — cancelling ALL open orders across all pairs. This is a global operation.")
        } else {
            None
        };
        match crate::commands::helpers::cancel_all_open_orders(&self.client, pair).await {
            Ok((cancelled_ids, failed_ids)) => {
                let mut result = serde_json::json!({
                    "cancelled_count": cancelled_ids.len(),
                    "cancelled_ids": cancelled_ids,
                    "failed_count": failed_ids.len(),
                    "failed_ids": failed_ids,
                });
                if let Some(warning) = scope_warning {
                    result["warning"] = serde_json::Value::String(warning.to_string());
                    eprintln!("[MCP] {}", warning);
                }
                Self::json_result(result)
            }
            Err(e) => Self::error_from_indodax(&e),
        }
    }

    pub async fn handle_get_order_by_client_id(&self, client_order_id: &str) -> CallToolResult {
        let mut params = HashMap::new();
        params.insert("client_order_id".to_string(), client_order_id.to_string());

        match self
            .client
            .private_post_v1::<Value>("getOrderByClientOrderId", &params)
            .await
        {
            Ok(data) => Self::json_result(data),
            Err(e) => Self::error_from_indodax(&e),
        }
    }
}
