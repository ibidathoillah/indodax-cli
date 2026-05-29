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
    ]
}

const BALANCE_EPSILON: f64 = 1e-8;

impl IndodaxMcp {
    pub async fn handle_buy_order(
        &self,
        pair: &str,
        idr: f64,
        price: Option<f64>,
        stop_price: Option<f64>,
        client_order_id: Option<&str>,
    ) -> CallToolResult {
        if idr <= 0.0 || !idr.is_finite() {
            return Self::validation_error_result(format!(
                "IDR amount must be positive and finite, got {}",
                idr
            ));
        }
        if let Some(p) = price {
            if p <= 0.0 || !p.is_finite() {
                return Self::validation_error_result(format!(
                    "Price must be positive and finite, got {}",
                    p
                ));
            }
        }
        if let Some(sp) = stop_price {
            if sp <= 0.0 || !sp.is_finite() {
                return Self::validation_error_result(format!(
                    "Stop price must be positive and finite, got {}",
                    sp
                ));
            }
            if price.is_none() {
                return Self::validation_error_result(
                    "Price (limit price) is required when stop_price is provided.".into(),
                );
            }
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

        let mut params = HashMap::new();
        params.insert("pair".to_string(), pair.to_string());
        params.insert("type".to_string(), "buy".to_string());
        params.insert("idr".to_string(), idr.to_string());
        if let Some(cid) = client_order_id {
            params.insert("client_order_id".to_string(), cid.to_string());
        }

        if let Some(sp) = stop_price {
            params.insert("order_type".to_string(), "stoplimit".to_string());
            params.insert("stop_price".to_string(), sp.to_string());
            if let Some(p) = price {
                params.insert("price".to_string(), p.to_string());
            }
        } else if let Some(p) = price {
            params.insert("price".to_string(), p.to_string());
        } else {
            eprintln!("[MCP] Warning: Market buy order without limit price. Indodax may reject market orders with IDR amount.");
            params.insert("order_type".to_string(), "market".to_string());
        }

        let tick_warning = if let Some(p) = price {
            crate::commands::helpers::validate_tick_size(&self.client, pair, p).await
        } else {
            None
        };

        match self.client.private_post_v1::<Value>("trade", &params).await {
            Ok(data) => Self::json_result_with_warning(data, tick_warning),
            Err(e) => Self::error_from_indodax(&e),
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
        if amount <= 0.0 || !amount.is_finite() {
            return Self::validation_error_result(format!(
                "Amount must be positive and finite, got {}",
                amount
            ));
        }
        if let Some(p) = price {
            if p <= 0.0 || !p.is_finite() {
                return Self::validation_error_result(format!(
                    "Price must be positive and finite, got {}",
                    p
                ));
            }
        }
        if let Some(sp) = stop_price {
            if sp <= 0.0 || !sp.is_finite() {
                return Self::validation_error_result(format!(
                    "Stop price must be positive and finite, got {}",
                    sp
                ));
            }
        }

        let base_currency = pair.split('_').next().unwrap_or_default();
        if base_currency.is_empty() {
            return Self::validation_error_result(format!("Invalid pair format: {}", pair));
        }

        let mut final_order_type = order_type.to_string();
        if stop_price.is_some() {
            final_order_type = "stoplimit".to_string();
        }

        let is_market = match final_order_type.as_str() {
            "market" => {
                if price.is_some() {
                    return Self::validation_error_result(
                        "Cannot specify 'price' for a 'market' sell order. Market orders use the current best available price.".into(),
                    );
                }
                true
            }
            "limit" => {
                if price.is_none() {
                    return Self::validation_error_result(
                        "price is required when order_type is 'limit'".into(),
                    );
                }
                false
            }
            "stoplimit" => {
                if price.is_none() {
                    return Self::validation_error_result(
                        "price (limit price) is required for 'stoplimit' orders".into(),
                    );
                }
                if stop_price.is_none() {
                    return Self::validation_error_result(
                        "stop_price is required for 'stoplimit' orders".into(),
                    );
                }
                false
            }
            _ => {
                return Self::validation_error_result(format!(
                    "Invalid order_type '{}'. Must be 'limit', 'market', or 'stoplimit'.",
                    final_order_type
                ));
            }
        };

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

        let mut params = HashMap::new();
        params.insert("pair".to_string(), pair.to_string());
        params.insert("type".to_string(), "sell".to_string());
        params.insert(base_currency.to_string(), amount.to_string());
        if let Some(cid) = client_order_id {
            params.insert("client_order_id".to_string(), cid.to_string());
        }

        if let Some(sp) = stop_price {
            params.insert("order_type".to_string(), "stoplimit".to_string());
            params.insert("stop_price".to_string(), sp.to_string());
        } else if is_market {
            params.insert("order_type".to_string(), "market".to_string());
        }

        if let Some(p) = price {
            params.insert("price".to_string(), p.to_string());
        }

        let tick_warning = if let Some(p) = price {
            crate::commands::helpers::validate_tick_size(&self.client, pair, p).await
        } else {
            None
        };

        match self.client.private_post_v1::<Value>("trade", &params).await {
            Ok(data) => Self::json_result_with_warning(data, tick_warning),
            Err(e) => Self::error_from_indodax(&e),
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
        params.insert("order_id".to_string(), (order_id as u64).to_string());
        params.insert("pair".to_string(), pair.to_string());
        params.insert("type".to_string(), order_type.to_string());

        match self
            .client
            .private_post_v1::<Value>("cancelOrder", &params)
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
