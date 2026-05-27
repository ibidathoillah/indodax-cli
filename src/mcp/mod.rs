pub mod safety;
pub mod service;
pub mod tools;

use crate::client::IndodaxClient;
use crate::config::IndodaxConfig;
use crate::errors::IndodaxError;
use rmcp::ServiceExt;
use service::ServiceGroup;
use tools::IndodaxMcp;

#[cfg(feature = "server")]
use axum::{
    extract::{Path, State},
    routing::{get, post},
    Json, Router,
    http::HeaderMap,
};
#[cfg(feature = "server")]
use serde_json::Value;

/// Run the MCP stdio server.
pub async fn run(
    groups_str: &str,
    allow_dangerous: bool,
    client: IndodaxClient,
    config: IndodaxConfig,
) -> Result<(), IndodaxError> {
    let enabled_groups = ServiceGroup::parse(groups_str)
        .map_err(|e| IndodaxError::Other(format!("Invalid service groups: {}", e)))?;
    let safety = safety::SafetyConfig::new(allow_dangerous);
    let mcp_server = IndodaxMcp::new(client, config, safety, enabled_groups);
    let service = mcp_server
        .serve(rmcp::transport::io::stdio())
        .await
        .map_err(|e| IndodaxError::Other(format!("MCP server error: {}", e)))?;
    tracing::info!("MCP server started with groups: {}, allow_dangerous: {}", groups_str, allow_dangerous);
    service.waiting().await.map_err(|e| IndodaxError::Other(format!("MCP server error: {}", e)))?;
    Ok(())
}

#[cfg(feature = "server")]
#[derive(Clone)]
pub struct AppState {
    pub groups: String,
    pub allow_dangerous: bool,
}

#[cfg(feature = "server")]
pub async fn run_http(
    port: u16,
    groups_str: &str,
    allow_dangerous: bool,
) -> Result<(), IndodaxError> {
    let state = AppState {
        groups: groups_str.to_string(),
        allow_dangerous,
    };

    let app = Router::new()
        .route("/health", get(|| async { "OK" }))
        .route("/call/:tool_name", post(handle_http_call))
        .with_state(state);

    let addr = std::net::SocketAddr::from(([0, 0, 0, 0], port));
    let listener = tokio::net::TcpListener::bind(addr).await
        .map_err(|e| IndodaxError::Other(format!("Failed to bind port: {}", e)))?;
    
    tracing::info!("MCP HTTP Server started on http://{}", addr);
    axum::serve(listener, app).await
        .map_err(|e| IndodaxError::Other(format!("Server error: {}", e)))?;
    Ok(())
}

#[cfg(feature = "server")]
async fn handle_http_call(
    State(state): State<AppState>,
    Path(tool_name): Path<String>,
    headers: HeaderMap,
    Json(arguments): Json<Value>,
) -> Json<Value> {
    let api_key = headers.get("x-api-key").and_then(|h| h.to_str().ok());
    let api_secret = headers.get("x-api-secret").and_then(|h| h.to_str().ok());

    let signer = match (api_key, api_secret) {
        (Some(k), Some(s)) => Some(crate::auth::Signer::new(k, s)),
        _ => None,
    };

    let client = crate::client::IndodaxClient::new(signer).unwrap();
    let config = crate::config::IndodaxConfig::default();
    let enabled_groups = ServiceGroup::parse(&state.groups).unwrap_or_else(|_| ServiceGroup::all());
    let safety = safety::SafetyConfig::new(state.allow_dangerous);
    let mcp = tools::IndodaxMcp::new(client, config, safety, enabled_groups);

    // Direct Tool Dispatching
    let args = arguments.as_object().cloned().unwrap_or_default();
    
    let result = match tool_name.as_str() {
        "ticker" => {
            let pair = crate::commands::helpers::normalize_pair(
                &IndodaxMcp::get_str(&args, "pair").unwrap_or_else(|| "btc_idr".into())
            );
            mcp.handle_ticker(&pair).await
        },
        "balance" => mcp.handle_balance().await,
        "account_info" => mcp.handle_account_info().await,
        "pairs" => mcp.handle_pairs().await,
        "server_time" => mcp.handle_server_time().await,
        "summaries" => mcp.handle_summaries().await,
        "open_orders" => {
            let pair = IndodaxMcp::get_str(&args, "pair").map(|p| crate::commands::helpers::normalize_pair(&p));
            mcp.handle_open_orders(pair.as_deref()).await
        },
        _ => rmcp::model::CallToolResult::error(vec![
            rmcp::model::Content::text(format!("Tool '{}' is not yet implemented in HTTP bridge or is unknown.", tool_name))
        ]),
    };

    Json(serde_json::to_value(result).unwrap())
}
