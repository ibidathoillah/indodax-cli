use std::collections::HashMap;

use rmcp::model::{CallToolResult, Tool};
use serde_json::Value;

use super::IndodaxMcp;

pub fn trade_tools() -> Vec<Tool> {
    vec![
        IndodaxMcp::tool_def(
            "buy_order",
            "Place a new buy order on the Indodax exchange. This tool supports both Limit orders (where you specify the maximum price) and Market orders (executed at the best available current price). It is a 'dangerous' operation that requires the 'acknowledged' parameter to be set to true. Always verify your available IDR balance before execution to avoid insufficient funds errors.",
            serde_json::json!({
                "pair": IndodaxMcp::str_param("The trading pair you wish to buy (e.g., 'btc_idr', 'eth_idr'). The format is typically base_quote in lowercase.", true, None),
                "idr": IndodaxMcp::num_param("The total amount of Indonesian Rupiah (IDR) you want to spend on this purchase, including fees.", true),
                "price": IndodaxMcp::num_param("Optional: The maximum price per unit you are willing to pay (Limit Order). If omitted, the exchange will execute a Market Order using the best available price in the order book.", false),
                "acknowledged":
                    IndodaxMcp::bool_param("Security confirmation: This must be explicitly set to true to acknowledge that you are performing a real-money trade operation."),
            }),
            vec!["pair", "idr", "acknowledged"],
        ),
        IndodaxMcp::tool_def(
            "sell_order",
            "Place a new sell order on the Indodax exchange. Supports Limit orders for target prices and Market orders for immediate liquidation. This tool requires the 'acknowledged' parameter for safety. Ensure you have the required quantity of the base asset (e.g., BTC) available in your account before attempting a sell.",
            serde_json::json!({
                "pair": IndodaxMcp::str_param("The trading pair you wish to sell (e.g., 'btc_idr').", true, None),
                "price": IndodaxMcp::num_param("Optional: The minimum price per unit you want to receive (required if order_type is 'limit').", false),
                "amount": IndodaxMcp::num_param("The exact quantity of the base asset (e.g., 0.005 for BTC) you wish to sell.", true),
                "order_type":
                    IndodaxMcp::str_param("The execution strategy: 'limit' (fixed price) or 'market' (execute immediately at current price). Default is 'limit'.", false, Some("limit")),
                "acknowledged":
                    IndodaxMcp::bool_param("Security confirmation: This must be set to true to acknowledge that you are performing a real-money trade operation."),
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
    ]
}

const BALANCE_EPSILON: f64 = 1e-8;

impl IndodaxMcp {
    pub async fn handle_buy_order(
        &self,
        pair: &str,
        idr: f64,
        price: Option<f64>,
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

        if let Some(p) = price {
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

        let base_currency = pair.split('_').next().unwrap_or_default();
        if base_currency.is_empty() {
            return Self::validation_error_result(format!("Invalid pair format: {}", pair));
        }

        let is_market = match order_type {
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
            _ => {
                return Self::validation_error_result(format!(
                    "Invalid order_type '{}'. Must be 'limit' or 'market'.",
                    order_type
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

        if let Some(p) = price {
            if !is_market {
                params.insert("price".to_string(), p.to_string());
            }
        }
        if is_market {
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
}
