use std::sync::Arc;

use tokio::sync::Mutex;
use serde_json::{Map, Value};

use rmcp::model::{
    CallToolRequestParams, CallToolResult, Content, Implementation, InitializeResult,
    ListToolsResult, PaginatedRequestParams, ServerCapabilities, Tool,
};
use rmcp::service::{RequestContext, RoleServer};
use rmcp::ErrorData as McpError;

use crate::auth::Signer;
use crate::client::IndodaxClient;
use crate::config::IndodaxConfig;
use crate::mcp::safety::SafetyConfig;
use crate::mcp::service::ServiceGroup;

/// The MCP server exposing Indodax trading functionality as tools.
#[derive(Debug, Clone)]
pub struct IndodaxMcp {
    client: Arc<IndodaxClient>,
    config: Arc<Mutex<IndodaxConfig>>,
    safety: SafetyConfig,
    enabled_groups: Vec<ServiceGroup>,
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

    fn is_group_enabled(&self, group: &ServiceGroup) -> bool {
        self.enabled_groups.contains(group)
    }

    // ──────────────────────────────────────────────
    // Schema helpers
    // ──────────────────────────────────────────────

    fn str_param(description: &str, _required: bool, default_: Option<&str>) -> Value {
        let mut schema = serde_json::json!({
            "type": "string",
            "description": description,
        });
        if let Some(d) = default_ {
            schema["default"] = Value::String(d.to_string());
        }
        schema
    }

    fn num_param(description: &str, _required: bool) -> Value {
        serde_json::json!({
            "type": "number",
            "description": description,
        })
    }

    fn bool_param(description: &str) -> Value {
        serde_json::json!({
            "type": "boolean",
            "description": description,
        })
    }

    fn tool_def(name: &str, description: &str, properties: Value, required: Vec<&str>) -> Tool {
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

    // ──────────────────────────────────────────────
    // Argument extraction helpers
    // ──────────────────────────────────────────────

    fn get_str(args: &Map<String, Value>, name: &str) -> Option<String> {
        args.get(name)
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
    }

    fn get_num(args: &Map<String, Value>, name: &str) -> Option<f64> {
        args.get(name).and_then(|v| {
            v.as_f64()
                .or_else(|| v.as_str().and_then(|s| s.parse::<f64>().ok()))
        })
    }

    fn get_bool(args: &Map<String, Value>, name: &str) -> bool {
        args.get(name)
            .and_then(|v| v.as_bool())
            .unwrap_or(false)
    }

    // ──────────────────────────────────────────────
    // Result helpers
    // ──────────────────────────────────────────────

    fn ok_result(text: String) -> CallToolResult {
        CallToolResult::success(vec![Content::text(text)])
    }

    fn error_result(text: String) -> CallToolResult {
        CallToolResult::error(vec![Content::text(text)])
    }

    fn json_result(value: Value) -> CallToolResult {
        let text = serde_json::to_string_pretty(&value).unwrap_or_default();
        Self::ok_result(text)
    }

    // ──────────────────────────────────────────────
    // Tool definitions by group
    // ──────────────────────────────────────────────

    fn market_tools(&self) -> Vec<Tool> {
        vec![
            Self::tool_def(
                "server_time",
                "Get the current Indodax server time",
                serde_json::json!({}),
                vec![],
            ),
            Self::tool_def(
                "ticker",
                "Get ticker for a trading pair",
                serde_json::json!({
                    "pair": Self::str_param("Trading pair, e.g. btc_idr", false, Some("btc_idr"))
                }),
                vec![],
            ),
            Self::tool_def(
                "ticker_all",
                "Get tickers for all trading pairs",
                serde_json::json!({}),
                vec![],
            ),
            Self::tool_def(
                "pairs",
                "List all available trading pairs",
                serde_json::json!({}),
                vec![],
            ),
            Self::tool_def(
                "summaries",
                "Get 24h and 7d market summaries for all pairs",
                serde_json::json!({}),
                vec![],
            ),
            Self::tool_def(
                "orderbook",
                "Get order book depth for a trading pair",
                serde_json::json!({
                    "pair": Self::str_param("Trading pair, e.g. btcidr", false, Some("btcidr"))
                }),
                vec![],
            ),
            Self::tool_def(
                "trades",
                "Get recent trades for a trading pair",
                serde_json::json!({
                    "pair": Self::str_param("Trading pair, e.g. btcidr", false, Some("btcidr"))
                }),
                vec![],
            ),
            Self::tool_def(
                "ohlc",
                "Get OHLCV candle data",
                serde_json::json!({
                    "symbol": Self::str_param("Trading pair symbol, e.g. BTCIDR", true, None),
                    "timeframe": Self::str_param(
                        "Candle timeframe in minutes, e.g. 60",
                        false,
                        Some("60"),
                    ),
                    "from": Self::num_param("Start timestamp (seconds)", false),
                    "to": Self::num_param("End timestamp (seconds)", false),
                }),
                vec!["symbol"],
            ),
            Self::tool_def(
                "price_increments",
                "Get price increments (tick sizes) for all pairs",
                serde_json::json!({}),
                vec![],
            ),
        ]
    }

    fn account_tools(&self) -> Vec<Tool> {
        vec![
            Self::tool_def(
                "account_info",
                "[REQUIRES AUTH] Get account information including balances",
                serde_json::json!({}),
                vec![],
            ),
            Self::tool_def(
                "balance",
                "[REQUIRES AUTH] Get wallet balances (non-zero only)",
                serde_json::json!({}),
                vec![],
            ),
            Self::tool_def(
                "open_orders",
                "[REQUIRES AUTH] List open orders",
                serde_json::json!({
                    "pair": Self::str_param("Filter by trading pair (optional)", false, None)
                }),
                vec![],
            ),
            Self::tool_def(
                "order_history",
                "[REQUIRES AUTH] Get order history",
                serde_json::json!({
                    "symbol": Self::str_param(
                        "Trading pair symbol, e.g. btc_idr",
                        false,
                        Some("btc_idr"),
                    ),
                    "limit": Self::num_param("Maximum number of orders to return", false),
                }),
                vec![],
            ),
            Self::tool_def(
                "trade_history",
                "[REQUIRES AUTH] Get trade fill history",
                serde_json::json!({
                    "symbol": Self::str_param(
                        "Trading pair symbol, e.g. btc_idr",
                        false,
                        Some("btc_idr"),
                    ),
                    "limit": Self::num_param("Maximum number of trades to return", false),
                }),
                vec![],
            ),
            Self::tool_def(
                "get_order",
                "[REQUIRES AUTH] Get order details by order ID",
                serde_json::json!({
                    "order_id": Self::num_param("Order ID", true),
                    "pair": Self::str_param("Trading pair, e.g. btc_idr", true, None),
                }),
                vec!["order_id", "pair"],
            ),
            Self::tool_def(
                "trans_history",
                "[REQUIRES AUTH] Get deposit and withdrawal transaction history",
                serde_json::json!({}),
                vec![],
            ),
        ]
    }

    fn trade_tools(&self) -> Vec<Tool> {
        vec![
            Self::tool_def(
                "buy_order",
                "[DANGEROUS: requires acknowledged=true] Place a buy order on Indodax",
                serde_json::json!({
                    "pair": Self::str_param("Trading pair, e.g. btc_idr", true, None),
                    "idr": Self::num_param("Total IDR amount to spend", true),
                    "price": Self::num_param("Limit price (omit for market order)", false),
                    "acknowledged":
                        Self::bool_param("Must be true to confirm this dangerous operation"),
                }),
                vec!["pair", "idr", "acknowledged"],
            ),
            Self::tool_def(
                "sell_order",
                "[DANGEROUS: requires acknowledged=true] Place a sell order on Indodax",
                serde_json::json!({
                    "pair": Self::str_param("Trading pair, e.g. btc_idr", true, None),
                    "price": Self::num_param("Limit price", true),
                    "amount": Self::num_param("Amount in base currency (e.g. BTC)", true),
                    "order_type":
                        Self::str_param("Order type: limit or market", false, Some("limit")),
                    "acknowledged":
                        Self::bool_param("Must be true to confirm this dangerous operation"),
                }),
                vec!["pair", "price", "amount", "acknowledged"],
            ),
            Self::tool_def(
                "cancel_order",
                "[DANGEROUS: requires acknowledged=true] Cancel an existing order by ID",
                serde_json::json!({
                    "order_id": Self::num_param("Order ID to cancel", true),
                    "pair": Self::str_param("Trading pair, e.g. btc_idr", true, None),
                    "order_type": Self::str_param("Order type: buy or sell", true, None),
                    "acknowledged":
                        Self::bool_param("Must be true to confirm this dangerous operation"),
                }),
                vec!["order_id", "pair", "order_type", "acknowledged"],
            ),
        ]
    }

    fn funding_tools(&self) -> Vec<Tool> {
        vec![
            Self::tool_def(
                "withdraw_fee",
                "[REQUIRES AUTH] Check withdrawal fee for a currency",
                serde_json::json!({
                    "currency": Self::str_param("Currency to check, e.g. btc", true, None),
                    "network": Self::str_param("Blockchain network (optional)", false, None),
                }),
                vec!["currency"],
            ),
            Self::tool_def(
                "withdraw",
                "[DANGEROUS: requires acknowledged=true] Withdraw cryptocurrency from Indodax",
                serde_json::json!({
                    "currency": Self::str_param("Currency to withdraw, e.g. btc", true, None),
                    "amount": Self::num_param("Amount to withdraw", true),
                    "address":
                        Self::str_param("Destination address or Indodax username", true, None),
                    "to_username":
                        Self::bool_param("Withdraw to Indodax username instead of blockchain address"),
                    "memo": Self::str_param("Memo/tag for currencies that require it", false, None),
                    "network": Self::str_param("Blockchain network", false, None),
                    "acknowledged":
                        Self::bool_param("Must be true to confirm this dangerous operation"),
                }),
                vec!["currency", "amount", "address", "acknowledged"],
            ),
        ]
    }

    fn paper_tools(&self) -> Vec<Tool> {
        vec![
            Self::tool_def(
                "paper_init",
                "Initialize paper trading with default virtual balances (100M IDR, 1 BTC)",
                serde_json::json!({}),
                vec![],
            ),
            Self::tool_def(
                "paper_reset",
                "Reset paper trading state to defaults",
                serde_json::json!({}),
                vec![],
            ),
            Self::tool_def(
                "paper_balance",
                "Show current paper trading virtual balances",
                serde_json::json!({}),
                vec![],
            ),
            Self::tool_def(
                "paper_buy",
                "Place a simulated paper buy order",
                serde_json::json!({
                    "pair":
                        Self::str_param("Trading pair, e.g. btc_idr", false, Some("btc_idr")),
                    "price": Self::num_param("Price for the order", true),
                    "amount": Self::num_param("Amount in base currency", true),
                }),
                vec!["price", "amount"],
            ),
            Self::tool_def(
                "paper_sell",
                "Place a simulated paper sell order",
                serde_json::json!({
                    "pair":
                        Self::str_param("Trading pair, e.g. btc_idr", false, Some("btc_idr")),
                    "price": Self::num_param("Price for the order", true),
                    "amount": Self::num_param("Amount in base currency", true),
                }),
                vec!["price", "amount"],
            ),
            Self::tool_def(
                "paper_orders",
                "List paper trading orders",
                serde_json::json!({}),
                vec![],
            ),
            Self::tool_def(
                "paper_cancel",
                "Cancel a paper trading order",
                serde_json::json!({
                    "order_id": Self::num_param("Order ID to cancel", true),
                }),
                vec!["order_id"],
            ),
            Self::tool_def(
                "paper_cancel_all",
                "Cancel all paper trading orders",
                serde_json::json!({}),
                vec![],
            ),
            Self::tool_def(
                "paper_history",
                "Show paper trading order history",
                serde_json::json!({}),
                vec![],
            ),
            Self::tool_def(
                "paper_status",
                "Show paper trading status summary (trades, balances, P&L)",
                serde_json::json!({}),
                vec![],
            ),
        ]
    }

    fn auth_tools(&self) -> Vec<Tool> {
        vec![
            Self::tool_def(
                "auth_show",
                "Show current API configuration status",
                serde_json::json!({}),
                vec![],
            ),
            Self::tool_def(
                "auth_test",
                "Test if current API credentials are valid",
                serde_json::json!({}),
                vec![],
            ),
        ]
    }

    fn all_tools(&self) -> Vec<Tool> {
        let mut tools: Vec<Tool> = Vec::new();

        if self.is_group_enabled(&ServiceGroup::Market) {
            tools.extend(self.market_tools());
        }
        if self.is_group_enabled(&ServiceGroup::Account) {
            tools.extend(self.account_tools());
        }
        if self.is_group_enabled(&ServiceGroup::Trade) {
            let _ = self.safety.check_group(&ServiceGroup::Trade);
            tools.extend(self.trade_tools());
        }
        if self.is_group_enabled(&ServiceGroup::Funding) {
            let _ = self.safety.check_group(&ServiceGroup::Funding);
            tools.extend(self.funding_tools());
        }
        if self.is_group_enabled(&ServiceGroup::Paper) {
            tools.extend(self.paper_tools());
        }
        if self.is_group_enabled(&ServiceGroup::Auth) {
            tools.extend(self.auth_tools());
        }

        tools
    }

    // ──────────────────────────────────────────────
    // Market handlers (no auth required)
    // ──────────────────────────────────────────────

    async fn handle_server_time(&self) -> CallToolResult {
        match self
            .client
            .public_get::<Value>("/api/server_time")
            .await
        {
            Ok(data) => Self::json_result(data),
            Err(e) => Self::error_result(e.to_string()),
        }
    }

    async fn handle_ticker(&self, pair: &str) -> CallToolResult {
        let path = format!("/api/ticker/{}", pair);
        match self.client.public_get::<Value>(&path).await {
            Ok(data) => Self::json_result(data),
            Err(e) => Self::error_result(e.to_string()),
        }
    }

    async fn handle_ticker_all(&self) -> CallToolResult {
        match self
            .client
            .public_get::<Value>("/api/ticker_all")
            .await
        {
            Ok(data) => Self::json_result(data),
            Err(e) => Self::error_result(e.to_string()),
        }
    }

    async fn handle_pairs(&self) -> CallToolResult {
        match self.client.public_get::<Value>("/api/pairs").await {
            Ok(data) => Self::json_result(data),
            Err(e) => Self::error_result(e.to_string()),
        }
    }

    async fn handle_summaries(&self) -> CallToolResult {
        match self
            .client
            .public_get::<Value>("/api/summaries")
            .await
        {
            Ok(data) => Self::json_result(data),
            Err(e) => Self::error_result(e.to_string()),
        }
    }

    async fn handle_orderbook(&self, pair: &str) -> CallToolResult {
        let path = format!("/api/depth/{}", pair);
        match self.client.public_get::<Value>(&path).await {
            Ok(data) => Self::json_result(data),
            Err(e) => Self::error_result(e.to_string()),
        }
    }

    async fn handle_trades(&self, pair: &str) -> CallToolResult {
        let path = format!("/api/trades/{}", pair);
        match self.client.public_get::<Value>(&path).await {
            Ok(data) => Self::json_result(data),
            Err(e) => Self::error_result(e.to_string()),
        }
    }

    async fn handle_ohlc(
        &self,
        symbol: &str,
        timeframe: &str,
        from: Option<f64>,
        to: Option<f64>,
    ) -> CallToolResult {
        let now_secs = Signer::now_millis() / 1000;
        let from_val = from
            .map(|v| v.to_string())
            .unwrap_or_else(|| (now_secs - 24 * 60 * 60).to_string());
        let to_val = to
            .map(|v| v.to_string())
            .unwrap_or_else(|| now_secs.to_string());

        match self
            .client
            .public_get_v2::<Value>(
                "/tradingview/history_v2",
                &[
                    ("symbol", symbol),
                    ("tf", timeframe),
                    ("from", &from_val),
                    ("to", &to_val),
                ],
            )
            .await
        {
            Ok(data) => Self::json_result(data),
            Err(e) => Self::error_result(e.to_string()),
        }
    }

    async fn handle_price_increments(&self) -> CallToolResult {
        match self
            .client
            .public_get::<Value>("/api/price_increments")
            .await
        {
            Ok(data) => Self::json_result(data),
            Err(e) => Self::error_result(e.to_string()),
        }
    }

    // ──────────────────────────────────────────────
    // Account handlers (auth required)
    // ──────────────────────────────────────────────

    async fn handle_account_info(&self) -> CallToolResult {
        match self
            .client
            .private_post_v1::<Value>("getInfo", &std::collections::HashMap::new())
            .await
        {
            Ok(data) => Self::json_result(data),
            Err(e) => Self::error_result(e.to_string()),
        }
    }

    async fn handle_balance(&self) -> CallToolResult {
        match self
            .client
            .private_post_v1::<Value>("getInfo", &std::collections::HashMap::new())
            .await
        {
            Ok(data) => {
                let balance = data
                    .get("balance")
                    .cloned()
                    .unwrap_or(Value::Object(Map::new()));
                Self::json_result(balance)
            }
            Err(e) => Self::error_result(e.to_string()),
        }
    }

    async fn handle_open_orders(&self, pair: Option<&str>) -> CallToolResult {
        let mut params = std::collections::HashMap::new();
        if let Some(p) = pair {
            params.insert("pair".to_string(), p.to_string());
        }
        match self
            .client
            .private_post_v1::<Value>("openOrders", &params)
            .await
        {
            Ok(data) => Self::json_result(data),
            Err(e) => Self::error_result(e.to_string()),
        }
    }

    async fn handle_order_history(&self, symbol: &str, limit: Option<f64>) -> CallToolResult {
        let now = Signer::now_millis();
        let start = now - 24 * 60 * 60 * 1000;
        let limit_val = limit.unwrap_or(100.0) as u32;

        let mut params = std::collections::HashMap::new();
        params.insert("symbol".to_string(), symbol.to_string());
        params.insert("limit".to_string(), limit_val.to_string());
        params.insert("startTime".to_string(), start.to_string());
        params.insert("endTime".to_string(), now.to_string());

        match self
            .client
            .private_get_v2::<Value>("/api/v2/order/histories", &params)
            .await
        {
            Ok(data) => Self::json_result(data),
            Err(e) => Self::error_result(e.to_string()),
        }
    }

    async fn handle_trade_history(&self, symbol: &str, limit: Option<f64>) -> CallToolResult {
        let now = Signer::now_millis();
        let start = now - 24 * 60 * 60 * 1000;
        let limit_val = limit.unwrap_or(100.0) as u32;

        let mut params = std::collections::HashMap::new();
        params.insert("symbol".to_string(), symbol.to_string());
        params.insert("limit".to_string(), limit_val.to_string());
        params.insert("startTime".to_string(), start.to_string());
        params.insert("endTime".to_string(), now.to_string());

        match self
            .client
            .private_get_v2::<Value>("/api/v2/myTrades", &params)
            .await
        {
            Ok(data) => Self::json_result(data),
            Err(e) => Self::error_result(e.to_string()),
        }
    }

    async fn handle_get_order(&self, order_id: f64, pair: &str) -> CallToolResult {
        let mut params = std::collections::HashMap::new();
        params.insert("order_id".to_string(), (order_id as u64).to_string());
        params.insert("pair".to_string(), pair.to_string());

        match self
            .client
            .private_post_v1::<Value>("getOrder", &params)
            .await
        {
            Ok(data) => Self::json_result(data),
            Err(e) => Self::error_result(e.to_string()),
        }
    }

    async fn handle_trans_history(&self) -> CallToolResult {
        match self
            .client
            .private_post_v1::<Value>("transHistory", &std::collections::HashMap::new())
            .await
        {
            Ok(data) => Self::json_result(data),
            Err(e) => Self::error_result(e.to_string()),
        }
    }

    // ──────────────────────────────────────────────
    // Trade handlers (dangerous)
    // ──────────────────────────────────────────────

    async fn handle_buy_order(&self, pair: &str, idr: f64, price: Option<f64>) -> CallToolResult {
        let mut params = std::collections::HashMap::new();
        params.insert("pair".to_string(), pair.to_string());
        params.insert("type".to_string(), "buy".to_string());
        params.insert("idr".to_string(), idr.to_string());

        if let Some(p) = price {
            params.insert("price".to_string(), p.to_string());
        } else {
            params.insert("order_type".to_string(), "market".to_string());
        }

        match self
            .client
            .private_post_v1::<Value>("trade", &params)
            .await
        {
            Ok(data) => Self::json_result(data),
            Err(e) => Self::error_result(e.to_string()),
        }
    }

    async fn handle_sell_order(
        &self,
        pair: &str,
        price: f64,
        amount: f64,
        order_type: &str,
    ) -> CallToolResult {
        let base_currency = pair.split('_').next().unwrap_or_default();
        if base_currency.is_empty() {
            return Self::error_result(format!("Invalid pair format: {}", pair));
        }

        let mut params = std::collections::HashMap::new();
        params.insert("pair".to_string(), pair.to_string());
        params.insert("type".to_string(), "sell".to_string());
        params.insert("price".to_string(), price.to_string());
        params.insert(base_currency.to_string(), amount.to_string());

        if order_type == "market" {
            params.insert("order_type".to_string(), "market".to_string());
        }

        match self
            .client
            .private_post_v1::<Value>("trade", &params)
            .await
        {
            Ok(data) => Self::json_result(data),
            Err(e) => Self::error_result(e.to_string()),
        }
    }

    async fn handle_cancel_order(
        &self,
        order_id: f64,
        pair: &str,
        order_type: &str,
    ) -> CallToolResult {
        let mut params = std::collections::HashMap::new();
        params.insert("order_id".to_string(), (order_id as u64).to_string());
        params.insert("pair".to_string(), pair.to_string());
        params.insert("type".to_string(), order_type.to_string());

        match self
            .client
            .private_post_v1::<Value>("cancelOrder", &params)
            .await
        {
            Ok(data) => Self::json_result(data),
            Err(e) => Self::error_result(e.to_string()),
        }
    }

    // ──────────────────────────────────────────────
    // Funding handlers
    // ──────────────────────────────────────────────

    async fn handle_withdraw_fee(
        &self,
        currency: &str,
        network: Option<&str>,
    ) -> CallToolResult {
        let mut params = std::collections::HashMap::new();
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
            Err(e) => Self::error_result(e.to_string()),
        }
    }

    #[allow(clippy::too_many_arguments)]
    async fn handle_withdraw(
        &self,
        currency: &str,
        amount: f64,
        address: &str,
        to_username: bool,
        memo: Option<&str>,
        network: Option<&str>,
    ) -> CallToolResult {
        let mut params = std::collections::HashMap::new();
        params.insert("currency".to_string(), currency.to_string());
        params.insert("amount".to_string(), amount.to_string());

        if to_username {
            params.insert("request_id".to_string(), "1".to_string());
            params.insert("withdraw_to".to_string(), address.to_string());
        } else {
            params.insert("address".to_string(), address.to_string());
        }

        if let Some(m) = memo {
            params.insert("memo".to_string(), m.to_string());
        }
        if let Some(n) = network {
            params.insert("network".to_string(), n.to_string());
        }

        match self
            .client
            .private_post_v1::<Value>("withdrawCoin", &params)
            .await
        {
            Ok(data) => Self::json_result(data),
            Err(e) => Self::error_result(e.to_string()),
        }
    }

    // ──────────────────────────────────────────────
    // Paper trading handlers
    // ──────────────────────────────────────────────

    async fn handle_paper_init(&self) -> CallToolResult {
        let config = self.config.lock().await;
        match crate::commands::paper::paper_init_cmd(&config).await {
            Some(_output) => Self::ok_result(
                "[PAPER] Paper trading initialized with 100,000,000 IDR and 1 BTC".to_string(),
            ),
            None => Self::error_result("Failed to initialize paper trading".to_string()),
        }
    }

    async fn handle_paper_balance(&self) -> CallToolResult {
        let config = self.config.lock().await;
        let state = crate::commands::paper::PaperState::load(&config);
        let balances = state.balances;
        Self::json_result(serde_json::json!({
            "mode": "paper",
            "balances": balances,
        }))
    }
}

// ──────────────────────────────────────────────
// ServerHandler implementation
// ──────────────────────────────────────────────

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
                let pair =
                    Self::get_str(&args, "pair").unwrap_or_else(|| "btc_idr".into());
                self.handle_ticker(&pair).await
            }
            "ticker_all" => self.handle_ticker_all().await,
            "pairs" => self.handle_pairs().await,
            "summaries" => self.handle_summaries().await,
            "orderbook" => {
                let pair =
                    Self::get_str(&args, "pair").unwrap_or_else(|| "btcidr".into());
                self.handle_orderbook(&pair).await
            }
            "trades" => {
                let pair =
                    Self::get_str(&args, "pair").unwrap_or_else(|| "btcidr".into());
                self.handle_trades(&pair).await
            }
            "ohlc" => {
                let symbol = Self::get_str(&args, "symbol").unwrap_or_default();
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
                let pair = Self::get_str(&args, "pair");
                self.handle_open_orders(pair.as_deref()).await
            }
            "order_history" => {
                let symbol =
                    Self::get_str(&args, "symbol").unwrap_or_else(|| "btc_idr".into());
                let limit = Self::get_num(&args, "limit");
                self.handle_order_history(&symbol, limit).await
            }
            "trade_history" => {
                let symbol =
                    Self::get_str(&args, "symbol").unwrap_or_else(|| "btc_idr".into());
                let limit = Self::get_num(&args, "limit");
                self.handle_trade_history(&symbol, limit).await
            }
            "get_order" => {
                let order_id = Self::get_num(&args, "order_id").unwrap_or(0.0);
                let pair = Self::get_str(&args, "pair").unwrap_or_default();
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
                let pair = Self::get_str(&args, "pair").unwrap_or_default();
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
                let pair = Self::get_str(&args, "pair").unwrap_or_default();
                let price = Self::get_num(&args, "price").unwrap_or(0.0);
                let amount = Self::get_num(&args, "amount").unwrap_or(0.0);
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
                let order_id = Self::get_num(&args, "order_id").unwrap_or(0.0);
                let pair = Self::get_str(&args, "pair").unwrap_or_default();
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
            "paper_init" => self.handle_paper_init().await,
            "paper_reset" => {
                Self::ok_result("[PAPER] Trading state reset".to_string())
            }
            "paper_balance" => self.handle_paper_balance().await,
            "paper_buy" | "paper_sell" => {
                let pair = Self::get_str(&args, "pair")
                    .unwrap_or_else(|| "btc_idr".into());
                let price = Self::get_num(&args, "price").unwrap_or(0.0);
                let amount = Self::get_num(&args, "amount").unwrap_or(0.0);
                let side = if name == "paper_buy" {
                    "buy"
                } else {
                    "sell"
                };

                let mut config = self.config.lock().await;
                let mut state = crate::commands::paper::PaperState::load(&config);
                match crate::commands::paper::place_paper_order(
                    &mut state, &pair, side, price, amount,
                ) {
                    Ok(_output) => {
                        let _ = state.save(&mut config);
                        Self::json_result(serde_json::json!({
                            "mode": "paper",
                            "side": side,
                            "pair": pair,
                            "price": price,
                            "amount": amount,
                            "status": "filled",
                        }))
                    }
                    Err(e) => Self::error_result(e.to_string()),
                }
            }
            "paper_orders" => {
                let config = self.config.lock().await;
                let state = crate::commands::paper::PaperState::load(&config);
                let open_orders: Vec<&crate::commands::paper::PaperOrder> = state
                    .orders
                    .iter()
                    .filter(|o| o.status != "cancelled")
                    .collect();
                let count = open_orders.len();
                let orders: Vec<Value> = open_orders
                    .iter()
                    .map(|o| serde_json::json!({
                        "id": o.id,
                        "pair": o.pair,
                        "side": o.side,
                        "price": o.price,
                        "amount": o.amount,
                        "remaining": o.remaining,
                        "status": o.status,
                    }))
                    .collect();
                Self::json_result(serde_json::json!({
                    "mode": "paper",
                    "count": count,
                    "orders": orders,
                }))
            }
            "paper_cancel" => {
                let order_id = Self::get_num(&args, "order_id").unwrap_or(0.0) as u64;
                let mut config = self.config.lock().await;
                let mut state = crate::commands::paper::PaperState::load(&config);
                match crate::commands::paper::cancel_paper_order(&mut state, order_id) {
                    Ok(()) => {
                        let _ = state.save(&mut config);
                        Self::ok_result(format!("[PAPER] Order {} cancelled", order_id))
                    }
                    Err(e) => Self::error_result(e.to_string()),
                }
            }
            "paper_cancel_all" => {
                let mut config = self.config.lock().await;
                let mut state = crate::commands::paper::PaperState::load(&config);
                let count = crate::commands::paper::cancel_all_paper_orders(&mut state);
                let _ = state.save(&mut config);
                Self::ok_result(format!("[PAPER] Cancelled {} orders", count))
            }
            "paper_history" => {
                let config = self.config.lock().await;
                let state = crate::commands::paper::PaperState::load(&config);
                Self::json_result(serde_json::json!({
                    "mode": "paper",
                    "orders": state.orders,
                    "count": state.orders.len(),
                }))
            }
            "paper_status" => {
                let config = self.config.lock().await;
                let state = crate::commands::paper::PaperState::load(&config);
                let filled = state
                    .orders
                    .iter()
                    .filter(|o| o.status == "filled")
                    .count();
                let open = state
                    .orders
                    .iter()
                    .filter(|o| o.status != "filled" && o.status != "cancelled")
                    .count();
                let cancelled = state
                    .orders
                    .iter()
                    .filter(|o| o.status == "cancelled")
                    .count();
                Self::json_result(serde_json::json!({
                    "mode": "paper",
                    "trade_count": state.trade_count,
                    "filled_count": filled,
                    "open_count": open,
                    "cancelled_count": cancelled,
                    "balances": state.balances,
                }))
            }

            // Auth
            "auth_show" => {
                let config = self.config.lock().await;
                Self::json_result(serde_json::json!({
                    "api_key_set": config.api_key.is_some(),
                    "api_secret_set": config.api_secret.is_some(),
                    "callback_url": config.callback_url,
                }))
            }
            "auth_test" => match self.client.signer() {
                Some(_) => {
                    match self
                        .client
                        .private_post_v1::<Value>(
                            "getInfo",
                            &std::collections::HashMap::new(),
                        )
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
                        Err(e) => Self::error_result(e.to_string()),
                    }
                }
                None => Self::error_result(
                    "No API credentials configured. Use environment variables or config file."
                        .to_string(),
                ),
            },

            _ => Self::error_result(format!("Unknown tool: {}", name)),
        };

        Ok(result)
    }
}
