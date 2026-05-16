use serde_json::Value;
use rmcp::model::{CallToolResult, Tool};
use std::collections::HashMap;

use super::IndodaxMcp;

pub fn funding_tools() -> Vec<Tool> {
    vec![
        IndodaxMcp::tool_def(
            "withdraw_fee",
            "[REQUIRES AUTH] Check withdrawal fee for a currency",
            serde_json::json!({
                "currency": IndodaxMcp::str_param("Currency to check, e.g. btc", true, None),
                "network": IndodaxMcp::str_param("Blockchain network (optional)", false, None),
            }),
            vec!["currency"],
        ),
        IndodaxMcp::tool_def(
            "withdraw",
            "[DANGEROUS: requires acknowledged=true] Withdraw cryptocurrency from Indodax",
            serde_json::json!({
                "currency": IndodaxMcp::str_param("Currency to withdraw, e.g. btc", true, None),
                "amount": IndodaxMcp::num_param("Amount to withdraw", true),
                "address":
                    IndodaxMcp::str_param("Destination address or Indodax username", true, None),
                "to_username":
                    IndodaxMcp::bool_param("Withdraw to Indodax username instead of blockchain address"),
                "memo": IndodaxMcp::str_param("Memo/tag for currencies that require it", false, None),
                "network": IndodaxMcp::str_param("Blockchain network", false, None),
                "callback_url": IndodaxMcp::str_param("Callback URL for withdrawal notification", false, None),
                "acknowledged":
                    IndodaxMcp::bool_param("Must be true to confirm this dangerous operation"),
            }),
            vec!["currency", "amount", "address", "acknowledged"],
        ),
    ]
}

impl IndodaxMcp {
    pub async fn handle_withdraw_fee(
        &self,
        currency: &str,
        network: Option<&str>,
    ) -> CallToolResult {
        let mut params = HashMap::new();
        params.insert("currency".to_string(), currency.to_string());
        if let Some(n) = network {
            params.insert("network".to_string(), n.to_string());
        }

        match self
            .client
            .private_post_v1::<Value>("withdrawFee", &params)
            .await
        {
            Ok(data) => Self::json_result(data),
            Err(e) => Self::error_from_indodax(&e),
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn handle_withdraw(
        &self,
        currency: &str,
        amount: f64,
        address: &str,
        to_username: bool,
        memo: Option<&str>,
        network: Option<&str>,
        callback_url: Option<&str>,
    ) -> CallToolResult {
        if currency.is_empty() {
            return Self::validation_error_result("Missing required parameter: currency".into());
        }
        if amount <= 0.0 || !amount.is_finite() {
            return Self::validation_error_result(format!("Amount must be positive and finite, got {}", amount));
        }
        if address.is_empty() {
            return Self::validation_error_result("Missing required parameter: address".into());
        }
        let params = crate::commands::helpers::build_withdraw_params(currency, amount, address, to_username, memo, network, callback_url);

        match self
            .client
            .private_post_v1::<Value>("withdrawCoin", &params)
            .await
        {
            Ok(data) => Self::json_result(data),
            Err(e) => Self::error_from_indodax(&e),
        }
    }
}
