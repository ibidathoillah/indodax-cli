pub mod account;
pub mod alert;
pub mod auth;
pub mod funding;
pub mod market;
pub mod paper;
pub mod trade;
pub mod websocket;

use std::sync::Arc;

use tokio::sync::Mutex;
use serde_json::{Map, Value};

use rmcp::model::{
    CallToolRequestParams, CallToolResult, Content, Implementation, InitializeResult,
    ListToolsResult, PaginatedRequestParams, ServerCapabilities, Tool,
    ListResourcesResult, Resource, ReadResourceResult, ListPromptsResult,
    GetPromptRequestParams,
};
use rmcp::service::{RequestContext, RoleServer};
use rmcp::ErrorData as McpError;

use crate::commands::helpers;
use crate::config::IndodaxConfig;
use crate::errors::IndodaxError;
use crate::mcp::safety::SafetyConfig;
use crate::mcp::service::ServiceGroup;
use crate::client::IndodaxClient;

/// The MCP server exposing Indodax trading functionality as tools.
#[derive(Debug, Clone)]
pub struct IndodaxMcp {
    pub client: Arc<IndodaxClient>,
    pub config: Arc<Mutex<IndodaxConfig>>,
    pub safety: SafetyConfig,
    pub enabled_groups: Vec<ServiceGroup>,
    pub paper_state: Arc<tokio::sync::Mutex<Option<crate::commands::paper::PaperState>>>,
}

impl IndodaxMcp {
    pub fn new(
        client: IndodaxClient,
        config: IndodaxConfig,
        safety: SafetyConfig,
        enabled_groups: Vec<ServiceGroup>,
    ) -> Self {
        Self {
            client: Arc::new(client),
            config: Arc::new(Mutex::new(config)),
            safety,
            enabled_groups,
            paper_state: Arc::new(tokio::sync::Mutex::new(None)),
        }
    }

    pub fn is_group_enabled(&self, group: &ServiceGroup) -> bool {
        self.enabled_groups.contains(group)
    }

    async fn load_paper_state(&self) -> crate::commands::paper::PaperState {
        let mut state_guard = self.paper_state.lock().await;
        if let Some(state) = state_guard.as_ref() {
            return state.clone();
        }
        let config = self.config.lock().await;
        let state = crate::commands::paper::PaperState::load(&config);
        *state_guard = Some(state.clone());
        state
    }

    async fn save_paper_state(
        &self,
        state: &crate::commands::paper::PaperState,
    ) -> Result<(), IndodaxError> {
        let mut state_guard = self.paper_state.lock().await;
        *state_guard = Some(state.clone());
        state.save()
    }

    pub fn str_param(description: &str, required: bool, default_: Option<&str>) -> Value {
        let mut schema = serde_json::json!({
            "type": "string",
            "description": description,
        });
        if let Some(d) = default_ {
            schema["default"] = Value::String(d.to_string());
        }
        if required {
            schema["required"] = Value::Bool(true);
        }
        schema
    }

    pub fn num_param(description: &str, required: bool) -> Value {
        let mut schema = serde_json::json!({
            "type": "number",
            "description": description,
        });
        if required {
            schema["required"] = Value::Bool(true);
        }
        schema
    }

    pub fn bool_param(description: &str) -> Value {
        serde_json::json!({
            "type": "boolean",
            "description": description,
        })
    }

    /// Helper to get account info from Indodax API.
    pub async fn get_account_info(&self) -> Result<Value, IndodaxError> {
        self.client
            .private_post_v1::<Value>("getInfo", &std::collections::HashMap::new())
            .await
    }

    pub fn tool_def(name: &str, description: &str, properties: Value, required: Vec<&str>) -> Tool {
        let mut schema = Map::new();
        schema.insert("type".to_string(), Value::String("object".to_string()));

        if let Value::Object(props) = properties {
            if !props.is_empty() {
                schema.insert("properties".to_string(), Value::Object(props));
            }
        }

        if !required.is_empty() {
            let req_values: Vec<Value> = required
                .iter()
                .map(|s| Value::String(s.to_string()))
                .collect();
            schema.insert("required".to_string(), Value::Array(req_values));
        }

        Tool::new(name.to_string(), description.to_string(), Arc::new(schema))
    }

    pub fn get_str(args: &Map<String, Value>, name: &str) -> Option<String> {
        args.get(name)
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
    }

    pub fn get_num(args: &Map<String, Value>, name: &str) -> Option<f64> {
        args.get(name).and_then(|v| {
            v.as_f64()
                .or_else(|| v.as_str().and_then(|s| s.parse::<f64>().ok()))
        })
    }

    pub fn get_bool(args: &Map<String, Value>, name: &str) -> bool {
        Self::get_opt_bool(args, name).unwrap_or(false)
    }

    pub fn get_opt_bool(args: &Map<String, Value>, name: &str) -> Option<bool> {
        args.get(name).and_then(|v| v.as_bool())
    }

    pub fn ok_result(text: String) -> CallToolResult {
        CallToolResult::success(vec![Content::text(text)])
    }

    pub fn error_result(text: String) -> CallToolResult {
        let envelope = serde_json::json!({
            "error": true,
            "message": text,
            "error_type": "mcp_error",
        });
        CallToolResult::error(vec![Content::text(
            serde_json::to_string_pretty(&envelope).unwrap_or(text),
        )])
    }

    pub fn validation_error_result(text: String) -> CallToolResult {
        let envelope = serde_json::json!({
            "error": true,
            "message": text,
            "error_type": "validation_error",
        });
        CallToolResult::error(vec![Content::text(
            serde_json::to_string_pretty(&envelope).unwrap_or(text),
        )])
    }

    pub fn error_from_indodax(err: &IndodaxError) -> CallToolResult {
        let envelope = serde_json::json!({
            "error": true,
            "message": err.to_string(),
            "error_type": err.category(),
        });
        CallToolResult::error(vec![Content::text(
            serde_json::to_string_pretty(&envelope).unwrap_or_else(|_| err.to_string()),
        )])
    }

    pub fn json_result(value: Value) -> CallToolResult {
        let text = serde_json::to_string_pretty(&value).unwrap_or_default();
        Self::ok_result(text)
    }

    pub fn json_result_with_warning(mut value: Value, warning: Option<String>) -> CallToolResult {
        if let Some(w) = warning {
            if let Some(obj) = value.as_object_mut() {
                obj.insert("warning".to_string(), Value::String(w));
            }
        }
        Self::json_result(value)
    }
}

impl rmcp::handler::server::ServerHandler for IndodaxMcp {
    fn get_info(&self) -> InitializeResult {
        InitializeResult::new(
            ServerCapabilities::builder()
                .enable_tools()
                .enable_resources()
                .enable_prompts()
                .build(),
        )
        .with_server_info(Implementation::new(
            "indodax-cli",
            env!("CARGO_PKG_VERSION"),
        ))
    }

    async fn list_resources(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListResourcesResult, McpError> {
        let resources = vec![
            Resource::new(
                rmcp::model::RawResource::new("config://current", "Current API config"),
                None,
            ),
            Resource::new(
                rmcp::model::RawResource::new("pairs://list", "Available trading pairs"),
                None,
            ),
            Resource::new(
                rmcp::model::RawResource::new("paper://state", "Paper trading state"),
                None,
            ),
        ];
        Ok(ListResourcesResult::with_all_items(resources))
    }

    async fn read_resource(
        &self,
        request: rmcp::model::ReadResourceRequestParams,
        _context: RequestContext<RoleServer>,
    ) -> Result<ReadResourceResult, McpError> {
        let uri = request.uri.as_str();
        let content = match uri {
            "config://current" => {
                let config = self.config.lock().await;
                serde_json::json!({
                    "api_key_set": config.api_key.is_some(),
                    "api_secret_set": config.api_secret.is_some(),
                    "callback_url": config.callback_url,
                })
            }
            "pairs://list" => {
                match self.client.public_get::<Value>("/api/pairs").await {
                    Ok(data) => data,
                    Err(e) => serde_json::json!({"error": e.to_string()}),
                }
            }
            "paper://state" => {
                let state = self.load_paper_state().await;
                serde_json::json!({
                    "balances": state.balances,
                    "open_orders_count": state.orders.len(),
                    "trade_count": state.trade_count,
                })
            }
            _ => {
                return Err(McpError::invalid_params(
                    format!("Unknown resource URI: {}", uri),
                    None,
                ));
            }
        };

        let text = serde_json::to_string_pretty(&content).unwrap_or_default();
        let text_content = rmcp::model::ResourceContents::TextResourceContents {
            uri: request.uri,
            mime_type: Some("application/json".to_string()),
            text,
            meta: None,
        };
        Ok(ReadResourceResult::new(vec![text_content]))
    }

    async fn list_prompts(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListPromptsResult, McpError> {
        let prompts = vec![
            rmcp::model::Prompt::new(
                "create_order",
                Some("Generate a buy or sell order with proper parameters and safety checks".to_string()),
                Some(vec![
                    rmcp::model::PromptArgument::new("side"),
                    rmcp::model::PromptArgument::new("pair"),
                    rmcp::model::PromptArgument::new("price"),
                    rmcp::model::PromptArgument::new("amount"),
                    rmcp::model::PromptArgument::new("idr"),
                ]),
            ),
            rmcp::model::Prompt::new(
                "check_portfolio",
                Some("Get account balance and open orders summary".to_string()),
                None,
            ),
            rmcp::model::Prompt::new(
                "analyze_market",
                Some("Analyze market conditions for a trading pair".to_string()),
                Some(vec![
                    rmcp::model::PromptArgument::new("pair"),
                ]),
            ),
        ];
        Ok(ListPromptsResult::with_all_items(prompts))
    }

    async fn get_prompt(
        &self,
        request: GetPromptRequestParams,
        _context: RequestContext<RoleServer>,
    ) -> Result<rmcp::model::GetPromptResult, McpError> {
        let name = request.name.as_str();
        let args = request.arguments.unwrap_or_default();

        let description = match name {
            "create_order" => {
                let side = args.get("side").and_then(|v| v.as_str()).unwrap_or("buy");
                let pair = args.get("pair").and_then(|v| v.as_str()).unwrap_or("btc_idr");
                let price = args.get("price").and_then(|v| v.as_str()).unwrap_or("");
                let amount = args.get("amount").and_then(|v| v.as_str()).unwrap_or("");
                let idr = args.get("idr").and_then(|v| v.as_str()).unwrap_or("");

                if side == "buy" {
                    format!(
                        "Place a buy order for {} pair. Use the buy_order tool. \
                         Pair: {}. {}{}{}. \
                         IMPORTANT: Set acknowledged=true to confirm this is intentional.",
                        pair,
                        pair,
                        if !price.is_empty() { format!("Price: {}. ", price) } else { String::new() },
                        if !idr.is_empty() { format!("IDR amount: {}. ", idr) } else { String::new() },
                        if price.is_empty() { "This will be a market order (no price specified). " } else { "" }
                    )
                } else {
                    format!(
                        "Place a sell order for {} pair. Use the sell_order tool. \
                         Pair: {}. {}{} \
                         IMPORTANT: Set acknowledged=true to confirm this is intentional.",
                        pair,
                        pair,
                        if !price.is_empty() { format!("Price: {}. ", price) } else { String::new() },
                        if !amount.is_empty() { format!("Amount: {}. ", amount) } else { "Amount is required for sell orders. ".to_string() }
                    )
                }
            }
            "check_portfolio" => {
                "Call account_info to get all balances, then call open_orders to see all open orders. \
                 Summarize the results showing: total IDR balance, crypto balances (non-zero only), \
                 and count of open orders per pair."
                    .to_string()
            }
            "analyze_market" => {
                let pair = args.get("pair").and_then(|v| v.as_str()).unwrap_or("btc_idr");
                let normalized = crate::commands::helpers::normalize_pair(pair);
                format!(
                    "Analyze market for {} pair:\n\
                     1. Call ticker to get current price and 24h stats\n\
                     2. Call orderbook to see bid/ask spread and depth\n\
                     3. Call trades to see recent trade activity\n\
                     Provide a summary with: current price, 24h change, bid-ask spread, \
                     and recent trade volume trend.",
                    normalized
                )
            }
            _ => {
                return Err(McpError::invalid_params(
                    format!("Unknown prompt: {}", name),
                    None,
                ));
            }
        };

        let messages = vec![rmcp::model::PromptMessage::new_text(
            rmcp::model::PromptMessageRole::User,
            description,
        )];
        Ok(rmcp::model::GetPromptResult::new(messages))
    }

    async fn list_tools(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListToolsResult, McpError> {
        let tools = self.all_tools();
        Ok(ListToolsResult::with_all_items(tools))
    }

    async fn call_tool(
        &self,
        request: CallToolRequestParams,
        _context: RequestContext<RoleServer>,
    ) -> Result<CallToolResult, McpError> {
        let name = request.name.to_string();
        let args = request.arguments.unwrap_or_default();

        let result = match name.as_str() {
            // Market (public, no auth needed)
            "server_time" => self.handle_server_time().await,
            "ticker" => {
                let pair = helpers::normalize_pair(
                    &Self::get_str(&args, "pair").unwrap_or_else(|| "btc_idr".into())
                );
                self.handle_ticker(&pair).await
            }
            "ticker_all" => self.handle_ticker_all().await,
            "pairs" => self.handle_pairs().await,
            "summaries" => self.handle_summaries().await,
            "orderbook" => {
                let pair = helpers::normalize_pair(
                    &Self::get_str(&args, "pair").unwrap_or_else(|| "btc_idr".into())
                );
                self.handle_orderbook(&pair).await
            }
            "trades" => {
                let pair = helpers::normalize_pair(
                    &Self::get_str(&args, "pair").unwrap_or_else(|| "btc_idr".into())
                );
                self.handle_trades(&pair).await
            }
            "ohlc" => {
                let symbol = helpers::normalize_pair(
                    &Self::get_str(&args, "symbol").unwrap_or_else(|| "btc_idr".into())
                ).replace('_', "").to_uppercase();
                let timeframe =
                    Self::get_str(&args, "timeframe").unwrap_or_else(|| "60".into());
                let from = Self::get_num(&args, "from");
                let to = Self::get_num(&args, "to");
                self.handle_ohlc(&symbol, &timeframe, from, to).await
            }
            "price_increments" => self.handle_price_increments().await,

            // Trade
            "buy_order" => {
                let acknowledged = Self::get_bool(&args, "acknowledged");
                if let Err(msg) =
                    self.safety.check_operation(&ServiceGroup::Trade, acknowledged)
                {
                    return Ok(Self::error_result(msg));
                }
                let pair = helpers::normalize_pair(
                    &Self::get_str(&args, "pair").unwrap_or_default()
                );
                let idr = Self::get_num(&args, "idr").unwrap_or(0.0);
                let price = Self::get_num(&args, "price");
                self.handle_buy_order(&pair, idr, price).await
            }
            "sell_order" => {
                let acknowledged = Self::get_bool(&args, "acknowledged");
                if let Err(msg) =
                    self.safety.check_operation(&ServiceGroup::Trade, acknowledged)
                {
                    return Ok(Self::error_result(msg));
                }
                let pair = helpers::normalize_pair(
                    &Self::get_str(&args, "pair").unwrap_or_default()
                );
                let price = Self::get_num(&args, "price");
                let amount = match Self::get_num(&args, "amount") {
                    Some(v) if v > 0.0 => v,
                    Some(v) => return Ok(Self::validation_error_result(format!("Amount must be positive, got {}", v))),
                    None => return Ok(Self::validation_error_result("Missing required parameter: amount".into())),
                };
                let order_type = Self::get_str(&args, "order_type").unwrap_or_else(|| "limit".into());
                self.handle_sell_order(&pair, price, amount, &order_type).await
            }

            // Account
            "account_info" => self.handle_account_info().await,
            "balance" => self.handle_balance().await,
            "open_orders" => {
                let pair = Self::get_str(&args, "pair").map(|p| helpers::normalize_pair(&p));
                self.handle_open_orders(pair.as_deref()).await
            }
            "order_history" => {
                let symbol = helpers::normalize_pair(
                    &Self::get_str(&args, "symbol").unwrap_or_else(|| "btc_idr".into())
                );
                let limit = Self::get_num(&args, "limit");
                self.handle_order_history(&symbol, limit).await
            }
            "trade_history" => {
                let symbol = helpers::normalize_pair(
                    &Self::get_str(&args, "symbol").unwrap_or_else(|| "btc_idr".into())
                );
                let limit = Self::get_num(&args, "limit");
                self.handle_trade_history(&symbol, limit).await
            }
            "get_order" => {
                let order_id = match Self::get_num(&args, "order_id") {
                    Some(v) => {
                        if v.fract() != 0.0 {
                            return Ok(Self::validation_error_result(format!("order_id must be a whole number, got {}", v)));
                        }
                        v
                    }
                    None => return Ok(Self::validation_error_result("Missing required parameter: order_id".into())),
                };
                let pair = match Self::get_str(&args, "pair") {
                    Some(v) => helpers::normalize_pair(&v),
                    None => return Ok(Self::validation_error_result("Missing required parameter: pair".into())),
                };
                self.handle_get_order(order_id, &pair).await
            }
            "trans_history" => self.handle_trans_history().await,
            "cancel_order" => {
                let acknowledged = Self::get_bool(&args, "acknowledged");
                if let Err(msg) =
                    self.safety.check_operation(&ServiceGroup::Trade, acknowledged)
                {
                    return Ok(Self::error_result(msg));
                }
                let order_id = match Self::get_num(&args, "order_id") {
                    Some(v) => {
                        if v.fract() != 0.0 {
                            return Ok(Self::validation_error_result(format!("order_id must be a whole number, got {}", v)));
                        }
                        v
                    }
                    None => return Ok(Self::validation_error_result("Missing required parameter: order_id".into())),
                };
                let pair = match Self::get_str(&args, "pair") {
                    Some(v) => helpers::normalize_pair(&v),
                    None => return Ok(Self::validation_error_result("Missing required parameter: pair".into())),
                };
                let order_type = Self::get_str(&args, "order_type").unwrap_or_default();
                self.handle_cancel_order(order_id, &pair, &order_type).await
            }
            "cancel_all_orders" => {
                let acknowledged = Self::get_bool(&args, "acknowledged");
                if let Err(msg) =
                    self.safety.check_operation(&ServiceGroup::Trade, acknowledged)
                {
                    return Ok(Self::error_result(msg));
                }
                let pair = Self::get_str(&args, "pair")
                    .map(|p| helpers::normalize_pair(&p));
                self.handle_cancel_all_orders(pair.as_deref()).await
            }

            // Funding
            "withdraw_fee" => {
                let currency = Self::get_str(&args, "currency").unwrap_or_default();
                let network = Self::get_str(&args, "network");
                self.handle_withdraw_fee(&currency, network.as_deref())
                    .await
            }
            "withdraw" => {
                let acknowledged = Self::get_bool(&args, "acknowledged");
                if let Err(msg) =
                    self.safety.check_operation(&ServiceGroup::Funding, acknowledged)
                {
                    return Ok(Self::error_result(msg));
                }
                let currency = Self::get_str(&args, "currency").unwrap_or_default();
                let amount = Self::get_num(&args, "amount").unwrap_or(0.0);
                let address = Self::get_str(&args, "address").unwrap_or_default();
                let to_username = Self::get_bool(&args, "to_username");
                let memo = Self::get_str(&args, "memo");
                let network = Self::get_str(&args, "network");
                let callback_url = Self::get_str(&args, "callback_url");
                self.handle_withdraw(
                    &currency,
                    amount,
                    &address,
                    to_username,
                    memo.as_deref(),
                    network.as_deref(),
                    callback_url.as_deref(),
                )
                .await
            }
            "deposit_address" => {
                let currency = Self::get_str(&args, "currency").unwrap_or_default();
                let network = Self::get_str(&args, "network");
                self.handle_deposit_address(&currency, network.as_deref()).await
            }

            // Paper
            "paper_init" => {
                let idr = Self::get_num(&args, "idr");
                let btc = Self::get_num(&args, "btc");
                self.handle_paper_init(idr, btc).await
            }
            "paper_reset" => self.handle_paper_reset().await,
            "paper_balance" => self.handle_paper_balance().await,
            "paper_buy" | "paper_sell" => {
                let pair = helpers::normalize_pair(
                    &Self::get_str(&args, "pair")
                        .unwrap_or_else(|| "btc_idr".into())
                );
                let price = Self::get_num(&args, "price");
                let amount = Self::get_num(&args, "amount");
                let idr = Self::get_num(&args, "idr");
                let side = if name == "paper_buy" {
                    "buy"
                } else {
                    "sell"
                };
                self.handle_paper_trade(side, &pair, price, amount, idr).await
            }
            "paper_orders" => self.handle_paper_orders().await,
            "paper_cancel" => {
                let order_id = match Self::get_num(&args, "order_id") {
                    Some(v) => {
                        if v.fract() != 0.0 {
                            return Ok(Self::validation_error_result(format!("order_id must be a whole number, got {}", v)));
                        }
                        v as u64
                    }
                    None => return Ok(Self::validation_error_result("Missing required parameter: order_id".into())),
                };
                self.handle_paper_cancel(order_id).await
            }
            "paper_cancel_all" => self.handle_paper_cancel_all().await,
            "paper_history" => self.handle_paper_history().await,
            "paper_status" => self.handle_paper_status().await,
            "paper_fill" => {
                let order_id = Self::get_num(&args, "order_id");
                let price = Self::get_num(&args, "price");
                let all = Self::get_bool(&args, "all");
                let fetch = Self::get_bool(&args, "fetch");
                self.handle_paper_fill(order_id, price, all, fetch).await
            }
            "paper_check_fills" => {
                let prices = Self::get_str(&args, "prices");
                let fetch = Self::get_bool(&args, "fetch");
                self.handle_paper_check_fills(prices.as_deref(), fetch).await
            }

            // Auth
            "auth_show" => self.handle_auth_show().await,
            "auth_test" => self.handle_auth_test().await,
            "auth_set" => {
                let api_key = Self::get_str(&args, "api_key").unwrap_or_default();
                let api_secret = Self::get_str(&args, "api_secret").unwrap_or_default();
                let callback_url = Self::get_str(&args, "callback_url");
                let test = Self::get_bool(&args, "test");
                self.handle_auth_set(api_key, api_secret, callback_url, test).await
            },

            // Alert
            "alert_add" => {
                let pair = Self::get_str(&args, "pair").unwrap_or_default();
                let above = Self::get_num(&args, "above");
                let below = Self::get_num(&args, "below");
                let percent_up = Self::get_num(&args, "percent_up");
                let percent_down = Self::get_num(&args, "percent_down");
                let note = Self::get_str(&args, "note");
                self.handle_alert_add(&pair, above, below, percent_up, percent_down, note).await
            }
            "alert_list" => {
                let history = Self::get_bool(&args, "history");
                self.handle_alert_list(history).await
            }
            "alert_cancel" => {
                let id = Self::get_num(&args, "id");
                let all = Self::get_bool(&args, "all");
                self.handle_alert_cancel(id, all).await
            }
            "alert_check" => {
                let id = Self::get_num(&args, "id");
                let pair = Self::get_str(&args, "pair");
                self.handle_alert_check(id, pair).await
            }

            // WebSocket snapshots (Market group)
            "ws_snapshot_ticker" => {
                let pair = helpers::normalize_pair(
                    &Self::get_str(&args, "pair").unwrap_or_else(|| "btc_idr".into())
                );
                self.handle_ws_snapshot_ticker(&pair).await
            }
            "ws_snapshot_book" => {
                let pair = helpers::normalize_pair(
                    &Self::get_str(&args, "pair").unwrap_or_else(|| "btc_idr".into())
                );
                self.handle_ws_snapshot_book(&pair).await
            }
            "ws_snapshot_summary" => self.handle_ws_snapshot_summary().await,
            "ws_token" => {
                let private = Self::get_bool(&args, "private");
                self.handle_ws_token(private).await
            }

            _ => Self::error_result(format!("Unknown tool: {}", name)),
        };

        Ok(result)
    }
}

fn all_tools(mcp: &IndodaxMcp) -> Vec<Tool> {
    let mut tools: Vec<Tool> = Vec::new();

    if mcp.is_group_enabled(&ServiceGroup::Market) {
        tools.extend(market::market_tools());
    }
    if mcp.is_group_enabled(&ServiceGroup::Account) {
        tools.extend(account::account_tools());
    }
    if mcp.is_group_enabled(&ServiceGroup::Trade) {
        if let Err(msg) = mcp.safety.check_group(&ServiceGroup::Trade) {
            eprintln!("[MCP] Warning: {}", msg);
        }
        tools.extend(trade::trade_tools());
    }
    if mcp.is_group_enabled(&ServiceGroup::Funding) {
        if let Err(msg) = mcp.safety.check_group(&ServiceGroup::Funding) {
            eprintln!("[MCP] Warning: {}", msg);
        }
        tools.extend(funding::funding_tools());
    }
    if mcp.is_group_enabled(&ServiceGroup::Paper) {
        tools.extend(paper::paper_tools());
    }
    if mcp.is_group_enabled(&ServiceGroup::Auth) {
        tools.extend(auth::auth_tools());
    }
    if mcp.is_group_enabled(&ServiceGroup::Alert) {
        tools.extend(alert::alert_tools());
    }
    if mcp.is_group_enabled(&ServiceGroup::Market) {
        tools.extend(websocket::websocket_tools());
    }

    tools
}

impl IndodaxMcp {
    pub fn all_tools(&self) -> Vec<Tool> {
        all_tools(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use crate::client::IndodaxClient;
    use crate::config::IndodaxConfig;

fn test_mcp() -> IndodaxMcp {
        let client = IndodaxClient::new(None).unwrap();
        let config = IndodaxConfig::default();
        let safety = SafetyConfig::new(false);
        let groups = vec![ServiceGroup::Market, ServiceGroup::Paper, ServiceGroup::Alert];
        IndodaxMcp::new(client, config, safety, groups)
    }

    #[test]
    fn test_get_str() {
        let mut args = Map::new();
        args.insert("name".into(), json!("test_value"));
        assert_eq!(IndodaxMcp::get_str(&args, "name"), Some("test_value".into()));
        assert_eq!(IndodaxMcp::get_str(&args, "missing"), None);
    }

    #[test]
    fn test_get_num_from_number() {
        let mut args = Map::new();
        args.insert("price".into(), json!(100.5));
        assert_eq!(IndodaxMcp::get_num(&args, "price"), Some(100.5));
    }

    #[test]
    fn test_get_num_from_string() {
        let mut args = Map::new();
        args.insert("amount".into(), json!("50.25"));
        assert_eq!(IndodaxMcp::get_num(&args, "amount"), Some(50.25));
    }

    #[test]
    fn test_get_num_missing() {
        let args = Map::new();
        assert_eq!(IndodaxMcp::get_num(&args, "missing"), None);
    }

    #[test]
    fn test_get_bool_true() {
        let mut args = Map::new();
        args.insert("acknowledged".into(), json!(true));
        assert!(IndodaxMcp::get_bool(&args, "acknowledged"));
    }

    #[test]
    fn test_get_bool_false() {
        let mut args = Map::new();
        args.insert("flag".into(), json!(false));
        assert!(!IndodaxMcp::get_bool(&args, "flag"));
    }

    #[test]
    fn test_get_bool_missing_defaults_false() {
        let args = Map::new();
        assert!(!IndodaxMcp::get_bool(&args, "missing"));
    }

    #[test]
    fn test_get_opt_bool() {
        let mut args = Map::new();
        args.insert("true_val".into(), json!(true));
        args.insert("false_val".into(), json!(false));
        
        assert_eq!(IndodaxMcp::get_opt_bool(&args, "true_val"), Some(true));
        assert_eq!(IndodaxMcp::get_opt_bool(&args, "false_val"), Some(false));
        assert_eq!(IndodaxMcp::get_opt_bool(&args, "missing"), None);
    }

    #[test]
    fn test_tool_def_creates_tool() {
        let properties = serde_json::json!({
            "pair": {
                "type": "string",
                "description": "Trading pair"
            }
        });
        let tool = IndodaxMcp::tool_def("test_tool", "A test tool", properties, vec!["pair"]);
        assert_eq!(tool.name.to_string(), "test_tool");
        assert!(tool.description.is_some_and(|d| d.as_ref() == "A test tool"));
    }

    #[test]
    fn test_tool_def_no_required_params() {
        let properties = serde_json::json!({});
        let tool = IndodaxMcp::tool_def("empty_tool", "No params", properties, vec![]);
        assert_eq!(tool.name.to_string(), "empty_tool");
    }

    #[test]
    fn test_str_param() {
        let param = IndodaxMcp::str_param("A test string", true, Some("default"));
        assert_eq!(param["type"], "string");
        assert_eq!(param["default"], "default");
        assert_eq!(param["required"], true);

        let param_opt = IndodaxMcp::str_param("Optional string", false, None);
        assert_eq!(param_opt["required"], Value::Null);
    }

    #[test]
    fn test_num_param_required() {
        let param = IndodaxMcp::num_param("A test number", true);
        assert_eq!(param["type"], "number");
        assert_eq!(param["required"], true);

        let param_opt = IndodaxMcp::num_param("Optional number", false);
        assert_eq!(param_opt["required"], Value::Null);
    }

    #[test]
    fn test_bool_param() {
        let param = IndodaxMcp::bool_param("A test boolean");
        assert_eq!(param["type"], "boolean");
    }

    #[test]
    fn test_mcp_is_group_enabled() {
        let mcp = test_mcp();
        assert!(mcp.is_group_enabled(&ServiceGroup::Market));
        assert!(mcp.is_group_enabled(&ServiceGroup::Paper));
        assert!(mcp.is_group_enabled(&ServiceGroup::Alert));
        assert!(!mcp.is_group_enabled(&ServiceGroup::Trade));
        assert!(!mcp.is_group_enabled(&ServiceGroup::Account));
        assert!(!mcp.is_group_enabled(&ServiceGroup::Funding));
        assert!(!mcp.is_group_enabled(&ServiceGroup::Auth));
    }

    #[test]
    fn test_ok_result() {
        let result = IndodaxMcp::ok_result("success".into());
        assert_eq!(result.is_error, Some(false));
    }

    #[test]
    fn test_error_result_contains_error() {
        let result = IndodaxMcp::error_result("something failed".into());
        assert_eq!(result.is_error, Some(true));
    }

    #[test]
    fn test_validation_error_result_contains_validation_type() {
        let result = IndodaxMcp::validation_error_result("bad input".into());
        assert_eq!(result.is_error, Some(true));
        let content = &result.content;
        let text = content.first().and_then(|c| c.as_text()).map(|t| t.text.as_str()).unwrap_or("");
        assert!(text.contains("validation_error"));
    }

    #[test]
    fn test_json_result() {
        let value = json!({"key": "value", "num": 42});
        let result = IndodaxMcp::json_result(value);
        assert_eq!(result.is_error, Some(false));
    }

    #[test]
    fn test_all_tools_respects_groups() {
        let mcp = test_mcp();
        let tools = mcp.all_tools();
        let names: Vec<String> = tools.iter().map(|t| t.name.to_string()).collect();
        assert!(names.contains(&"server_time".to_string()));
        assert!(names.contains(&"ticker".to_string()));
        assert!(names.contains(&"paper_init".to_string()));
        assert!(names.contains(&"paper_balance".to_string()));
        assert!(names.contains(&"alert_add".to_string()));
        assert!(names.contains(&"alert_list".to_string()));
        assert!(!names.contains(&"buy_order".to_string()));
        assert!(!names.contains(&"sell_order".to_string()));
        assert!(!names.contains(&"account_info".to_string()));
    }
}
