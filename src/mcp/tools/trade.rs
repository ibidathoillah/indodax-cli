use std::collections::HashMap;

use serde_json::Value;
use rmcp::model::{CallToolResult, Tool};

use super::IndodaxMcp;

pub fn trade_tools() -> Vec<Tool> {
    vec![
        IndodaxMcp::tool_def(
            "buy_order",
            "[DANGEROUS: requires acknowledged=true] Place a buy order on Indodax",
            serde_json::json!({
                "pair": IndodaxMcp::str_param("Trading pair, e.g. btc_idr", true, None),
                "idr": IndodaxMcp::num_param("Total IDR amount to spend", true),
                "price": IndodaxMcp::num_param("Limit price (omit for market order)", false),
                "acknowledged":
                    IndodaxMcp::bool_param("Must be true to confirm this dangerous operation"),
            }),
            vec!["pair", "idr", "acknowledged"],
        ),
        IndodaxMcp::tool_def(
            "sell_order",
            "[DANGEROUS: requires acknowledged=true] Place a sell order on Indodax",
            serde_json::json!({
                "pair": IndodaxMcp::str_param("Trading pair, e.g. btc_idr", true, None),
                "price": IndodaxMcp::num_param("Limit price (omit for market order)", false),
                "amount": IndodaxMcp::num_param("Amount in base currency (e.g. BTC)", true),
                "order_type":
                    IndodaxMcp::str_param("Order type: limit or market", false, Some("limit")),
                "acknowledged":
                    IndodaxMcp::bool_param("Must be true to confirm this dangerous operation"),
            }),
            vec!["pair", "amount", "acknowledged"],
        ),
        IndodaxMcp::tool_def(
            "cancel_order",
            "[DANGEROUS: requires acknowledged=true] Cancel an existing order by ID",
            serde_json::json!({
                "order_id": IndodaxMcp::num_param("Order ID to cancel", true),
                "pair": IndodaxMcp::str_param("Trading pair, e.g. btc_idr", true, None),
                "order_type": IndodaxMcp::str_param("Order type: buy or sell", true, None),
                "acknowledged":
                    IndodaxMcp::bool_param("Must be true to confirm this dangerous operation"),
            }),
            vec!["order_id", "pair", "order_type", "acknowledged"],
        ),
        IndodaxMcp::tool_def(
            "cancel_all_orders",
            "[DANGEROUS: requires acknowledged=true] Cancel all open orders, optionally filtered by pair",
            serde_json::json!({
                "pair": IndodaxMcp::str_param("Only cancel orders for this trading pair (e.g. btc_idr)", false, None),
                "acknowledged":
                    IndodaxMcp::bool_param("Must be true to confirm this dangerous operation"),
            }),
            vec!["acknowledged"],
        ),
    ]
}

const BALANCE_EPSILON: f64 = 1e-8;

impl IndodaxMcp {
    pub async fn handle_buy_order(&self, pair: &str, idr: f64, price: Option<f64>) -> CallToolResult {
        if idr <= 0.0 || !idr.is_finite() {
            return Self::validation_error_result(format!("IDR amount must be positive and finite, got {}", idr));
        }
        if let Some(p) = price {
            if p <= 0.0 || !p.is_finite() {
                return Self::validation_error_result(format!("Price must be positive and finite, got {}", p));
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

        match self
            .client
            .private_post_v1::<Value>("trade", &params)
            .await
        {
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
            return Self::validation_error_result(format!("Amount must be positive and finite, got {}", amount));
        }
        if let Some(p) = price {
            if p <= 0.0 || !p.is_finite() {
                return Self::validation_error_result(format!("Price must be positive and finite, got {}", p));
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

        match self
            .client
            .private_post_v1::<Value>("trade", &params)
            .await
        {
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

    pub async fn handle_cancel_all_orders(
        &self,
        pair: Option<&str>,
    ) -> CallToolResult {
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
