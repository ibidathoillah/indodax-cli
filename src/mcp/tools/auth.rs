use serde_json::Value;
use rmcp::model::{CallToolResult, Tool};

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
                "No API credentials configured. Use environment variables or config file."
                    .to_string(),
            ),
        }
    }
}
