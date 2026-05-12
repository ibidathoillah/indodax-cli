pub mod account;
pub mod auth;
pub mod funding;
pub mod market;
pub mod paper;
pub mod trade;

use std::sync::Arc;

use tokio::sync::Mutex;
use serde_json::{Map, Value};

use rmcp::model::{
    CallToolRequestParams, CallToolResult, Content, Implementation, InitializeResult,
    ListToolsResult, PaginatedRequestParams, ServerCapabilities, Tool,
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
        }
    }

    pub fn is_group_enabled(&self, group: &ServiceGroup) -> bool {
        self.enabled_groups.contains(group)
    }

    pub fn str_param(description: &str, _required: bool, default_: Option<&str>) -> Value {
        let mut schema = serde_json::json!({
            "type": "string",
            "description": description,
        });
        if let Some(d) = default_ {
            schema["default"] = Value::String(d.to_string());
        }
        schema
    }

    pub fn num_param(description: &str, _required: bool) -> Value {
        serde_json::json!({
            "type": "number",
            "description": description,
        })
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
        args.get(name)
            .and_then(|v| v.as_bool())
            .unwrap_or(false)
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
}

impl rmcp::handler::server::ServerHandler for IndodaxMcp {
    fn get_info(&self) -> InitializeResult {
        InitializeResult::new(
            ServerCapabilities::builder()
                .enable_tools()
                .build(),
        )
        .with_server_info(Implementation::new(
            "indodax-cli",
            env!("CARGO_PKG_VERSION"),
        ))
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
                    &Self::get_str(&args, "symbol").unwrap_or_default()
                ).replace('_', "").to_uppercase();
                let timeframe =
                    Self::get_str(&args, "timeframe").unwrap_or_else(|| "60".into());
                let from = Self::get_num(&args, "from");
                let to = Self::get_num(&args, "to");
                self.handle_ohlc(&symbol, &timeframe, from, to).await
            }
            "price_increments" => self.handle_price_increments().await,

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
                    Some(v) => v,
                    None => return Ok(Self::error_result("Missing required parameter: order_id".into())),
                };
                let pair = match Self::get_str(&args, "pair") {
                    Some(v) => helpers::normalize_pair(&v),
                    None => return Ok(Self::error_result("Missing required parameter: pair".into())),
                };
                self.handle_get_order(order_id, &pair).await
            }
            "trans_history" => self.handle_trans_history().await,

            // Trade (dangerous)
            "buy_order" => {
                let acknowledged = Self::get_bool(&args, "acknowledged");
                if let Err(msg) =
                    self.safety.check_operation(&ServiceGroup::Trade, acknowledged)
                {
                    return Ok(Self::error_result(msg));
                }
                let pair = match Self::get_str(&args, "pair") {
                    Some(v) => helpers::normalize_pair(&v),
                    None => return Ok(Self::error_result("Missing required parameter: pair".into())),
                };
                let idr = match Self::get_num(&args, "idr") {
                    Some(v) => v,
                    None => return Ok(Self::error_result("Missing required parameter: idr".into())),
                };
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
                let pair = match Self::get_str(&args, "pair") {
                    Some(v) => helpers::normalize_pair(&v),
                    None => return Ok(Self::error_result("Missing required parameter: pair".into())),
                };
                let price = Self::get_num(&args, "price");
                let amount = match Self::get_num(&args, "amount") {
                    Some(v) => v,
                    None => return Ok(Self::error_result("Missing required parameter: amount".into())),
                };
                let order_type =
                    Self::get_str(&args, "order_type").unwrap_or_else(|| "limit".into());
                self.handle_sell_order(&pair, price, amount, &order_type).await
            }
            "cancel_order" => {
                let acknowledged = Self::get_bool(&args, "acknowledged");
                if let Err(msg) =
                    self.safety.check_operation(&ServiceGroup::Trade, acknowledged)
                {
                    return Ok(Self::error_result(msg));
                }
                let order_id = match Self::get_num(&args, "order_id") {
                    Some(v) => v,
                    None => return Ok(Self::error_result("Missing required parameter: order_id".into())),
                };
                let pair = match Self::get_str(&args, "pair") {
                    Some(v) => helpers::normalize_pair(&v),
                    None => return Ok(Self::error_result("Missing required parameter: pair".into())),
                };
                let order_type = Self::get_str(&args, "order_type").unwrap_or_default();
                self.handle_cancel_order(order_id, &pair, &order_type).await
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
                self.handle_withdraw(
                    &currency,
                    amount,
                    &address,
                    to_username,
                    memo.as_deref(),
                    network.as_deref(),
                )
                .await
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
                let amount = Self::get_num(&args, "amount").unwrap_or(0.0);
                let side = if name == "paper_buy" {
                    "buy"
                } else {
                    "sell"
                };
                self.handle_paper_trade(side, &pair, price, amount).await
            }
            "paper_orders" => self.handle_paper_orders().await,
            "paper_cancel" => {
                let order_id = match Self::get_num(&args, "order_id") {
                    Some(v) => v as u64,
                    None => return Ok(Self::error_result("Missing required parameter: order_id".into())),
                };
                self.handle_paper_cancel(order_id).await
            }
            "paper_cancel_all" => self.handle_paper_cancel_all().await,
            "paper_history" => self.handle_paper_history().await,
            "paper_status" => self.handle_paper_status().await,

            // Auth
            "auth_show" => self.handle_auth_show().await,
            "auth_test" => self.handle_auth_test().await,

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
        let groups = vec![ServiceGroup::Market, ServiceGroup::Paper];
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
    fn test_tool_def_creates_tool() {
        let properties = serde_json::json!({
            "pair": {
                "type": "string",
                "description": "Trading pair"
            }
        });
        let tool = IndodaxMcp::tool_def("test_tool", "A test tool", properties, vec!["pair"]);
        assert_eq!(tool.name.to_string(), "test_tool");
        assert!(tool.description.map_or(false, |d| d.as_ref() == "A test tool"));
    }

    #[test]
    fn test_tool_def_no_required_params() {
        let properties = serde_json::json!({});
        let tool = IndodaxMcp::tool_def("empty_tool", "No params", properties, vec![]);
        assert_eq!(tool.name.to_string(), "empty_tool");
    }

    #[test]
    fn test_str_param() {
        let param = IndodaxMcp::str_param("A test string", false, Some("default"));
        assert_eq!(param["type"], "string");
        assert_eq!(param["default"], "default");
    }

    #[test]
    fn test_num_param() {
        let param = IndodaxMcp::num_param("A test number", true);
        assert_eq!(param["type"], "number");
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
        assert!(!names.contains(&"buy_order".to_string()));
        assert!(!names.contains(&"sell_order".to_string()));
        assert!(!names.contains(&"account_info".to_string()));
    }
}
