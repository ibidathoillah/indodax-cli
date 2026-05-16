use rmcp::model::{CallToolResult, Tool};
use crate::errors::IndodaxError;
use crate::auth::Signer;
use serde_json::Value;

use super::IndodaxMcp;

pub fn paper_tools() -> Vec<Tool> {
    vec![
        IndodaxMcp::tool_def(
            "paper_init",
            "Initialize paper trading with default or custom virtual balances",
            serde_json::json!({
                "idr": IndodaxMcp::num_param("Initial IDR balance (default: 100000000)", false),
                "btc": IndodaxMcp::num_param("Initial BTC balance (default: 1.0)", false),
            }),
            vec![],
        ),
        IndodaxMcp::tool_def(
            "paper_reset",
            "Reset paper trading state to defaults",
            serde_json::json!({}),
            vec![],
        ),
        IndodaxMcp::tool_def(
            "paper_balance",
            "Show current paper trading virtual balances",
            serde_json::json!({}),
            vec![],
        ),
        IndodaxMcp::tool_def(
            "paper_buy",
            "Place a simulated paper buy order (omit price for market order)",
            serde_json::json!({
                "pair":
                    IndodaxMcp::str_param("Trading pair, e.g. btc_idr", false, Some("btc_idr")),
                "price": IndodaxMcp::num_param("Price for the order (omit for market order)", false),
                "amount": IndodaxMcp::num_param("Amount in base currency (alternative to idr)", false),
                "idr": IndodaxMcp::num_param("IDR amount to spend (alternative to amount)", false),
            }),
            vec![],
        ),
        IndodaxMcp::tool_def(
            "paper_sell",
            "Place a simulated paper sell order (omit price for market order)",
            serde_json::json!({
                "pair":
                    IndodaxMcp::str_param("Trading pair, e.g. btc_idr", false, Some("btc_idr")),
                "price": IndodaxMcp::num_param("Price for the order (omit for market order)", false),
                "amount": IndodaxMcp::num_param("Amount in base currency", true),
            }),
            vec!["amount"],
        ),
        IndodaxMcp::tool_def(
            "paper_orders",
            "List paper trading orders",
            serde_json::json!({}),
            vec![],
        ),
        IndodaxMcp::tool_def(
            "paper_cancel",
            "Cancel a paper trading order",
            serde_json::json!({
                "order_id": IndodaxMcp::num_param("Order ID to cancel", true),
            }),
            vec!["order_id"],
        ),
        IndodaxMcp::tool_def(
            "paper_cancel_all",
            "Cancel all paper trading orders",
            serde_json::json!({}),
            vec![],
        ),
        IndodaxMcp::tool_def(
            "paper_history",
            "Show paper trading order history",
            serde_json::json!({}),
            vec![],
        ),
        IndodaxMcp::tool_def(
            "paper_status",
            "Show paper trading status summary (trades, balances, P&L)",
            serde_json::json!({}),
            vec![],
        ),
        IndodaxMcp::tool_def(
            "paper_fill",
            "Fill an open paper order (provide order_id or set all=true)",
            serde_json::json!({
                "order_id":
                    IndodaxMcp::num_param("Order ID to fill (optional if all=true)", false),
                "price": IndodaxMcp::num_param("Fill price (defaults to order price)", false),
                "all": IndodaxMcp::bool_param("Fill all open orders"),
            }),
            vec![],
        ),
        IndodaxMcp::tool_def(
            "paper_check_fills",
            "Auto-fill open paper orders based on current market prices",
            serde_json::json!({
                "prices":
                    IndodaxMcp::str_param("JSON object of market prices, e.g. {\"btc_idr\": 100000000}", false, None),
                "fetch": IndodaxMcp::bool_param("Auto-fetch current market prices from Indodax API"),
            }),
            vec![],
        ),
    ]
}

impl IndodaxMcp {
    async fn save_paper_state(&self, state: &crate::commands::paper::PaperState) -> Result<(), IndodaxError> {
        let json = serde_json::to_value(state).map_err(|e| IndodaxError::Other(e.to_string()))?;
        let config_clone = {
            let mut config = self.config.lock().await;
            config.paper_balances = Some(json);
            config.clone()
        };
        config_clone.save().map_err(|e| IndodaxError::Other(e.to_string()))
    }

    pub async fn handle_paper_init(&self, idr: Option<f64>, btc: Option<f64>) -> CallToolResult {
        let state = crate::commands::paper::init_paper_state(idr, btc);
        use crate::commands::paper::{DEFAULT_BALANCE_BTC, DEFAULT_BALANCE_IDR};
        let idr_str = crate::commands::paper::format_balance("idr", state.balances.get("idr").copied().unwrap_or(DEFAULT_BALANCE_IDR));
        let btc_str = crate::commands::paper::format_balance("btc", state.balances.get("btc").copied().unwrap_or(DEFAULT_BALANCE_BTC));
        let msg = format!(
            "[PAPER] Paper trading initialized with {} IDR and {} BTC",
            idr_str, btc_str,
        );
        match self.save_paper_state(&state).await {
            Ok(()) => Self::ok_result(msg),
            Err(e) => Self::error_from_indodax(&e),
        }
    }

    pub async fn handle_paper_reset(&self) -> CallToolResult {
        let state = crate::commands::paper::PaperState::default();
        match self.save_paper_state(&state).await {
            Ok(()) => Self::ok_result("[PAPER] Paper trading state reset".to_string()),
            Err(e) => Self::error_from_indodax(&e),
        }
    }

    pub async fn handle_paper_balance(&self) -> CallToolResult {
        let config = self.config.lock().await;
        let state = crate::commands::paper::PaperState::load(&config);
        Self::json_result(crate::commands::paper::paper_balance_value(&state))
    }

    pub async fn handle_paper_trade(
        &self,
        side: &str,
        pair: &str,
        price: Option<f64>,
        amount: Option<f64>,
        idr: Option<f64>,
    ) -> CallToolResult {
        let mut state = {
            let config = self.config.lock().await;
            crate::commands::paper::PaperState::load(&config)
        };
        let result = if side == "buy" {
            if let Some(idr_val) = idr {
                crate::commands::paper::place_paper_order_idr(&mut state, pair, side, idr_val, price)
            } else if let Some(amt) = amount {
                crate::commands::paper::place_paper_order(&mut state, pair, side, price, amt)
            } else {
                return Self::validation_error_result("Either amount or idr must be specified for buy".into());
            }
        } else {
            let amt = match amount {
                Some(v) if v > 0.0 => v,
                Some(v) => return Self::validation_error_result(format!("Amount must be positive, got {}", v)),
                None => return Self::validation_error_result("Missing required parameter: amount".into()),
            };
            crate::commands::paper::place_paper_order(&mut state, pair, side, price, amt)
        };
        match result {
            Ok(_output) => {
                if let Err(e) = self.save_paper_state(&state).await {
                    return Self::error_from_indodax(&e);
                }
                let effective_amount = amount.unwrap_or(0.0);
                Self::json_result(serde_json::json!({
                    "mode": "paper",
                    "side": side,
                    "pair": pair,
                    "price": price,
                    "amount": effective_amount,
                    "status": "open",
                }))
            }
            Err(e) => Self::error_from_indodax(&e),
        }
    }

    pub async fn handle_paper_orders(&self) -> CallToolResult {
        let config = self.config.lock().await;
        let state = crate::commands::paper::PaperState::load(&config);
        Self::json_result(crate::commands::paper::paper_orders_value(&state))
    }

    pub async fn handle_paper_cancel(&self, order_id: u64) -> CallToolResult {
        let mut state = {
            let config = self.config.lock().await;
            crate::commands::paper::PaperState::load(&config)
        };
        match crate::commands::paper::cancel_paper_order(&mut state, order_id) {
            Ok(()) => {
                if let Err(e) = self.save_paper_state(&state).await {
                    return Self::error_from_indodax(&e);
                }
                Self::ok_result(format!("[PAPER] Order {} cancelled", order_id))
            }
            Err(e) => Self::error_from_indodax(&e),
        }
    }

    pub async fn handle_paper_cancel_all(&self) -> CallToolResult {
        let mut state = {
            let config = self.config.lock().await;
            crate::commands::paper::PaperState::load(&config)
        };
        let (count, failures) = crate::commands::paper::cancel_all_paper_orders(&mut state);
        if let Err(e) = self.save_paper_state(&state).await {
            return Self::error_from_indodax(&e);
        }
        let msg = if failures.is_empty() {
            format!("[PAPER] Cancelled {} orders", count)
        } else {
            let reasons: Vec<String> = failures.iter().map(|(id, e)| format!("{}: {}", id, e)).collect();
            format!("[PAPER] Cancelled {} orders, {} failed: {}", count, failures.len(), reasons.join("; "))
        };
        Self::ok_result(msg)
    }

    pub async fn handle_paper_history(&self) -> CallToolResult {
        let config = self.config.lock().await;
        let state = crate::commands::paper::PaperState::load(&config);
        Self::json_result(crate::commands::paper::paper_history_value(&state))
    }

    pub async fn handle_paper_status(&self) -> CallToolResult {
        let config = self.config.lock().await;
        let state = crate::commands::paper::PaperState::load(&config);
        Self::json_result(crate::commands::paper::paper_status_value(&state))
    }

    pub async fn handle_paper_fill(
        &self,
        order_id: Option<f64>,
        fill_price: Option<f64>,
        fill_all: bool,
    ) -> CallToolResult {
        let order_id = match order_id {
            Some(v) if v.fract() != 0.0 || v <= 0.0 => {
                return Self::validation_error_result(format!("order_id must be a positive whole number, got {}", v));
            }
            Some(v) => Some(v as u64),
            None => None,
        };
        let mut state = {
            let config = self.config.lock().await;
            crate::commands::paper::PaperState::load(&config)
        };
        match crate::commands::paper::paper_fill(&mut state, order_id, fill_price, fill_all) {
            Ok(output) => {
                if let Err(e) = self.save_paper_state(&state).await {
                    return Self::error_from_indodax(&e);
                }
                Self::json_result(output.data)
            }
            Err(e) => Self::error_from_indodax(&e),
        }
    }

    pub async fn handle_paper_check_fills(
        &self,
        prices: Option<&str>,
        fetch: bool,
    ) -> CallToolResult {
        let mut state = {
            let config = self.config.lock().await;
            crate::commands::paper::PaperState::load(&config)
        };
        match crate::commands::paper::paper_check_fills(
            &self.client,
            &mut state,
            prices,
            fetch,
        )
        .await
        {
            Ok(output) => {
                if let Err(e) = self.save_paper_state(&state).await {
                    return Self::error_from_indodax(&e);
                }
                Self::json_result(output.data)
            }
            Err(e) => Self::error_from_indodax(&e),
        }
    }
}
