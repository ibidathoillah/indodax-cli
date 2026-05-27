use rmcp::model::{CallToolResult, Tool};
use serde_json::Value;

use super::IndodaxMcp;

pub fn auth_tools() -> Vec<Tool> {
    vec![
        IndodaxMcp::tool_def(
            "auth_show",
            "Show current API configuration status",
            serde_json::json!({}),
            vec![],
        ),
        IndodaxMcp::tool_def(
            "auth_test",
            "Test if current API credentials are valid",
            serde_json::json!({}),
            vec![],
        ),
        IndodaxMcp::tool_def(
            "auth_set",
            "Configure API credentials and callback URL",
            serde_json::json!({
                "api_key": IndodaxMcp::str_param("Indodax API key", true, None),
                "api_secret": IndodaxMcp::str_param("Indodax API secret", true, None),
                "callback_url": IndodaxMcp::str_param("Callback URL for WebSocket notifications (optional)", false, None),
                "test": IndodaxMcp::bool_param("Test credentials after saving"),
            }),
            vec!["api_key", "api_secret"],
        ),
    ]
}

impl IndodaxMcp {
    pub async fn handle_auth_show(&self) -> CallToolResult {
        let config = self.config.lock().await;
        Self::json_result(serde_json::json!({
            "api_key_set": config.api_key.is_some(),
            "api_secret_set": config.api_secret.is_some(),
            "callback_url": config.callback_url,
        }))
    }

    pub async fn handle_auth_test(&self) -> CallToolResult {
        match self.client.signer() {
            Some(_) => {
                match self
                    .client
                    .private_post_v1::<Value>("getInfo", &std::collections::HashMap::new())
                    .await
                {
                    Ok(data) => {
                        let name = data
                            .get("name")
                            .and_then(|v| v.as_str())
                            .unwrap_or("unknown");
                        Self::json_result(serde_json::json!({
                            "status": "ok",
                            "name": name,
                        }))
                    }
                    Err(e) => Self::error_from_indodax(&e),
                }
            }
            None => Self::error_result(
                "No API credentials configured. Use auth_set tool or set environment variables."
                    .to_string(),
            ),
        }
    }

    pub async fn handle_auth_set(
        &self,
        api_key: String,
        api_secret: String,
        callback_url: Option<String>,
        test: bool,
    ) -> CallToolResult {
        if api_key.trim().is_empty() {
            return Self::validation_error_result("api_key cannot be empty".into());
        }
        if api_secret.trim().is_empty() {
            return Self::validation_error_result("api_secret cannot be empty".into());
        }

        let mut config = self.config.lock().await;
        config.api_key = Some(crate::config::SecretValue::new(&api_key));
        config.api_secret = Some(crate::config::SecretValue::new(&api_secret));
        if let Some(url) = callback_url.filter(|u| !u.is_empty()) {
            config.callback_url = Some(url.clone());
        }

        if let Err(e) = config.save() {
            return Self::error_result(format!("Failed to save config: {}", e));
        }

        let mut result = serde_json::json!({
            "status": "saved",
            "message": "API credentials saved successfully",
        });

        if test {
            let test_client = {
                let signer = crate::auth::Signer::new(&api_key, &api_secret);
                match crate::client::IndodaxClient::new(Some(signer)) {
                    Ok(c) => c,
                    Err(e) => return Self::error_result(format!("Failed to create test client: {}", e)),
                }
            };

            match test_client
                .private_post_v1::<Value>("getInfo", &std::collections::HashMap::new())
                .await
            {
                Ok(data) => {
                    let name = data
                        .get("name")
                        .and_then(|v| v.as_str())
                        .unwrap_or("unknown");
                    let user_id = data
                        .get("user_id")
                        .and_then(|v| v.as_str())
                        .unwrap_or("unknown");
                    result["test_status"] = serde_json::json!("ok");
                    result["name"] = serde_json::json!(name);
                    result["user_id"] = serde_json::json!(user_id);
                }
                Err(e) => {
                    result["test_status"] = serde_json::json!("failed");
                    result["test_error"] = serde_json::json!(e.to_string());
                }
            }
        }

        Self::json_result(result)
    }
}
