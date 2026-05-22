use rmcp::model::{CallToolResult, Tool};

use super::IndodaxMcp;
use crate::commands::helpers;

pub fn websocket_tools() -> Vec<Tool> {
    vec![
        IndodaxMcp::tool_def(
            "ws_snapshot_ticker",
            "Get a real-time price snapshot via WebSocket (faster than REST)",
            serde_json::json!({
                "pair": IndodaxMcp::str_param("Trading pair, e.g. btc_idr", true, None),
            }),
            vec!["pair"],
        ),
        IndodaxMcp::tool_def(
            "ws_snapshot_book",
            "Get current order book snapshot via WebSocket",
            serde_json::json!({
                "pair": IndodaxMcp::str_param("Trading pair, e.g. btc_idr", true, None),
            }),
            vec!["pair"],
        ),
        IndodaxMcp::tool_def(
            "ws_snapshot_summary",
            "Get 24h summary snapshots for all pairs via WebSocket",
            serde_json::json!({}),
            vec![],
        ),
        IndodaxMcp::tool_def(
            "ws_token",
            "Generate a WebSocket token for external streaming clients",
            serde_json::json!({
                "private": IndodaxMcp::bool_param("Generate private token (requires auth)"),
            }),
            vec![],
        ),
    ]
}

const PUBLIC_WS_URL: &str = "wss://ws3.indodax.com/ws/";

impl IndodaxMcp {
    async fn fetch_ws_snapshot(
        &self,
        channel: &str,
    ) -> CallToolResult {
        let token = match helpers::fetch_public_ws_token(&self.client).await {
            Ok(t) => t,
            Err(e) => return Self::error_result(format!("Failed to fetch WS token: {}", e)),
        };

        let url = format!("{}?token={}", PUBLIC_WS_URL, token);
        match tokio::time::timeout(
            std::time::Duration::from_secs(10),
            self.ws_single_request(&url, channel),
        )
        .await
        {
            Ok(result) => result,
            Err(_) => Self::error_result("WebSocket request timed out after 10s".into()),
        }
    }

    async fn ws_single_request(
        &self,
        url: &str,
        channel: &str,
    ) -> CallToolResult {
        use futures_util::{SinkExt, StreamExt};
        use tokio_tungstenite::connect_async;
        use tokio_tungstenite::tungstenite::Message;

        let (mut ws_stream, _) = match connect_async(url).await {
            Ok(s) => s,
            Err(e) => return Self::error_result(format!("WebSocket connection failed: {}", e)),
        };

        let auth_msg = serde_json::json!({
            "params": { "token": self.ws_token_value().await },
            "id": 1
        });
        if let Err(e) = ws_stream.send(Message::Text(auth_msg.to_string())).await {
            return Self::error_result(format!("Failed to send auth: {}", e));
        }

        let mut authed = false;
        loop {
            tokio::select! {
                msg = ws_stream.next() => {
                    let msg = match msg {
                        Some(Ok(Message::Text(text))) => text,
                        Some(Ok(Message::Ping(data))) => {
                            let _ = ws_stream.send(Message::Pong(data)).await;
                            continue;
                        }
                        Some(Ok(Message::Close(_))) => {
                            return Self::error_result("WebSocket closed before data received".into());
                        }
                        Some(Err(e)) => return Self::error_result(format!("WebSocket error: {}", e)),
                        None => return Self::error_result("WebSocket stream ended unexpectedly".into()),
                        _ => continue,
                    };

                    let val: serde_json::Value = match serde_json::from_str(&msg) {
                        Ok(v) => v,
                        Err(e) => {
                            eprintln!("[MCP WS] JSON parse error: {}", e);
                            continue;
                        }
                    };

                    if !authed {
                        if val.get("id").and_then(|v| v.as_i64()) == Some(1)
                            && val.get("result").is_some()
                        {
                            authed = true;
                            let sub_msg = serde_json::json!({
                                "method": 1,
                                "params": { "channel": channel },
                                "id": 2
                            });
                            if let Err(e) = ws_stream.send(Message::Text(sub_msg.to_string())).await {
                                return Self::error_result(format!("Failed to subscribe: {}", e));
                            }
                        }
                        continue;
                    }

                    if val.get("id").and_then(|v| v.as_i64()) == Some(2) {
                        continue;
                    }

                    if val.get("result").is_some() {
                        return Self::json_result(val);
                    }
                }
                _ = tokio::time::sleep(std::time::Duration::from_secs(8)) => {
                    return Self::error_result("Timeout waiting for WebSocket data".into());
                }
            }
        }
    }

    async fn ws_token_value(&self) -> String {
        helpers::fetch_public_ws_token(&self.client)
            .await
            .unwrap_or_else(|_| helpers::DEFAULT_STATIC_WS_TOKEN.to_string())
    }

    pub async fn handle_ws_snapshot_ticker(&self, pair: &str) -> CallToolResult {
        let channel = format!("chart:tick-{}", pair);
        self.fetch_ws_snapshot(&channel).await
    }

    pub async fn handle_ws_snapshot_book(&self, pair: &str) -> CallToolResult {
        let channel = format!("market:order-book-{}", pair);
        self.fetch_ws_snapshot(&channel).await
    }

    pub async fn handle_ws_snapshot_summary(&self) -> CallToolResult {
        self.fetch_ws_snapshot("market:summary-24h").await
    }

    pub async fn handle_ws_token(&self, private: bool) -> CallToolResult {
        if private {
            match self.client.signer() {
                Some(_) => {
                    match self.client.generate_ws_token().await {
                        Ok((token, channel)) => Self::json_result(serde_json::json!({
                            "token": token,
                            "channel": channel,
                            "url": "wss://pws.indodax.com/ws/?cf_ws_frame_ping_pong=true",
                            "type": "private",
                        })),
                        Err(e) => Self::error_result(format!("Failed to generate private token: {}", e)),
                    }
                }
                None => Self::error_result(
                    "Private WebSocket token requires API credentials. Use auth_set first.".into()
                ),
            }
        } else {
            match helpers::fetch_public_ws_token(&self.client).await {
                Ok(token) => Self::json_result(serde_json::json!({
                    "token": token,
                    "url": "wss://ws3.indodax.com/ws/",
                    "type": "public",
                })),
                Err(e) => Self::error_result(format!("Failed to fetch public token: {}", e)),
            }
        }
    }
}
