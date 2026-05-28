use rmcp::model::{CallToolResult, Tool};
use serde_json::Value;
use std::collections::HashMap;

use super::IndodaxMcp;

pub fn funding_tools() -> Vec<Tool> {
    vec![
        IndodaxMcp::tool_def(
            "withdraw_fee",
            "Calculate the precise withdrawal fee for a specific cryptocurrency and blockchain network. Fees can vary significantly between networks (e.g., ERC20 vs. BEP20). It is highly recommended to call this tool before initiating a withdrawal to ensure you understand the final net amount that will be received at the destination.",
            serde_json::json!({
                "currency": IndodaxMcp::str_param("The ticker symbol of the cryptocurrency to check (e.g., 'btc', 'eth', 'usdt').", true, None),
                "network": IndodaxMcp::str_param("Optional: The specific blockchain network for the withdrawal (e.g., 'TRC20', 'ERC20'). If omitted, the tool uses the default network for that asset.", false, None),
            }),
            vec!["currency"],
        ),
        IndodaxMcp::tool_def(
            "withdraw",
            "Initiate a cryptocurrency withdrawal from your Indodax account. This tool supports transfers to external blockchain addresses as well as internal transfers to other Indodax usernames. This is a high-security operation that requires the 'acknowledged' parameter. Note that the exchange fee returned by 'withdraw_fee' will be deducted from the 'amount' you specify.",
            serde_json::json!({
                "currency": IndodaxMcp::str_param("The cryptocurrency asset to withdraw (e.g., 'btc').", true, None),
                "amount": IndodaxMcp::num_param("The total gross amount of the asset you wish to withdraw.", true),
                "address":
                    IndodaxMcp::str_param("The target destination: either a valid blockchain address OR an Indodax username if 'to_username' is true.", true, None),
                "to_username":
                    IndodaxMcp::bool_param("Set to true if you are performing an internal transfer to another Indodax user using their username instead of a blockchain address."),
                "memo": IndodaxMcp::str_param("Optional: A memo, tag, or payment ID required by certain assets (e.g., XRP, XLM) for correct routing to the destination wallet.", false, None),
                "network": IndodaxMcp::str_param("Optional: The specific blockchain network to use for the transfer (e.g., 'ERC20'). Ensure the destination address is compatible with the selected network.", false, None),
                "callback_url": IndodaxMcp::str_param("Optional: A webhook URL that Indodax will notify via a POST request once the withdrawal transaction is broadcast or completed.", false, None),
                "acknowledged":
                    IndodaxMcp::bool_param("Security confirmation: This must be explicitly set to true to authorize the withdrawal of funds from your account."),
            }),
            vec!["currency", "amount", "address", "acknowledged"],
        ),
        IndodaxMcp::tool_def(
            "deposit_address",
            "Retrieve or generate your personal deposit address for a specific cryptocurrency. This address is used to send funds from an external wallet or another exchange into your Indodax account. Always double-check the network and asset compatibility to avoid loss of funds.",
            serde_json::json!({
                "currency": IndodaxMcp::str_param("The cryptocurrency you intend to deposit (e.g., 'btc').", true, None),
                "network": IndodaxMcp::str_param("Optional: The specific blockchain network you will use for the deposit (e.g., 'BEP20').", false, None),
            }),
            vec!["currency"],
        ),
        IndodaxMcp::tool_def(
            "create_voucher",
            "Create an IDR voucher for a registered Indodax email. Partner-only dangerous operation that requires formal Indodax approval.",
            serde_json::json!({
                "amount": IndodaxMcp::num_param("Voucher value in IDR, minimum 1000.", true),
                "to_email": IndodaxMcp::str_param("Registered Indodax recipient email.", true, None),
                "acknowledged": IndodaxMcp::bool_param("Security confirmation: must be true to authorize voucher creation."),
            }),
            vec!["amount", "to_email", "acknowledged"],
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
            return Self::validation_error_result(format!(
                "Amount must be positive and finite, got {}",
                amount
            ));
        }
        if address.is_empty() {
            return Self::validation_error_result("Missing required parameter: address".into());
        }
        let params = crate::commands::helpers::build_withdraw_params(
            currency,
            amount,
            address,
            to_username,
            memo,
            network,
            callback_url,
        );

        match self
            .client
            .private_post_v1::<Value>("withdrawCoin", &params)
            .await
        {
            Ok(data) => Self::json_result(data),
            Err(e) => Self::error_from_indodax(&e),
        }
    }

    pub async fn handle_deposit_address(
        &self,
        currency: &str,
        network: Option<&str>,
    ) -> CallToolResult {
        if currency.is_empty() {
            return Self::validation_error_result("Missing required parameter: currency".into());
        }
        let mut params = HashMap::new();
        params.insert("currency".to_string(), currency.to_string());
        if let Some(n) = network {
            params.insert("network".to_string(), n.to_string());
        }

        match self
            .client
            .private_post_v1::<Value>("depositAddress", &params)
            .await
        {
            Ok(data) => Self::json_result(data),
            Err(e) => Self::error_from_indodax(&e),
        }
    }

    pub async fn handle_create_voucher(&self, amount: f64, to_email: &str) -> CallToolResult {
        if amount < 1000.0 || !amount.is_finite() {
            return Self::validation_error_result(format!(
                "amount must be at least 1000 IDR, got {}",
                amount
            ));
        }
        if to_email.trim().is_empty() {
            return Self::validation_error_result("Missing required parameter: to_email".into());
        }

        let mut params = HashMap::new();
        params.insert("amount".to_string(), amount.to_string());
        params.insert("to_email".to_string(), to_email.to_string());

        match self
            .client
            .private_post_v1::<Value>("createVoucher", &params)
            .await
        {
            Ok(data) => Self::json_result(data),
            Err(e) => Self::error_from_indodax(&e),
        }
    }
}
