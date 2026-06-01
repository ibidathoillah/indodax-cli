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
    http::{HeaderMap, Method, StatusCode},
};
#[cfg(feature = "server")]
use tower_http::cors::{Any, CorsLayer};
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
    pub bridge_secret: Option<String>,
}

#[cfg(feature = "server")]
pub async fn run_http(
    port: u16,
    groups_str: &str,
    allow_dangerous: bool,
) -> Result<(), IndodaxError> {
    // Load optional bridge secret for extra security
    let bridge_secret = std::env::var("BRIDGE_SECRET").ok();
    
    let state = AppState {
        groups: groups_str.to_string(),
        allow_dangerous,
        bridge_secret,
    };

    let cors = CorsLayer::new()
        .allow_methods([Method::GET, Method::POST])
        .allow_origin(Any)
        .allow_headers(Any);

    let app = Router::new()
        .route("/health", get(|| async { "OK" }))
        .route("/call/:tool_name", post(handle_http_call))
        .layer(cors)
        .with_state(state.clone());

    let addr = std::net::SocketAddr::from(([0, 0, 0, 0], port));
    let listener = tokio::net::TcpListener::bind(addr).await
        .map_err(|e| IndodaxError::Other(format!("Failed to bind port: {}", e)))?;
    
    tracing::info!("🚀 MCP HTTP Isolated Server started on http://{}", addr);
    if state.bridge_secret.is_some() {
        tracing::info!("🔒 Bridge Security Enabled: X-Bridge-Auth header required.");
    }
    
    axum::serve(listener, app).await
        .map_err(|e| IndodaxError::Other(format!("Server error: {}", e)))?;
    Ok(())
}

#[cfg(feature = "server")]
pub async fn handle_http_call(
    State(state): State<AppState>,
    Path(tool_name): Path<String>,
    headers: HeaderMap,
    Json(arguments): Json<Value>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    // 1. Security Check: Bridge Secret
    if let Some(secret) = &state.bridge_secret {
        let auth_header = headers.get("x-bridge-auth").and_then(|h| h.to_str().ok());
        if auth_header != Some(secret) {
            return Err((StatusCode::UNAUTHORIZED, Json(serde_json::json!({
                "error": true,
                "message": "Unauthorized: Invalid or missing X-Bridge-Auth header."
            }))));
        }
    }

    // 2. Extract User Credentials
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

    let args = arguments.as_object().cloned().unwrap_or_default();
    
    let result = match tool_name.as_str() {
        // --- Market Data ---
        "ticker" => {
            let pair = crate::commands::helpers::normalize_pair(&IndodaxMcp::get_str(&args, "pair").unwrap_or_else(|| "btc_idr".into()));
            mcp.handle_ticker(&pair).await
        },
        "orderbook" => {
            let pair = crate::commands::helpers::normalize_pair(&IndodaxMcp::get_str(&args, "pair").unwrap_or_else(|| "btc_idr".into()));
            mcp.handle_orderbook(&pair).await
        },
        "trades" => {
            let pair = crate::commands::helpers::normalize_pair(&IndodaxMcp::get_str(&args, "pair").unwrap_or_else(|| "btc_idr".into()));
            mcp.handle_trades(&pair).await
        },
        "ohlc" => {
            let symbol = crate::commands::helpers::normalize_pair(&IndodaxMcp::get_str(&args, "symbol").unwrap_or_else(|| "btc_idr".into())).replace('_', "").to_uppercase();
            let timeframe = IndodaxMcp::get_str(&args, "timeframe").unwrap_or_else(|| "60".into());
            let from = IndodaxMcp::get_num(&args, "from");
            let to = IndodaxMcp::get_num(&args, "to");
            mcp.handle_ohlc(&symbol, &timeframe, from, to).await
        },
        "pairs" => mcp.handle_pairs().await,
        "summaries" => mcp.handle_summaries().await,
        "server_time" => mcp.handle_server_time().await,
        "price_increments" => mcp.handle_price_increments().await,
        "orderbook_grouped" => {
            let pair = crate::commands::helpers::normalize_pair(&IndodaxMcp::get_str(&args, "pair").unwrap_or_else(|| "btc_idr".into()));
            let grouping = IndodaxMcp::get_num(&args, "grouping");
            let depth = IndodaxMcp::get_num(&args, "depth");
            mcp.handle_orderbook_grouped(&pair, grouping, depth).await
        },
        "spreads" => {
            let pair = crate::commands::helpers::normalize_pair(&IndodaxMcp::get_str(&args, "pair").unwrap_or_else(|| "btc_idr".into()));
            mcp.handle_spreads(&pair).await
        },
        
        // --- Account & Balance ---
        "balance" => mcp.handle_balance().await,
        "account_info" => mcp.handle_account_info().await,
        "portfolio_summary" => mcp.handle_portfolio_summary().await,
        "portfolio_allocation" => mcp.handle_portfolio_allocation().await,
        "open_orders" => {
            let pair = IndodaxMcp::get_str(&args, "pair").map(|p| crate::commands::helpers::normalize_pair(&p));
            mcp.handle_open_orders(pair.as_deref()).await
        },
        "order_history" => {
            let pair = crate::commands::helpers::normalize_pair(&IndodaxMcp::get_str(&args, "symbol").unwrap_or_else(|| "btc_idr".into()));
            let limit = IndodaxMcp::get_num(&args, "limit");
            mcp.handle_order_history(&pair, limit).await
        },
        "trade_history" => {
            let symbol = crate::commands::helpers::normalize_pair(&IndodaxMcp::get_str(&args, "symbol").unwrap_or_else(|| "btc_idr".into()));
            let limit = IndodaxMcp::get_num(&args, "limit");
            mcp.handle_trade_history(&symbol, limit).await
        },
        "get_order" => {
            let id = IndodaxMcp::get_num(&args, "order_id").unwrap_or(0.0);
            let pair = crate::commands::helpers::normalize_pair(&IndodaxMcp::get_str(&args, "pair").unwrap_or_default());
            mcp.handle_get_order(id, &pair).await
        },
        "get_order_by_client_id" => {
            let client_order_id = IndodaxMcp::get_str(&args, "client_order_id").unwrap_or_default();
            mcp.handle_get_order_by_client_id(&client_order_id).await
        },
        "trans_history" => mcp.handle_trans_history().await,
        "equity_snap" => mcp.handle_equity_snap().await,
        "equity_history" => {
            let limit = IndodaxMcp::get_num(&args, "limit");
            mcp.handle_equity_history(limit).await
        },
        "list_downline" => mcp.handle_list_downline().await,
        "check_downline" => {
            let email = IndodaxMcp::get_str(&args, "email").unwrap_or_default();
            mcp.handle_check_downline(&email).await
        },
        
        // --- Trading ---
        "buy_preview" => {
            let pair = crate::commands::helpers::normalize_pair(&IndodaxMcp::get_str(&args, "pair").unwrap_or_default());
            let idr = IndodaxMcp::get_num(&args, "idr").unwrap_or(0.0);
            let price = IndodaxMcp::get_num(&args, "price");
            let stop_price = IndodaxMcp::get_num(&args, "stop_price");
            let client_order_id = IndodaxMcp::get_str(&args, "client_order_id");
            mcp.handle_buy_preview(&pair, idr, price, stop_price, client_order_id.as_deref()).await
        },
        "sell_preview" => {
            let pair = crate::commands::helpers::normalize_pair(&IndodaxMcp::get_str(&args, "pair").unwrap_or_default());
            let price = IndodaxMcp::get_num(&args, "price");
            let amount = IndodaxMcp::get_num(&args, "amount").unwrap_or(0.0);
            let stop_price = IndodaxMcp::get_num(&args, "stop_price");
            let client_order_id = IndodaxMcp::get_str(&args, "client_order_id");
            mcp.handle_sell_preview(&pair, price, amount, stop_price, client_order_id.as_deref()).await
        },
        "buy_order" => {
            let pair = crate::commands::helpers::normalize_pair(&IndodaxMcp::get_str(&args, "pair").unwrap_or_default());
            let idr = IndodaxMcp::get_num(&args, "idr").unwrap_or(0.0);
            let price = IndodaxMcp::get_num(&args, "price");
            let stop_price = IndodaxMcp::get_num(&args, "stop_price");
            let client_order_id = IndodaxMcp::get_str(&args, "client_order_id");
            mcp.handle_buy_order(&pair, idr, price, stop_price, client_order_id.as_deref()).await
        },
        "sell_order" => {
            let pair = crate::commands::helpers::normalize_pair(&IndodaxMcp::get_str(&args, "pair").unwrap_or_default());
            let price = IndodaxMcp::get_num(&args, "price");
            let amount = IndodaxMcp::get_num(&args, "amount").unwrap_or(0.0);
            let stop_price = IndodaxMcp::get_num(&args, "stop_price");
            let client_order_id = IndodaxMcp::get_str(&args, "client_order_id");
            let order_type = IndodaxMcp::get_str(&args, "order_type").unwrap_or_else(|| "limit".into());
            mcp.handle_sell_order(&pair, price, amount, &order_type, stop_price, client_order_id.as_deref()).await
        },
        "cancel_order" => {
            let id = IndodaxMcp::get_num(&args, "order_id").unwrap_or(0.0);
            let pair = crate::commands::helpers::normalize_pair(&IndodaxMcp::get_str(&args, "pair").unwrap_or_default());
            let order_type = IndodaxMcp::get_str(&args, "order_type").unwrap_or_default();
            mcp.handle_cancel_order(id, &pair, &order_type).await
        },
        "cancel_all_orders" => {
            let pair = IndodaxMcp::get_str(&args, "pair").map(|p| crate::commands::helpers::normalize_pair(&p));
            mcp.handle_cancel_all_orders(pair.as_deref()).await
        },

        // --- Funding ---
        "withdraw" => {
            let currency = IndodaxMcp::get_str(&args, "currency").unwrap_or_default();
            let amount = IndodaxMcp::get_num(&args, "amount").unwrap_or(0.0);
            let address = IndodaxMcp::get_str(&args, "address").unwrap_or_default();
            let to_username = IndodaxMcp::get_bool(&args, "to_username");
            let memo = IndodaxMcp::get_str(&args, "memo");
            let network = IndodaxMcp::get_str(&args, "network");
            let callback = IndodaxMcp::get_str(&args, "callback_url");
            mcp.handle_withdraw(&currency, amount, &address, to_username, memo.as_deref(), network.as_deref(), callback.as_deref()).await
        },
        "deposit_address" => {
            let currency = IndodaxMcp::get_str(&args, "currency").unwrap_or_default();
            let network = IndodaxMcp::get_str(&args, "network");
            mcp.handle_deposit_address(&currency, network.as_deref()).await
        },

        // --- Paper Trading ---
        "paper_init" => {
            let idr = IndodaxMcp::get_num(&args, "idr");
            let btc = IndodaxMcp::get_num(&args, "btc");
            mcp.handle_paper_init(idr, btc).await
        },
        "paper_balance" => mcp.handle_paper_balance().await,
        "paper_buy" | "paper_sell" => {
            let pair = crate::commands::helpers::normalize_pair(&IndodaxMcp::get_str(&args, "pair").unwrap_or_else(|| "btc_idr".into()));
            let price = IndodaxMcp::get_num(&args, "price");
            let amount = IndodaxMcp::get_num(&args, "amount");
            let idr = IndodaxMcp::get_num(&args, "idr");
            let side = if tool_name == "paper_buy" { "buy" } else { "sell" };
            mcp.handle_paper_trade(side, &pair, price, amount, idr).await
        },
        "paper_status" => mcp.handle_paper_status().await,
        "paper_history" => mcp.handle_paper_history().await,
        "paper_orders" => mcp.handle_paper_orders().await,

        // --- Alerts ---
        "alert_add" => {
            let pair = IndodaxMcp::get_str(&args, "pair").unwrap_or_default();
            let above = IndodaxMcp::get_num(&args, "above");
            let below = IndodaxMcp::get_num(&args, "below");
            let note = IndodaxMcp::get_str(&args, "note");
            mcp.handle_alert_add(&pair, above, below, None, None, note).await
        },
        "alert_list" => mcp.handle_alert_list(IndodaxMcp::get_bool(&args, "history")).await,
        "alert_cancel" => {
            let id = IndodaxMcp::get_num(&args, "id");
            let all = IndodaxMcp::get_bool(&args, "all");
            mcp.handle_alert_cancel(id, all).await
        },

        _ => rmcp::model::CallToolResult::error(vec![
            rmcp::model::Content::text(format!("Tool '{}' is not yet implemented in HTTP bridge.", tool_name))
        ]),
    };

    Ok(Json(serde_json::to_value(result).unwrap()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_service_group_parsing() {
        let groups = ServiceGroup::parse("market,account").unwrap();
        assert_eq!(groups.len(), 2);
        assert!(groups.contains(&ServiceGroup::Market));
        assert!(groups.contains(&ServiceGroup::Account));
    }

    #[test]
    fn test_app_state_clonable() {
        let state = AppState {
            groups: "all".into(),
            allow_dangerous: true,
            bridge_secret: Some("secret".into()),
        };
        let cloned = state.clone();
        assert_eq!(cloned.groups, "all");
    }
}
