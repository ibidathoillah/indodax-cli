use rmcp::model::{CallToolResult, Tool};

use super::IndodaxMcp;
use crate::commands::alert;

pub fn alert_tools() -> Vec<Tool> {
    vec![
        IndodaxMcp::tool_def(
            "alert_add",
            "Create a price alert for a trading pair",
            serde_json::json!({
                "pair": IndodaxMcp::str_param("Trading pair, e.g. btc_idr", true, None),
                "above": IndodaxMcp::num_param("Alert when price goes above this value", false),
                "below": IndodaxMcp::num_param("Alert when price goes below this value", false),
                "percent_up": IndodaxMcp::num_param("Alert when price increases by this percent", false),
                "percent_down": IndodaxMcp::num_param("Alert when price decreases by this percent", false),
                "note": IndodaxMcp::str_param("Optional note for this alert", false, None),
            }),
            vec!["pair"],
        ),
        IndodaxMcp::tool_def(
            "alert_list",
            "List all price alerts",
            serde_json::json!({
                "history": IndodaxMcp::bool_param("Include triggered and cancelled alerts"),
            }),
            vec![],
        ),
        IndodaxMcp::tool_def(
            "alert_cancel",
            "Cancel a price alert by ID or cancel all",
            serde_json::json!({
                "id": IndodaxMcp::num_param("Alert ID to cancel", false),
                "all": IndodaxMcp::bool_param("Cancel all alerts"),
            }),
            vec![],
        ),
        IndodaxMcp::tool_def(
            "alert_check",
            "Check active alerts against current prices and trigger any that match",
            serde_json::json!({
                "id": IndodaxMcp::num_param("Check specific alert by ID", false),
                "pair": IndodaxMcp::str_param("Filter by pair, e.g. btc_idr", false, None),
            }),
            vec![],
        ),
    ]
}

impl IndodaxMcp {
    pub async fn handle_alert_add(
        &self,
        pair: &str,
        above: Option<f64>,
        below: Option<f64>,
        percent_up: Option<f64>,
        percent_down: Option<f64>,
        note: Option<String>,
    ) -> CallToolResult {
        if pair.is_empty() {
            return Self::validation_error_result("Missing required parameter: pair".into());
        }
        if above.is_none() && below.is_none() && percent_up.is_none() && percent_down.is_none() {
            return Self::validation_error_result(
                "Must provide at least one trigger: above, below, percent_up, or percent_down".into()
            );
        }
        let pair = crate::commands::helpers::normalize_pair(pair);
        match alert::alert_add(&pair, above, below, percent_up, percent_down, note, &self.client).await {
            Ok(output) => Self::json_result(output.data),
            Err(e) => Self::error_result(e.to_string()),
        }
    }

    pub async fn handle_alert_list(&self, history: bool) -> CallToolResult {
        match alert::alert_list(history) {
            Ok(output) => Self::json_result(output.data),
            Err(e) => Self::error_result(e.to_string()),
        }
    }

    pub async fn handle_alert_cancel(&self, id: Option<f64>, cancel_all: bool) -> CallToolResult {
        if id.is_none() && !cancel_all {
            return Self::validation_error_result("Must provide either `id` or `all`".into());
        }
        if let Some(v) = id {
            if v.fract() != 0.0 {
                return Self::validation_error_result(format!("alert ID must be a whole number, got {}", v));
            }
        }
        let id = id.map(|v| v as u64);
        match alert::alert_cancel(id, cancel_all) {
            Ok(output) => Self::json_result(output.data),
            Err(e) => Self::error_result(e.to_string()),
        }
    }

    pub async fn handle_alert_check(
        &self,
        id: Option<f64>,
        pair: Option<String>,
    ) -> CallToolResult {
        if let Some(v) = id {
            if v.fract() != 0.0 {
                return Self::validation_error_result(format!("alert ID must be a whole number, got {}", v));
            }
        }
        let id = id.map(|v| v as u64);
        let pair = pair.as_deref().map(|p| crate::commands::helpers::normalize_pair(p));
        match alert::alert_check(&self.client, id, pair.as_deref()).await {
            Ok(output) => Self::json_result(output.data),
            Err(e) => Self::error_result(e.to_string()),
        }
    }
}
