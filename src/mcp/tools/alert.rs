use rmcp::model::{CallToolResult, Tool};

use super::IndodaxMcp;
use crate::commands::alert;

pub fn alert_tools() -> Vec<Tool> {
    vec![
        IndodaxMcp::tool_def(
            "alert_add",
            "Create a new price alert for a specific trading pair. Alerts can be triggered by absolute price thresholds or percentage changes.",
            serde_json::json!({
                "pair": IndodaxMcp::str_param("The trading pair to monitor (e.g., 'btc_idr').", true, None),
                "above": IndodaxMcp::num_param("Trigger the alert when the price rises ABOVE this absolute value.", false),
                "below": IndodaxMcp::num_param("Trigger the alert when the price falls BELOW this absolute value.", false),
                "percent_up": IndodaxMcp::num_param("Trigger the alert when the price increases by this percentage from the current price.", false),
                "percent_down": IndodaxMcp::num_param("Trigger the alert when the price decreases by this percentage from the current price.", false),
                "note": IndodaxMcp::str_param("Optional: A custom note or label to include with the alert notification.", false, None),
            }),
            vec!["pair"],
        ),
        IndodaxMcp::tool_def(
            "alert_list",
            "Retrieve a list of all configured price alerts. By default, only active alerts are shown.",
            serde_json::json!({
                "history": IndodaxMcp::bool_param("Set to true to also include triggered and cancelled alerts in the output."),
            }),
            vec![],
        ),
        IndodaxMcp::tool_def(
            "alert_cancel",
            "Cancel one or more existing price alerts. Use this to stop monitoring specific price conditions.",
            serde_json::json!({
                "id": IndodaxMcp::num_param("The unique ID of the specific alert to cancel.", false),
                "all": IndodaxMcp::bool_param("Set to true to cancel ALL currently active alerts."),
            }),
            vec![],
        ),
        IndodaxMcp::tool_def(
            "alert_check",
            "Manually trigger a check of active alerts against current market prices. Any alerts whose conditions are met will be marked as triggered.",
            serde_json::json!({
                "id": IndodaxMcp::num_param("Optional: Only check a specific alert by its ID.", false),
                "pair": IndodaxMcp::str_param("Optional: Only check alerts for a specific trading pair.", false, None),
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
                "Must provide at least one trigger: above, below, percent_up, or percent_down"
                    .into(),
            );
        }
        let pair = crate::commands::helpers::normalize_pair(pair);
        match alert::alert_add(
            &pair,
            above,
            below,
            percent_up,
            percent_down,
            note,
            &self.client,
        )
        .await
        {
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
                return Self::validation_error_result(format!(
                    "alert ID must be a whole number, got {}",
                    v
                ));
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
                return Self::validation_error_result(format!(
                    "alert ID must be a whole number, got {}",
                    v
                ));
            }
        }
        let id = id.map(|v| v as u64);
        let pair = pair
            .as_deref()
            .map(|p| crate::commands::helpers::normalize_pair(p));
        match alert::alert_check(&self.client, id, pair.as_deref()).await {
            Ok(output) => Self::json_result(output.data),
            Err(e) => Self::error_result(e.to_string()),
        }
    }
}
