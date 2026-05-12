use rmcp::model::{CallToolResult, Tool};

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
                "amount": IndodaxMcp::num_param("Amount in base currency", true),
            }),
            vec!["amount"],
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
    ]
}

impl IndodaxMcp {
    pub async fn handle_paper_init(&self, idr: Option<f64>, btc: Option<f64>) -> CallToolResult {
        let mut config = self.config.lock().await;
        let state = crate::commands::paper::init_paper_state(idr, btc);
        let msg = format!(
            "[PAPER] Paper trading initialized with {:.0} IDR and {:.8} BTC",
            state.balances.get("idr").copied().unwrap_or(100_000_000.0),
            state.balances.get("btc").copied().unwrap_or(1.0),
        );
        match state.save(&mut config) {
            Ok(()) => Self::ok_result(msg),
            Err(e) => Self::error_from_indodax(&e),
        }
    }

    pub async fn handle_paper_reset(&self) -> CallToolResult {
        let mut config = self.config.lock().await;
        let state = crate::commands::paper::PaperState::default();
        match state.save(&mut config) {
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
        amount: f64,
    ) -> CallToolResult {
        let mut config = self.config.lock().await;
        let mut state = crate::commands::paper::PaperState::load(&config);
        match crate::commands::paper::place_paper_order(&mut state, pair, side, price, amount) {
            Ok(_output) => {
                if let Err(e) = state.save(&mut config) {
                    return Self::error_from_indodax(&e);
                }
                Self::json_result(serde_json::json!({
                    "mode": "paper",
                    "side": side,
                    "pair": pair,
                    "price": price,
                    "amount": amount,
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
        let mut config = self.config.lock().await;
        let mut state = crate::commands::paper::PaperState::load(&config);
        match crate::commands::paper::cancel_paper_order(&mut state, order_id) {
            Ok(()) => {
                if let Err(e) = state.save(&mut config) {
                    return Self::error_from_indodax(&e);
                }
                Self::ok_result(format!("[PAPER] Order {} cancelled", order_id))
            }
            Err(e) => Self::error_from_indodax(&e),
        }
    }

    pub async fn handle_paper_cancel_all(&self) -> CallToolResult {
        let mut config = self.config.lock().await;
        let mut state = crate::commands::paper::PaperState::load(&config);
        let count = crate::commands::paper::cancel_all_paper_orders(&mut state);
        if let Err(e) = state.save(&mut config) {
            return Self::error_from_indodax(&e);
        }
        Self::ok_result(format!("[PAPER] Cancelled {} orders", count))
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
}
