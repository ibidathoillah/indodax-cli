use rmcp::model::{CallToolResult, Tool};

use super::IndodaxMcp;

pub fn paper_tools() -> Vec<Tool> {
    vec![
        IndodaxMcp::tool_def(
            "paper_init",
            "[LOCAL WRITE, DESTRUCTIVE TO PAPER STATE ONLY] Initialize or replace the paper trading simulation with virtual IDR and BTC balances. This never touches real Indodax funds, but it overwrites any existing simulated balances, open paper orders, and paper trade history. Call this before paper_buy or paper_sell when starting a fresh practice session; the response confirms the virtual starting balances.",
            serde_json::json!({
                "idr": IndodaxMcp::num_param("The initial virtual IDR balance for the simulation. Defaults to 100,000,000 IDR if not specified.", false),
                "btc": IndodaxMcp::num_param("The initial virtual BTC balance for the simulation. Defaults to 1.0 BTC if not specified.", false),
            }),
            vec![],
        ),
        IndodaxMcp::tool_def(
            "paper_reset",
            "[LOCAL WRITE, DESTRUCTIVE TO PAPER STATE ONLY] Reset the paper trading simulation to the default virtual balances. This clears simulated open orders and simulated trade history, but never places or cancels real Indodax orders. Use paper_init instead when you need custom starting balances.",
            serde_json::json!({}),
            vec![],
        ),
        IndodaxMcp::tool_def(
            "paper_balance",
            "View your current virtual balances in the paper trading simulation. This tool shows you how much simulated IDR and cryptocurrency you have available for trading, as well as any funds currently locked in simulated open orders.",
            serde_json::json!({}),
            vec![],
        ),
        IndodaxMcp::tool_def(
            "paper_buy",
            "Execute a simulated buy order in the paper trading environment. You can specify a target price for a Limit order or omit it for a Market order. This tool allows you to test your market entry timing and price sensitivity without using real funds.",
            serde_json::json!({
                "pair":
                    IndodaxMcp::str_param("The trading pair to simulate buying (e.g., 'btc_idr').", false, Some("btc_idr")),
                "price": IndodaxMcp::num_param("Optional: The simulated limit price per unit. If omitted, the tool simulates a Market Order at the current market price.", false),
                "amount": IndodaxMcp::num_param("The amount of base currency (e.g., BTC) to buy. Use this OR 'idr' to specify the trade size.", false),
                "idr": IndodaxMcp::num_param("The total amount of virtual IDR to spend. Use this OR 'amount' to specify the trade size.", false),
            }),
            vec![],
        ),
        IndodaxMcp::tool_def(
            "paper_sell",
            "Execute a simulated sell order in the paper trading environment. Test your profit-taking targets and exit strategies. Like 'paper_buy', this supports both fixed limit prices and immediate market-price simulation.",
            serde_json::json!({
                "pair":
                    IndodaxMcp::str_param("The trading pair to simulate selling (e.g., 'btc_idr').", false, Some("btc_idr")),
                "price": IndodaxMcp::num_param("Optional: The simulated limit price per unit. Omit for a simulated Market Order.", false),
                "amount": IndodaxMcp::num_param("The exact quantity of the simulated asset you wish to sell.", true),
            }),
            vec!["amount"],
        ),
        IndodaxMcp::tool_def(
            "paper_orders",
            "[LOCAL READ-ONLY] List active, unfilled simulated orders in the current paper trading session as JSON text. This does not contact Indodax and does not change virtual balances. Use paper_history for filled or cancelled simulated orders.",
            serde_json::json!({}),
            vec![],
        ),
        IndodaxMcp::tool_def(
            "paper_cancel",
            "Cancel a specific pending order in the paper trading simulation using its unique Order ID. Successfully cancelled orders will return their locked virtual funds to your available simulation balance.",
            serde_json::json!({
                "order_id": IndodaxMcp::num_param("The unique numerical ID of the simulated order you want to cancel.", true),
            }),
            vec!["order_id"],
        ),
        IndodaxMcp::tool_def(
            "paper_cancel_all",
            "Bulk cancel every active simulated order currently pending in the paper trading environment. This tool is useful for quickly resetting your virtual positions or clearing a cluttered order book.",
            serde_json::json!({}),
            vec![],
        ),
        IndodaxMcp::tool_def(
            "paper_history",
            "Retrieve a complete history of all filled and cancelled orders within the current paper trading session. This allows you to review your simulated trade performance and refine your strategies based on past results.",
            serde_json::json!({}),
            vec![],
        ),
        IndodaxMcp::tool_def(
            "paper_status",
            "Get a comprehensive performance summary of your paper trading session. This tool calculates total trades executed, current virtual portfolio value, and your overall unrealized profit and loss (P&L) based on current market prices.",
            serde_json::json!({}),
            vec![],
        ),
        IndodaxMcp::tool_def(
            "paper_fill",
            "[LOCAL WRITE TO PAPER STATE] Manually mark one or all open paper orders as filled, updating only simulated balances and simulated history. It never sends real exchange orders. Provide price for a manual fill, omit price to use the order price, or set fetch=true to read current Indodax market prices before deciding fills.",
            serde_json::json!({
                "order_id":
                    IndodaxMcp::num_param("The ID of the specific simulated order to fill. Optional if 'all' is true.", false),
                "price": IndodaxMcp::num_param("Optional: The specific price at which the order should be filled. If omitted, the tool uses the original order's price.", false),
                "all": IndodaxMcp::bool_param("Set to true to attempt to fill ALL currently open simulated orders at once."),
                "fetch": IndodaxMcp::bool_param("Set to true to automatically fetch current real-world market prices and use them as the fill price for the simulation."),
            }),
            vec![],
        ),
        IndodaxMcp::tool_def(
            "paper_check_fills",
            "[LOCAL WRITE TO PAPER STATE, OPTIONAL PUBLIC MARKET READ] Evaluate open paper orders against supplied prices or fetched Indodax market prices, then fill only simulated orders whose conditions match. This is the paper matching-engine step after paper_buy or paper_sell; it never places real orders. Returns JSON text summarizing filled and remaining simulated orders.",
            serde_json::json!({
                "prices":
                    IndodaxMcp::str_param("Optional: A JSON object of current prices to check against (e.g., '{\"btc_idr\": 100000000}'). If omitted, use 'fetch: true' to get real-time data.", false, None),
                "fetch": IndodaxMcp::bool_param("Set to true to fetch the latest real-time market prices from the Indodax API to accurately simulate order fills."),
            }),
            vec![],
        ),
    ]
}

impl IndodaxMcp {
    pub async fn handle_paper_init(&self, idr: Option<f64>, btc: Option<f64>) -> CallToolResult {
        let state = crate::commands::paper::init_paper_state(idr, btc);
        use crate::commands::paper::{DEFAULT_BALANCE_BTC, DEFAULT_BALANCE_IDR};
        let idr_str = crate::commands::helpers::format_balance(
            "idr",
            state
                .balances
                .get("idr")
                .copied()
                .unwrap_or(DEFAULT_BALANCE_IDR),
        );
        let btc_str = crate::commands::helpers::format_balance(
            "btc",
            state
                .balances
                .get("btc")
                .copied()
                .unwrap_or(DEFAULT_BALANCE_BTC),
        );
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
        let state = self.load_paper_state().await;
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
        let mut state = self.load_paper_state().await;
        let result = if side == "buy" {
            if let Some(idr_val) = idr {
                crate::commands::paper::place_paper_order_idr(
                    &mut state, pair, side, idr_val, price,
                )
            } else if let Some(amt) = amount {
                crate::commands::paper::place_paper_order(&mut state, pair, side, price, amt)
            } else {
                return Self::validation_error_result(
                    "Either amount or idr must be specified for buy".into(),
                );
            }
        } else {
            let amt = match amount {
                Some(v) if v > 0.0 => v,
                Some(v) => {
                    return Self::validation_error_result(format!(
                        "Amount must be positive, got {}",
                        v
                    ))
                }
                None => {
                    return Self::validation_error_result(
                        "Missing required parameter: amount".into(),
                    )
                }
            };
            crate::commands::paper::place_paper_order(&mut state, pair, side, price, amt)
        };
        match result {
            Ok(_output) => {
                if let Err(e) = self.save_paper_state(&state).await {
                    return Self::error_from_indodax(&e);
                }
                let response_amount = state
                    .orders
                    .last()
                    .map(|o| o.amount)
                    .or(amount)
                    .unwrap_or(0.0);
                let mut response = serde_json::json!({
                    "mode": "paper",
                    "side": side,
                    "pair": pair,
                    "price": price,
                    "amount": response_amount,
                    "status": "open",
                });
                if idr.is_some() {
                    response["idr_spent"] = serde_json::json!(idr);
                }
                Self::json_result(response)
            }
            Err(e) => Self::error_from_indodax(&e),
        }
    }

    pub async fn handle_paper_orders(&self) -> CallToolResult {
        let state = self.load_paper_state().await;
        Self::json_result(crate::commands::paper::paper_orders_value(&state))
    }

    pub async fn handle_paper_cancel(&self, order_id: u64) -> CallToolResult {
        let mut state = self.load_paper_state().await;
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
        let mut state = self.load_paper_state().await;
        let (count, failures) = crate::commands::paper::cancel_all_paper_orders(&mut state);
        if let Err(e) = self.save_paper_state(&state).await {
            return Self::error_from_indodax(&e);
        }
        let msg = if failures.is_empty() {
            format!("[PAPER] Cancelled {} orders", count)
        } else {
            let reasons: Vec<String> = failures
                .iter()
                .map(|(id, e)| format!("{}: {}", id, e))
                .collect();
            format!(
                "[PAPER] Cancelled {} orders, {} failed: {}",
                count,
                failures.len(),
                reasons.join("; ")
            )
        };
        Self::ok_result(msg)
    }

    pub async fn handle_paper_history(&self) -> CallToolResult {
        let state = self.load_paper_state().await;
        Self::json_result(crate::commands::paper::paper_history_value(&state))
    }

    pub async fn handle_paper_status(&self) -> CallToolResult {
        let state = self.load_paper_state().await;
        Self::json_result(crate::commands::paper::paper_status_value(&state))
    }

    pub async fn handle_paper_fill(
        &self,
        order_id: Option<f64>,
        fill_price: Option<f64>,
        fill_all: bool,
        fetch: bool,
    ) -> CallToolResult {
        let order_id = match order_id {
            Some(v) if v.fract() != 0.0 || v <= 0.0 => {
                return Self::validation_error_result(format!(
                    "order_id must be a positive whole number, got {}",
                    v
                ));
            }
            Some(v) => Some(v as u64),
            None => None,
        };
        let mut state = self.load_paper_state().await;
        match crate::commands::paper::paper_fill(
            &mut state,
            order_id,
            fill_price,
            fill_all,
            Some(&self.client),
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

    pub async fn handle_paper_check_fills(
        &self,
        prices: Option<&str>,
        fetch: bool,
    ) -> CallToolResult {
        let mut state = self.load_paper_state().await;
        match crate::commands::paper::paper_check_fills(&self.client, &mut state, prices, fetch)
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
