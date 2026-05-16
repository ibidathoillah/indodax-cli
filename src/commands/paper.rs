use crate::client::IndodaxClient;
use crate::commands::helpers;
use crate::config::IndodaxConfig;
use crate::errors::IndodaxError;
use crate::output::CommandOutput;
use futures_util::future::join_all;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub const DEFAULT_BALANCE_IDR: f64 = 100_000_000.0;
pub const DEFAULT_BALANCE_BTC: f64 = 1.0;
const TAKER_FEE: f64 = 0.0026; // 0.26% taker fee
const BALANCE_EPSILON: f64 = 1e-8;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaperOrder {
    pub id: u64,
    pub pair: String,
    pub side: String,
    pub price: f64,
    pub amount: f64,
    pub remaining: f64,
    pub order_type: String,
    pub status: String,
    pub created_at: u64,
    #[serde(default)]
    pub fees_paid: f64,
    #[serde(default)]
    pub filled_price: f64,
    #[serde(default)]
    pub total_spent: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaperState {
    pub balances: HashMap<String, f64>,
    pub orders: Vec<PaperOrder>,
    pub next_order_id: u64,
    pub trade_count: u64,
    #[serde(default)]
    pub total_fees_paid: f64,
    #[serde(default)]
    pub initial_balances: Option<HashMap<String, f64>>,
}

impl Default for PaperState {
    fn default() -> Self {
        let mut balances = HashMap::new();
        balances.insert("idr".into(), DEFAULT_BALANCE_IDR);
        balances.insert("btc".into(), DEFAULT_BALANCE_BTC);
        Self {
            initial_balances: Some(balances.clone()),
            balances,
            orders: Vec::new(),
            next_order_id: 1,
            trade_count: 0,
            total_fees_paid: 0.0,
        }
    }
}

impl PaperState {
    pub fn initial_balance(&self, currency: &str) -> f64 {
        self.initial_balances
            .as_ref()
            .and_then(|b| b.get(currency).copied())
            .unwrap_or(0.0)
    }
}

impl PaperState {
    pub fn load(config: &IndodaxConfig) -> Self {
        let mut result: Option<PaperState> = config
            .paper_balances
            .as_ref()
            .and_then(|v| serde_json::from_value(v.clone()).ok());
        if config.paper_balances.is_some() && result.is_none() {
            eprintln!("[PAPER] Warning: Failed to deserialize saved paper state, resetting to defaults");
        }
        if let Some(ref mut state) = result {
            if state.initial_balances.is_none() {
                eprintln!("[PAPER] Warning: Saved state predates balance tracking. Snapshotting current balances as initial (P&L will reflect only future changes).");
                state.initial_balances = Some(state.balances.clone());
            }
        }
        result.unwrap_or_default()
    }

    pub fn save(&self, config: &mut IndodaxConfig) -> Result<(), IndodaxError> {
        config.paper_balances = Some(serde_json::to_value(self).map_err(|e| IndodaxError::Other(e.to_string()))?);
        config.save().map_err(|e| IndodaxError::Other(e.to_string()))?;
        Ok(())
    }
}

#[derive(Debug, clap::Subcommand)]
pub enum PaperCommand {
    #[command(name = "init", about = "Initialize paper trading with custom or default balances")]
    Init {
        #[arg(long, help = "Initial IDR balance (default: 100000000)")]
        idr: Option<f64>,
        #[arg(long, help = "Initial BTC balance (default: 1.0)")]
        btc: Option<f64>,
    },

    #[command(name = "reset", about = "Reset paper trading state")]
    Reset,

    #[command(name = "topup", about = "Add balance to a currency")]
    Topup {
        #[arg(short = 'c', long, help = "Currency to topup (e.g. idr, btc)")]
        currency: String,
        #[arg(short = 'a', long, help = "Amount to add")]
        amount: f64,
    },

    #[command(name = "balance", about = "Show paper trading balances")]
    Balance,

    #[command(name = "buy", about = "Place a simulated buy order")]
    Buy {
        #[arg(short = 'p', long, help = "Trading pair (e.g. btc_idr)")]
        pair: String,
        #[arg(short = 'i', long, help = "The total IDR amount to spend.")]
        idr: Option<f64>,
        #[arg(short = 'a', long, help = "Amount in base currency (e.g. BTC) (alternative to --idr)")]
        amount: Option<f64>,
        #[arg(short = 'r', long, help = "Limit price. If omitted, a market order will be placed.")]
        price: Option<f64>,
    },

    #[command(name = "sell", about = "Place a simulated sell order")]
    Sell {
        #[arg(short = 'p', long, help = "Trading pair (e.g. btc_idr)")]
        pair: String,
        #[arg(short = 'a', long, help = "Amount in base currency (e.g. BTC)")]
        amount: f64,
        #[arg(short = 'r', long, help = "Limit price. If omitted, a market order will be placed.")]
        price: Option<f64>,
    },

    #[command(name = "orders", about = "List open paper orders (use history for all orders)")]
    Orders {
        #[arg(short = 'p', long, help = "Filter by trading pair (e.g. btc_idr)")]
        pair: Option<String>,
        #[arg(long, help = "Sort by field: id, pair, side, price, amount, remaining, status")]
        sort_by: Option<String>,
        #[arg(long, default_value = "asc", help = "Sort order: asc or desc")]
        sort_order: Option<String>,
    },

    #[command(name = "cancel", about = "Cancel a paper order")]
    Cancel {
        #[arg(short = 'i', long, help = "Order ID to cancel")]
        order_id: u64,
    },

    #[command(name = "cancel-all", about = "Cancel all paper orders")]
    CancelAll,

    #[command(name = "fill", about = "Fill an open paper order")]
    Fill {
        #[arg(short = 'i', long, help = "Order ID to fill (required unless --all is set)")]
        order_id: Option<u64>,
        #[arg(short = 'r', long, help = "Fill price (defaults to order price)")]
        price: Option<f64>,
        #[arg(short = 'a', long, help = "Fill all open orders at once")]
        all: bool,
    },

    #[command(name = "history", about = "Show paper trading history")]
    History {
        #[arg(long, help = "Sort by field: id, pair, side, price, amount, status")]
        sort_by: Option<String>,
        #[arg(long, default_value = "desc", help = "Sort order: asc or desc (default: newest first)")]
        sort_order: Option<String>,
    },

    #[command(name = "check-fills", about = "Auto-fill open orders when market conditions match")]
    CheckFills {
        #[arg(short = 'p', long, help = "JSON object of current market prices, e.g. '{\"btc_idr\": 100000000}'")]
        prices: Option<String>,
        #[arg(long, help = "Auto-fetch current market prices from Indodax API for relevant pairs")]
        fetch: bool,
    },

    #[command(name = "status", about = "Show paper trading status summary")]
    Status,
}

pub async fn execute(
    client: &IndodaxClient,
    config: &mut IndodaxConfig,
    cmd: &PaperCommand,
) -> Result<CommandOutput, IndodaxError> {
    let mut state = PaperState::load(config);
    let result = dispatch_paper(client, &mut state, cmd).await;
    state.save(config)?;
    result
}

async fn dispatch_paper(
    client: &IndodaxClient,
    state: &mut PaperState,
    cmd: &PaperCommand,
) -> Result<CommandOutput, IndodaxError> {
    match cmd {
        PaperCommand::Init { idr, btc } => paper_init(state, *idr, *btc),
        PaperCommand::Reset => paper_reset(state),
        PaperCommand::Topup { currency, amount } => paper_topup(state, currency, *amount),
        PaperCommand::Balance => paper_balance(state),
        PaperCommand::Buy { pair, idr, amount, price } => {
            let pair = helpers::normalize_pair(pair);
            if let Some(idr_val) = idr {
                place_paper_order_idr(state, &pair, "buy", *idr_val, *price)
            } else if let Some(amt) = amount {
                place_paper_order(state, &pair, "buy", *price, *amt)
            } else {
                Err(IndodaxError::Other("Either --idr or --amount must be specified".to_string()))
            }
        }
        PaperCommand::Sell { pair, price, amount } => {
            let pair = helpers::normalize_pair(pair);
            place_paper_order(state, &pair, "sell", *price, *amount)
        }
        PaperCommand::Orders { pair, sort_by, sort_order } => {
            let pair = pair.as_ref().map(|p| helpers::normalize_pair(p));
            paper_orders(state, pair.as_deref(), sort_by.as_deref(), sort_order.as_deref())
        }
        PaperCommand::Cancel { order_id } => paper_cancel(state, *order_id),
        PaperCommand::CancelAll => paper_cancel_all(state),
        PaperCommand::Fill { order_id, price, all } => paper_fill(state, *order_id, *price, *all),
        PaperCommand::CheckFills { prices, fetch } => paper_check_fills(client, state, prices.as_deref(), *fetch).await,
        PaperCommand::History { sort_by, sort_order } => {
            paper_history(state, sort_by.as_deref(), sort_order.as_deref())
        }
        PaperCommand::Status => paper_status(state),
    }
}

pub fn init_paper_state(idr: Option<f64>, btc: Option<f64>) -> PaperState {
    let mut balances = HashMap::new();
    balances.insert("idr".into(), idr.unwrap_or(DEFAULT_BALANCE_IDR));
    balances.insert("btc".into(), btc.unwrap_or(DEFAULT_BALANCE_BTC));
    let initial = balances.clone();
    PaperState {
        balances,
        orders: Vec::new(),
        next_order_id: 1,
        trade_count: 0,
        total_fees_paid: 0.0,
        initial_balances: Some(initial),
    }
}

fn paper_init(state: &mut PaperState, idr: Option<f64>, btc: Option<f64>) -> Result<CommandOutput, IndodaxError> {
    *state = init_paper_state(idr, btc);
    let data = serde_json::json!({
        "mode": "paper",
        "status": "initialized",
        "balances": state.balances,
    });
    Ok(CommandOutput::json(data).with_addendum("[PAPER] Trading initialized with virtual balances"))
}

fn paper_reset(state: &mut PaperState) -> Result<CommandOutput, IndodaxError> {
    *state = PaperState::default();
    let data = serde_json::json!({
        "mode": "paper",
        "status": "reset"
    });
    Ok(CommandOutput::json(data).with_addendum("[PAPER] Trading state reset"))
}

fn paper_topup(state: &mut PaperState, currency: &str, amount: f64) -> Result<CommandOutput, IndodaxError> {
    if amount <= 0.0 {
        return Err(IndodaxError::Other(
            format!("[PAPER] Amount must be positive, got {}", amount)
        ));
    }
    let balance_val = {
        let balance = state.balances.entry(currency.to_lowercase()).or_insert(0.0);
        *balance += amount;
        *balance
    };
    round_balance(&mut state.balances, &currency.to_lowercase());
    let current_balance = *state.balances.get(&currency.to_lowercase()).unwrap_or(&balance_val);
    let data = serde_json::json!({
        "mode": "paper",
        "currency": currency.to_uppercase(),
        "amount_added": amount,
        "new_balance": current_balance,
    });
    Ok(CommandOutput::json(data).with_addendum(format!(
        "[PAPER] Added {} to {} balance. New balance: {}",
        format_balance(currency, amount),
        currency.to_uppercase(),
        format_balance(currency, current_balance)
    )))
}

fn is_fiat_or_stable(currency: &str) -> bool {
    match currency.to_lowercase().as_str() {
        "idr" | "usdt" | "usdc" | "dai" | "busd" | "pax" | "usde" | "gusd" | "tusd" => true,
        _ => false,
    }
}

pub fn format_balance(currency: &str, value: f64) -> String {
    if is_fiat_or_stable(currency) {
        format!("{:.2}", value)
    } else {
        format!("{:.8}", value)
    }
}

fn paper_balance(state: &PaperState) -> Result<CommandOutput, IndodaxError> {
    let headers = vec!["Currency".into(), "Balance".into()];
    let mut rows_with_balance: Vec<(f64, Vec<String>)> = state
        .balances
        .iter()
        .map(|(k, v)| (*v, vec![k.to_uppercase(), format_balance(k, *v)]))
        .collect();
    rows_with_balance.sort_by(|a, b| {
        b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal)
    });
    let rows: Vec<Vec<String>> = rows_with_balance.into_iter().map(|(_, r)| r).collect();

    let data = paper_balance_value(state);
    let balance_count = state.balances.len();
    Ok(CommandOutput::new(data, headers, rows)
        .with_addendum(format!("[PAPER] {} balance(s) tracked", balance_count)))
}

pub fn place_paper_order(
    state: &mut PaperState,
    pair: &str,
    side: &str,
    price: Option<f64>,
    amount: f64,
) -> Result<CommandOutput, IndodaxError> {
    if amount <= 0.0 {
        return Err(IndodaxError::Other(
            format!("[PAPER] Amount must be positive, got {}", amount)
        ));
    }
    let is_market = price.is_none();
    let order_price = price.unwrap_or(0.0);
    if !is_market && order_price <= 0.0 {
        return Err(IndodaxError::Other(
            format!("[PAPER] Price must be positive, got {}", order_price)
        ));
    }
    let base = pair.split('_').next().unwrap_or(pair);
    let quote = pair.split('_').next_back().unwrap_or("idr");
    let total_cost = order_price * amount;

    if side == "buy" {
        if is_market {
            let quote_balance = state.balances.entry(quote.to_string()).or_insert(0.0);
            if *quote_balance <= 0.0 {
                return Err(IndodaxError::Other(
                    format!("[PAPER] Insufficient {} balance for market buy. Need positive balance, have {}",
                        quote.to_uppercase(), format_balance(quote, *quote_balance))
                ));
            }
        } else {
            let quote_balance = state.balances.entry(quote.to_string()).or_insert(0.0);
            if *quote_balance + BALANCE_EPSILON < total_cost {
                return Err(IndodaxError::Other(
                    format!("[PAPER] Insufficient {} balance. Need {}, have {}",
                        quote.to_uppercase(), format_balance(quote, total_cost), format_balance(quote, *quote_balance))
                ));
            }
            *quote_balance -= total_cost;
            round_balance(&mut state.balances, quote);
        }
    } else {
        let base_balance = state.balances.entry(base.to_string()).or_insert(0.0);
        if *base_balance + BALANCE_EPSILON < amount {
            return Err(IndodaxError::Other(
                format!("[PAPER] Insufficient {} balance. Need {}, have {}",
                    base.to_uppercase(), format_balance(base, amount), format_balance(base, *base_balance))
            ));
        }
        *base_balance -= amount;
    }

    let order_id = state.next_order_id;
    state.next_order_id += 1;

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64;

    state.orders.push(PaperOrder {
        id: order_id,
        pair: pair.to_string(),
        side: side.to_string(),
        price: order_price,
        amount,
        remaining: amount,
        order_type: if is_market { "market".into() } else { "limit".into() },
        status: "open".into(),
        created_at: now,
        fees_paid: 0.0,
        filled_price: 0.0,
        total_spent: if side == "buy" && !is_market { total_cost } else { 0.0 },
    });

    state.trade_count += 1;

    let price_display = if is_market { "market".to_string() } else { order_price.to_string() };
    let data = serde_json::json!({
        "mode": "paper",
        "order_id": order_id,
        "pair": pair,
        "side": side,
        "price": order_price,
        "amount": amount,
        "order_type": if is_market { "market" } else { "limit" },
        "status": "open",
    });

    let headers = vec!["Field".into(), "Value".into()];
    let rows = vec![
        vec!["Order ID".into(), order_id.to_string()],
        vec!["Pair".into(), pair.to_string()],
        vec!["Side".into(), side.to_string()],
        vec!["Price".into(), price_display.clone()],
        vec!["Amount".into(), amount.to_string()],
        vec!["Type".into(), if is_market { "market".into() } else { "limit".into() }],
        vec!["Status".into(), "open".into()],
    ];

    Ok(CommandOutput::new(data, headers, rows)
        .with_addendum(format!("[PAPER] {} {} {} @ {} — open", side, amount, pair, price_display)))
}

pub fn place_paper_order_idr(
    state: &mut PaperState,
    pair: &str,
    side: &str,
    idr_amount: f64,
    price: Option<f64>,
) -> Result<CommandOutput, IndodaxError> {
    if idr_amount <= 0.0 {
        return Err(IndodaxError::Other(
            format!("[PAPER] IDR amount must be positive, got {}", idr_amount)
        ));
    }
    if side != "buy" {
        return Err(IndodaxError::Other(
            "[PAPER] --idr is only valid for buy orders".to_string()
        ));
    }
    if price.is_none() {
        return Err(IndodaxError::Other(
            "[PAPER] Market buy via --idr requires a limit price (simulation cannot guess the fill price)".to_string()
        ));
    }
    let order_price = price.unwrap_or(0.0);
    if order_price <= 0.0 {
        return Err(IndodaxError::Other(
            format!("[PAPER] Price must be positive, got {}", order_price)
        ));
    }
    let quote = pair.split('_').next_back().unwrap_or("idr");

    let amount = {
        let quote_balance = state.balances.entry(quote.to_string()).or_insert(0.0);
        if *quote_balance + BALANCE_EPSILON < idr_amount {
                return Err(IndodaxError::Other(
                    format!("[PAPER] Insufficient {} balance. Need {}, have {}",
                        quote.to_uppercase(), format_balance(quote, idr_amount), format_balance(quote, *quote_balance))
                ));
        }
        *quote_balance -= idr_amount;
        round_balance(&mut state.balances, quote);
        idr_amount / order_price
    };

    let order_id = state.next_order_id;
    state.next_order_id += 1;

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64;

    state.orders.push(PaperOrder {
        id: order_id,
        pair: pair.to_string(),
        side: side.to_string(),
        price: order_price,
        amount,
        remaining: amount,
        order_type: "limit".into(),
        status: "open".into(),
        created_at: now,
        fees_paid: 0.0,
        filled_price: 0.0,
        total_spent: idr_amount,
    });

    state.trade_count += 1;

    let price_display = order_price.to_string();
    let data = serde_json::json!({
        "mode": "paper",
        "order_id": order_id,
        "pair": pair,
        "side": side,
        "price": order_price,
        "amount": amount,
        "order_type": "limit",
        "status": "open",
    });

    let headers = vec!["Field".into(), "Value".into()];
    let rows = vec![
        vec!["Order ID".into(), order_id.to_string()],
        vec!["Pair".into(), pair.to_string()],
        vec!["Side".into(), side.to_string()],
        vec!["Price".into(), price_display.clone()],
        vec!["Amount".into(), format!("{:.8}", amount)],
        vec!["IDR Spent".into(), format!("{:.2}", idr_amount)],
        vec!["Type".into(), "limit".into()],
        vec!["Status".into(), "open".into()],
    ];

    let base = pair.split('_').next().unwrap_or("btc");
    Ok(CommandOutput::new(data, headers, rows)
        .with_addendum(format!("[PAPER] buy {} {} for {} IDR @ {} — open", amount, base, idr_amount, price_display)))
}

fn round_balance(balances: &mut HashMap<String, f64>, currency: &str) {
    if let Some(balance) = balances.get_mut(currency) {
        if is_fiat_or_stable(currency) {
            *balance = (*balance * 100.0).round() / 100.0;
        } else {
            *balance = (*balance * 100_000_000.0).round() / 100_000_000.0;
        }
    }
}

fn execute_fill(
    state: &mut PaperState,
    order_id: u64,
    base: &str,
    quote: &str,
    side: &str,
    price: f64,
    amount: f64,
) -> Result<(), IndodaxError> {
    let total = price * amount;
    let fee = total * TAKER_FEE;
    if side == "buy" {
        let quote_balance = state.balances.entry(quote.to_string()).or_insert(0.0);
        if *quote_balance < fee {
            return Err(IndodaxError::Other(format!(
                "[PAPER] Insufficient {} balance to pay fee of {:.2}. Need {:.2}, have {:.2}",
                quote.to_uppercase(), fee, fee, *quote_balance
            )));
        }
        *quote_balance -= fee;
        round_balance(&mut state.balances, quote);
        let base_balance = state.balances.entry(base.to_string()).or_insert(0.0);
        *base_balance += amount;
    } else {
        let quote_balance = state.balances.entry(quote.to_string()).or_insert(0.0);
        *quote_balance += total - fee;
        round_balance(&mut state.balances, quote);
    }

    if let Some(order) = state.orders.iter_mut().find(|o| o.id == order_id) {
        order.remaining = 0.0;
        order.status = "filled".to_string();
        order.fees_paid = fee;
        order.filled_price = price;
    }
    state.total_fees_paid += fee;
    Ok(())
}

fn sort_paper_orders(orders: &mut Vec<&PaperOrder>, sort_by: Option<&str>, sort_order: Option<&str>) {
    let desc = sort_order.map(|o| o == "desc" || o == "d").unwrap_or(false);
    let by = sort_by.unwrap_or("id");
    orders.sort_by(|a, b| {
        let cmp = match by {
            "id" => a.id.cmp(&b.id),
            "pair" => a.pair.cmp(&b.pair),
            "side" => a.side.cmp(&b.side),
            "price" => a.price.partial_cmp(&b.price).unwrap_or(std::cmp::Ordering::Equal),
            "amount" => a.amount.partial_cmp(&b.amount).unwrap_or(std::cmp::Ordering::Equal),
            "remaining" => a.remaining.partial_cmp(&b.remaining).unwrap_or(std::cmp::Ordering::Equal),
            "status" => a.status.cmp(&b.status),
            _ => a.id.cmp(&b.id),
        };
        if desc { cmp.reverse() } else { cmp }
    });
}

fn paper_orders(state: &PaperState, filter_pair: Option<&str>, sort_by: Option<&str>, sort_order: Option<&str>) -> Result<CommandOutput, IndodaxError> {
    let mut filtered: Vec<&PaperOrder> = state
        .orders
        .iter()
        .filter(|o| o.status == "open")
        .filter(|o| filter_pair.map_or(true, |p| o.pair == p))
        .collect();

    sort_paper_orders(&mut filtered, sort_by, sort_order);

    let headers = vec![
        "Order ID".into(), "Pair".into(), "Side".into(), "Price".into(),
        "Amount".into(), "Remaining".into(), "Status".into(),
    ];
    let rows: Vec<Vec<String>> = filtered
        .iter()
        .map(|o| {
            vec![
                o.id.to_string(),
                o.pair.clone(),
                o.side.clone(),
                o.price.to_string(),
                o.amount.to_string(),
                o.remaining.to_string(),
                o.status.clone(),
            ]
        })
        .collect();

    let data = serde_json::json!({
        "mode": "paper",
        "orders": filtered,
        "count": filtered.len(),
    });

    let msg = match filter_pair {
        Some(p) => format!("[PAPER] {} orders for {}", filtered.len(), p),
        None => format!("[PAPER] {} orders", filtered.len()),
    };

    Ok(CommandOutput::new(data, headers, rows).with_addendum(msg))
}

fn paper_cancel(state: &mut PaperState, order_id: u64) -> Result<CommandOutput, IndodaxError> {
    refund_and_cancel(state, order_id)?;

    let data = serde_json::json!({
        "mode": "paper",
        "order_id": order_id,
        "status": "cancelled"
    });
    Ok(CommandOutput::json(data).with_addendum(format!("[PAPER] Order {} cancelled", order_id)))
}

fn paper_cancel_all(state: &mut PaperState) -> Result<CommandOutput, IndodaxError> {
    let (count, failures) = cancel_all_paper_orders(state);

    let mut data = serde_json::json!({
        "mode": "paper",
        "cancelled_count": count,
        "failed_count": failures.len(),
    });
    if !failures.is_empty() {
        data["failures"] = serde_json::json!(failures.iter().map(|(id, e)| serde_json::json!({
            "order_id": id,
            "error": e,
        })).collect::<Vec<_>>());
    }

    let addendum = if failures.is_empty() {
        format!("[PAPER] Cancelled {} orders", count)
    } else {
        let reasons: Vec<String> = failures.iter().map(|(id, e)| format!("{}: {}", id, e)).collect();
        format!("[PAPER] Cancelled {} orders, {} failed: {}", count, failures.len(), reasons.join("; "))
    };

    Ok(CommandOutput::json(data).with_addendum(addendum))
}

pub fn paper_fill(state: &mut PaperState, order_id: Option<u64>, fill_price: Option<f64>, fill_all: bool) -> Result<CommandOutput, IndodaxError> {
    if fill_all {
        let open_ids: Vec<u64> = state.orders.iter()
            .filter(|o| o.status == "open")
            .map(|o| o.id)
            .collect();

        if open_ids.is_empty() {
            return Ok(CommandOutput::json(serde_json::json!({
                "mode": "paper",
                "filled_count": 0,
            })).with_addendum("[PAPER] No open orders to fill"));
        }

        let mut skipped = 0u64;
        let mut filled = 0u64;
        let mut errors: Vec<String> = Vec::new();
        for id in &open_ids {
            let order = match state.orders.iter().find(|o| o.id == *id) {
                Some(o) => o.clone(),
                None => { skipped += 1; continue; }
            };
            let price = fill_price.unwrap_or(order.price);
            if !price.is_finite() {
                errors.push(format!("Order {}: invalid fill price {}", id, price));
                skipped += 1;
                continue;
            }
            let should_fill = match fill_price {
                Some(fp) => match order.side.as_str() {
                    "buy" => fp <= order.price,
                    "sell" => fp >= order.price,
                    _ => false,
                },
                None => true,
            };
            if !should_fill { skipped += 1; continue; }
            let base = order.pair.split('_').next().unwrap_or("btc").to_string();
            let quote = order.pair.split('_').next_back().unwrap_or("idr").to_string();
            match execute_fill(state, *id, &base, &quote, &order.side, price, order.remaining) {
                Ok(()) => filled += 1,
                Err(e) => {
                    errors.push(format!("Order {}: {}", id, e));
                    skipped += 1;
                }
            }
        }

        let data = serde_json::json!({
            "mode": "paper",
            "filled_count": filled,
            "skipped_count": skipped,
            "error_count": errors.len(),
            "errors": errors,
        });

        let addendum = if !errors.is_empty() {
            format!("[PAPER] Filled {} order(s), {} errors: {}", filled, errors.len(), errors.join("; "))
        } else if skipped > 0 {
            let skip_reason = if fill_price.is_some() {
                " (orders not matching fill price condition)"
            } else {
                ""
            };
            format!("[PAPER] Filled {} order(s), skipped {}{}", filled, skipped, skip_reason)
        } else {
            format!("[PAPER] Filled {} order(s)", filled)
        };

        return Ok(CommandOutput::json(data).with_addendum(addendum));
    }

    let order_id = order_id.ok_or_else(|| IndodaxError::Other("[PAPER] Either --order-id or --all must be specified".into()))?;

    let (status, side, pair, order_price, amount, remaining) = {
        let order = state.orders.iter().find(|o| o.id == order_id)
            .ok_or_else(|| IndodaxError::Other(format!("[PAPER] Order {} not found", order_id)))?;
        (order.status.clone(), order.side.clone(), order.pair.clone(), order.price, order.amount, order.remaining)
    };

    if status != "open" {
        return Err(IndodaxError::Other(format!("[PAPER] Order {} status is '{}', only open orders can be filled", order_id, status)));
    }

    let price = fill_price.unwrap_or(order_price);
    if !price.is_finite() {
        return Err(IndodaxError::Other(format!(
            "[PAPER] Invalid fill price: {}. Ensure order price or explicit fill price is valid.",
            price
        )));
    }
    if let Some(fp) = fill_price {
        let should_fill = match side.as_str() {
            "buy" => fp <= order_price,
            "sell" => fp >= order_price,
            _ => false,
        };
        if !should_fill {
            return Err(IndodaxError::Other(format!(
                "[PAPER] Fill price {} does not match order condition ({} side, limit {}). Use --all to skip non-matching orders.",
                fp, side, order_price
            )));
        }
    }
    let base = pair.split('_').next().unwrap_or("btc");
    let quote = pair.split('_').next_back().unwrap_or("idr");

    execute_fill(state, order_id, base, quote, &side, price, remaining)?;

    let data = serde_json::json!({
        "mode": "paper",
        "order_id": order_id,
        "pair": pair,
        "side": side,
        "price": price,
        "amount": amount,
        "status": "filled",
    });

    let headers = vec!["Field".into(), "Value".into()];
    let rows = vec![
        vec!["Order ID".into(), order_id.to_string()],
        vec!["Pair".into(), pair],
        vec!["Side".into(), side],
        vec!["Price".into(), price.to_string()],
        vec!["Amount".into(), amount.to_string()],
        vec!["Total".into(), (price * amount).to_string()],
        vec!["Status".into(), "filled".into()],
    ];

    Ok(CommandOutput::new(data, headers, rows)
        .with_addendum(format!("[PAPER] Order {} filled at {}", order_id, price)))
}

fn paper_history(state: &PaperState, sort_by: Option<&str>, sort_order: Option<&str>) -> Result<CommandOutput, IndodaxError> {
    let mut sorted: Vec<&PaperOrder> = state.orders.iter().collect();
    sort_paper_orders(&mut sorted, sort_by, sort_order);

    let headers = vec![
        "Order ID".into(),
        "Pair".into(),
        "Side".into(),
        "Price".into(),
        "Amount".into(),
        "Status".into(),
    ];
    let rows: Vec<Vec<String>> = sorted
        .iter()
        .map(|o| {
            vec![
                o.id.to_string(),
                o.pair.clone(),
                o.side.clone(),
                o.price.to_string(),
                o.amount.to_string(),
                o.status.clone(),
            ]
        })
        .collect();

    let data = paper_history_value(state);
    let order_count = state.orders.len();
    Ok(CommandOutput::new(data, headers, rows)
        .with_addendum(format!("[PAPER] {} order(s) total", order_count)))
}

fn paper_status(state: &PaperState) -> Result<CommandOutput, IndodaxError> {
    let data = paper_status_value(state);
    let filled_count = data["filled_count"].as_u64().unwrap_or(0);
    let open_count = data["open_count"].as_u64().unwrap_or(0);
    let cancelled_count = data["cancelled_count"].as_u64().unwrap_or(0);

    let pnl_parts: Vec<(String, String)> = state
        .balances
        .iter()
        .filter_map(|(k, v)| {
            let init = state.initial_balance(k);
            if init > 0.0 || *v > 0.0 {
                let diff = *v - init;
                Some((
                    k.to_uppercase(),
                    format!("{} ({})", format_balance(k, *v), format!("{:+.8}", diff)),
                ))
            } else {
                None
            }
        })
        .collect();

    let headers = vec!["Metric".into(), "Value".into()];
    let mut rows = vec![
        vec!["Total trades".into(), state.trade_count.to_string()],
        vec!["Orders filled".into(), filled_count.to_string()],
        vec!["Orders open".into(), open_count.to_string()],
        vec!["Orders cancelled".into(), cancelled_count.to_string()],
        vec![
            "Total fees paid".into(),
            format!("{:.8}", state.total_fees_paid),
        ],
    ];
    for (currency, bal) in &pnl_parts {
        rows.push(vec![format!("{} balance", currency), bal.clone()]);
    }

    Ok(CommandOutput::new(data, headers, rows).with_addendum(format!(
        "[PAPER] {} trades, {} filled, {} open, {} cancelled",
        state.trade_count, filled_count, open_count, cancelled_count
    )))
}

async fn fetch_market_prices(client: &IndodaxClient, state: &PaperState) -> Result<HashMap<String, f64>, IndodaxError> {
    let pairs: std::collections::BTreeSet<String> = state.orders.iter()
        .filter(|o| o.status == "open")
        .map(|o| o.pair.clone())
        .collect();

    if pairs.is_empty() {
        return Ok(HashMap::new());
    }

    let tasks: Vec<_> = pairs.iter().map(|pair| {
        let pair = pair.clone();
        let path = format!("/api/ticker/{}", pair);
        async move {
            match client.public_get::<serde_json::Value>(&path).await {
                Ok(data) => {
                    if let Some(ticker) = data.get("ticker") {
                        let last = ticker.get("last")
                            .and_then(|v| v.as_str())
                            .and_then(|s| s.parse::<f64>().ok())
                            .or_else(|| ticker.get("last").and_then(|v| v.as_f64()));
                        if let Some(price) = last {
                            Some((pair, price))
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                }
                Err(e) => {
                    eprintln!("[PAPER] Warning: Failed to fetch price for {}: {}", pair, e);
                    None
                }
            }
        }
    }).collect();

    let results = join_all(tasks).await;
    let mut prices = HashMap::new();
    for result in results.into_iter().flatten() {
        prices.insert(result.0, result.1);
    }
    Ok(prices)
}

pub async fn paper_check_fills(client: &IndodaxClient, state: &mut PaperState, prices: Option<&str>, fetch: bool) -> Result<CommandOutput, IndodaxError> {
    let market_prices: HashMap<String, f64> = if fetch {
        fetch_market_prices(client, state).await?
    } else if let Some(p) = prices {
        serde_json::from_str(p)
            .map_err(|e| IndodaxError::Other(format!("[PAPER] Invalid prices JSON: {}", e)))?
    } else {
        return Err(IndodaxError::Other("[PAPER] Either --prices or --fetch must be specified".into()));
    };

    // Normalize price keys to match stored order pairs
    let market_prices: HashMap<String, f64> = market_prices
        .into_iter()
        .map(|(k, v)| (helpers::normalize_pair(&k), v))
        .collect();

    let mut filled_ids: Vec<u64> = Vec::new();
    let mut errors: Vec<String> = Vec::new();

    let open_ids: Vec<(u64, String, String, f64, f64)> = state.orders.iter()
        .filter(|o| o.status == "open")
        .map(|o| (o.id, o.pair.clone(), o.side.clone(), o.price, o.remaining))
        .collect();

    for (order_id, pair, side, order_price, remaining) in &open_ids {
        let current_price = match market_prices.get(pair) {
            Some(p) => *p,
            None => continue,
        };

        let should_fill = match side.as_str() {
            "buy" => current_price <= *order_price,
            "sell" => current_price >= *order_price,
            _ => false,
        };

        if should_fill {
            let base = pair.split('_').next().unwrap_or("btc");
            let quote = pair.split('_').next_back().unwrap_or("idr");
            match execute_fill(state, *order_id, base, quote, side, current_price, *remaining) {
                Ok(()) => filled_ids.push(*order_id),
                Err(e) => errors.push(format!("Order {}: {}", order_id, e)),
            }
        }
    }

    let data = serde_json::json!({
        "mode": "paper",
        "filled_count": filled_ids.len(),
        "filled_ids": filled_ids,
        "error_count": errors.len(),
        "errors": errors,
    });

    let msg = if !errors.is_empty() {
        format!("[PAPER] Filled {} order(s) with {} error(s): {}",
            filled_ids.len(), errors.len(), errors.join("; "))
    } else if filled_ids.is_empty() {
        "[PAPER] No orders matched market conditions".to_string()
    } else {
        format!("[PAPER] Filled {} order(s): {}",
            filled_ids.len(),
            filled_ids.iter().map(|id| id.to_string()).collect::<Vec<_>>().join(", "))
    };

    Ok(CommandOutput::json(data).with_addendum(msg))
}

// ──────────────────────────────────────────────
// Public helpers for MCP tools (Value-returning)
// ──────────────────────────────────────────────

pub fn paper_balance_value(state: &PaperState) -> serde_json::Value {
    let rounded: std::collections::HashMap<String, f64> = state.balances.iter()
        .map(|(k, v)| {
            let val = match k.as_str() {
                "idr" | "usdt" | "usdc" => (*v * 100.0).round() / 100.0,
                _ => (*v * 100_000_000.0).round() / 100_000_000.0,
            };
            (k.clone(), val)
        })
        .collect();
    serde_json::json!({
        "mode": "paper",
        "balances": rounded,
    })
}

pub fn paper_orders_value(state: &PaperState) -> serde_json::Value {
    let open_orders: Vec<&PaperOrder> = state
        .orders
        .iter()
        .filter(|o| o.status == "open")
        .collect();
    let count = open_orders.len();
    let orders: Vec<serde_json::Value> = open_orders
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
    serde_json::json!({
        "mode": "paper",
        "count": count,
        "orders": orders,
    })
}

pub fn paper_history_value(state: &PaperState) -> serde_json::Value {
    serde_json::json!({
        "mode": "paper",
        "orders": state.orders,
        "count": state.orders.len(),
    })
}

pub fn paper_status_value(state: &PaperState) -> serde_json::Value {
    let filled = state.orders.iter().filter(|o| o.status == "filled").count();
    let open = state.orders.iter().filter(|o| o.status == "open").count();
    let cancelled = state.orders.iter().filter(|o| o.status == "cancelled").count();
    let pnl: std::collections::HashMap<String, serde_json::Value> = state
        .balances
        .iter()
        .filter_map(|(k, v)| {
            let init = state.initial_balance(k);
            if init > 0.0 || *v > 0.0 {
                Some((k.to_uppercase(), serde_json::json!({
                    "current": v,
                    "initial": init,
                    "diff": v - init,
                })))
            } else {
                None
            }
        })
        .collect();
    serde_json::json!({
        "mode": "paper",
        "trade_count": state.trade_count,
        "filled_count": filled,
        "open_count": open,
        "cancelled_count": cancelled,
        "total_fees_paid": state.total_fees_paid,
        "balances": state.balances,
        "initial_balances": state.initial_balances,
        "pnl": pnl,
    })
}

// ──────────────────────────────────────────────
// Public helpers for MCP tools
// ──────────────────────────────────────────────

/// Cancel a paper order by ID (public wrapper for MCP tools).
pub fn cancel_paper_order(state: &mut PaperState, order_id: u64) -> Result<(), IndodaxError> {
    refund_and_cancel(state, order_id)
}

/// Cancel all paper orders that can be cancelled (public wrapper for MCP tools).
/// Returns the number of cancelled orders.
pub fn cancel_all_paper_orders(state: &mut PaperState) -> (usize, Vec<(u64, String)>) {
    let active_ids: Vec<u64> = state
        .orders
        .iter()
        .filter(|o| o.status == "open")
        .map(|o| o.id)
        .collect();

    let mut success_count = 0usize;
    let mut failures = Vec::new();
    for id in &active_ids {
        match refund_and_cancel(state, *id) {
            Ok(()) => success_count += 1,
            Err(e) => failures.push((*id, e.to_string())),
        }
    }
    (success_count, failures)
}

fn refund_and_cancel(state: &mut PaperState, order_id: u64) -> Result<(), IndodaxError> {
    let order = state.orders.iter().find(|o| o.id == order_id)
        .ok_or_else(|| IndodaxError::Other(format!("[PAPER] Order {} not found", order_id)))?;

    if order.status == "filled" || order.status == "cancelled" {
        return Err(IndodaxError::Other(format!("[PAPER] Order {} already {}", order_id, order.status)));
    }

    let base = order.pair.split('_').next().unwrap_or("btc");
    let quote = order.pair.split('_').next_back().unwrap_or("idr");
    let refund = order.price * order.remaining;
    let remaining = order.remaining;

    if order.side == "buy" {
        *state.balances.entry(quote.to_string()).or_insert(0.0) += refund;
        round_balance(&mut state.balances, quote);
    } else {
        *state.balances.entry(base.to_string()).or_insert(0.0) += remaining;
    }

    if let Some(order) = state.orders.iter_mut().find(|o| o.id == order_id) {
        order.status = "cancelled".to_string();
        order.remaining = 0.0;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::IndodaxConfig;
    use serde_json::json;

    #[test]
    fn test_paper_state_default() {
        let state = PaperState::default();
        assert_eq!(state.balances.get("idr"), Some(&100_000_000.0));
        assert_eq!(state.balances.get("btc"), Some(&1.0));
        assert!(state.orders.is_empty());
        assert_eq!(state.next_order_id, 1);
        assert_eq!(state.trade_count, 0);
        assert_eq!(state.total_fees_paid, 0.0);
        assert!(state.initial_balances.is_some());
    }

    #[test]
    fn test_paper_state_load_none() {
        let config = IndodaxConfig::default();
        let state = PaperState::load(&config);
        assert_eq!(state.balances.get("idr"), Some(&100_000_000.0));
    }

    #[test]
    fn test_paper_state_load_some() {
        let mut config = IndodaxConfig::default();
        let state_json = json!({
            "balances": {"btc": 2.0, "idr": 50_000_000.0},
            "orders": [],
            "next_order_id": 5,
            "trade_count": 3,
            "total_fees_paid": 0.0,
            "initial_balances": {"btc": 2.0, "idr": 50_000_000.0}
        });
        config.paper_balances = Some(state_json);
        
        let state = PaperState::load(&config);
        assert_eq!(state.balances.get("btc"), Some(&2.0));
        assert_eq!(state.next_order_id, 5);
        assert_eq!(state.trade_count, 3);
        assert_eq!(state.total_fees_paid, 0.0);
    }

    #[test]
    fn test_paper_state_save() {
        let mut config = IndodaxConfig::default();
        let mut state = PaperState::default();
        state.balances.insert("eth".into(), 10.0);
        state.next_order_id = 42;
        
        let result = state.save(&mut config);
        assert!(result.is_ok());
        assert!(config.paper_balances.is_some());
    }

    #[test]
    fn test_paper_init() {
        let mut state = PaperState::default();
        state.balances.insert("eth".into(), 100.0);
        state.next_order_id = 99;
        
        let output = paper_init(&mut state, None, None).unwrap();
        assert_eq!(state.balances.get("idr"), Some(&100_000_000.0));
        assert_eq!(state.balances.get("btc"), Some(&1.0));
        assert_eq!(state.next_order_id, 1);
        assert_eq!(state.total_fees_paid, 0.0);
        assert!(state.initial_balances.is_some());
        assert!(output.render().contains("initialized"));
    }

    #[test]
    fn test_paper_reset() {
        let mut state = PaperState {
            balances: { let mut m = std::collections::HashMap::new(); m.insert("custom".into(), 999.0); m },
            orders: vec![PaperOrder {
                id: 1, pair: "test".into(), side: "buy".into(), price: 1.0,
                amount: 1.0, remaining: 0.0, order_type: "limit".into(),
                status: "filled".into(), created_at: 0, fees_paid: 0.0, filled_price: 0.0,
                total_spent: 0.0,
            }],
            next_order_id: 50,
            trade_count: 10,
            total_fees_paid: 0.0,
            initial_balances: None,
        };
        
        let output = paper_reset(&mut state).unwrap();
        assert_eq!(state.balances.get("idr"), Some(&100_000_000.0));
        assert_eq!(state.next_order_id, 1);
        assert_eq!(state.trade_count, 0);
        assert!(output.render().contains("reset"));
    }

    #[test]
    fn test_paper_balance() {
        let mut state = PaperState::default();
        state.balances.insert("eth".into(), 5.0);
        
        let output = paper_balance(&state).unwrap();
        let rendered = output.render();
        assert!(rendered.contains("IDR") || rendered.contains("BTC") || rendered.contains("ETH"));
    }

    #[test]
    fn test_place_paper_order_buy() {
        let mut state = PaperState::default();
        let result = place_paper_order(&mut state, "btc_idr", "buy", Some(100_000.0), 0.5);
        
        assert!(result.is_ok());
        assert_eq!(state.balances.get("idr").unwrap(), &99950000.0);
        assert_eq!(state.balances.get("btc").unwrap(), &1.0);
        assert_eq!(state.orders.len(), 1);
        assert_eq!(state.trade_count, 1);
        assert_eq!(state.orders[0].status, "open");
    }

    #[test]
    fn test_place_paper_order_sell() {
        let mut state = PaperState::default();
        let result = place_paper_order(&mut state, "btc_idr", "sell", Some(100_000_000.0), 0.5);
        
        assert!(result.is_ok());
        assert_eq!(state.balances.get("btc").unwrap(), &0.5);
        assert_eq!(state.balances.get("idr").unwrap(), &100000000.0);
        assert_eq!(state.orders[0].status, "open");
    }

    #[test]
    fn test_place_paper_order_insufficient_quote() {
        let mut state = PaperState::default();
        // Try to buy with insufficient IDR
        let result = place_paper_order(&mut state, "btc_idr", "buy", Some(200_000_000.0), 1.0);
        
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Insufficient"));
    }

    #[test]
    fn test_place_paper_order_insufficient_base() {
        let mut state = PaperState::default();
        // Try to sell more BTC than we have
        let result = place_paper_order(&mut state, "btc_idr", "sell", Some(100_000_000.0), 2.0);
        
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Insufficient"));
    }

    #[test]
    fn test_paper_orders_empty() {
        let state = PaperState::default();
        let output = paper_orders(&state, None, None, None).unwrap();
        assert!(output.render().len() > 0);
    }

    #[test]
    fn test_paper_orders_with_orders() {
        let mut state = PaperState::default();
        place_paper_order(&mut state, "btc_idr", "buy", Some(100_000.0), 0.5).unwrap();
        place_paper_order(&mut state, "btc_idr", "sell", Some(110_000_000.0), 0.3).unwrap();
        
        let output = paper_orders(&state, None, None, None).unwrap();
        let rendered = output.render();
        assert!(rendered.contains("btc_idr"));
    }

    #[test]
    fn test_paper_orders_filter_by_pair() {
        let mut state = PaperState::default();
        place_paper_order(&mut state, "btc_idr", "buy", Some(100_000.0), 0.5).unwrap();
        place_paper_order(&mut state, "eth_idr", "buy", Some(10_000_000.0), 1.0).unwrap();
        
        let output = paper_orders(&state, Some("btc_idr"), None, None).unwrap();
        let rendered = output.render();
        assert!(rendered.contains("btc_idr"));
        assert!(!rendered.contains("eth_idr"));
    }

    #[test]
    fn test_paper_cancel() {
        let mut state = PaperState::default();
        place_paper_order(&mut state, "btc_idr", "buy", Some(100_000.0), 0.5).unwrap();
        let order_id = state.orders[0].id;
        
        // Order is open, cancel should succeed and refund balance
        let output = paper_cancel(&mut state, order_id);
        assert!(output.is_ok());
        assert_eq!(state.orders[0].status, "cancelled");
        assert_eq!(state.balances.get("idr").unwrap(), &100000000.0);
    }

    #[test]
    fn test_paper_cancel_not_found() {
        let mut state = PaperState::default();
        let output = paper_cancel(&mut state, 999);
        assert!(output.is_err());
    }

    #[test]
    fn test_paper_cancel_already_filled() {
        let mut state = PaperState::default();
        place_paper_order(&mut state, "btc_idr", "buy", Some(100_000.0), 0.5).unwrap();
        let order_id = state.orders[0].id;
        
        // First cancel should succeed (order is open)
        paper_cancel(&mut state, order_id).unwrap();
        // Second cancel should fail (already cancelled)
        let output = paper_cancel(&mut state, order_id);
        assert!(output.is_err());
        assert!(output.unwrap_err().to_string().contains("already cancelled"));
    }

    #[test]
    fn test_paper_cancel_all() {
        let mut state = PaperState::default();
        place_paper_order(&mut state, "btc_idr", "buy", Some(100_000.0), 0.5).unwrap();
        place_paper_order(&mut state, "eth_idr", "buy", Some(10_000_000.0), 1.0).unwrap();
        
        // Orders are open, cancel_all should cancel them
        let output = paper_cancel_all(&mut state);
        assert!(output.is_ok());
        assert_eq!(state.orders[0].status, "cancelled");
        assert_eq!(state.orders[1].status, "cancelled");
    }

    #[test]
    fn test_paper_cancel_all_no_orders() {
        let mut state = PaperState::default();
        let output = paper_cancel_all(&mut state);
        assert!(output.is_ok());
    }

    #[test]
    fn test_paper_history() {
        let mut state = PaperState::default();
        place_paper_order(&mut state, "btc_idr", "buy", Some(100_000.0), 0.5).unwrap();
        
        let output = paper_history(&state, None, None).unwrap();
        assert!(output.render().len() > 0);
    }

    #[test]
    fn test_paper_status() {
        let mut state = PaperState::default();
        place_paper_order(&mut state, "btc_idr", "buy", Some(100_000.0), 0.5).unwrap();
        
        let output = paper_status(&state).unwrap();
        let rendered = output.render();
        assert!(rendered.contains("trade_count") || rendered.contains("Trade") || rendered.contains("BTC"));
    }

    #[test]
    fn test_paper_fill_buy() {
        let mut state = PaperState::default();
        place_paper_order(&mut state, "btc_idr", "buy", Some(100_000.0), 0.5).unwrap();
        let order_id = state.orders[0].id;
        
        let result = paper_fill(&mut state, Some(order_id), None, false);
        assert!(result.is_ok());
        assert_eq!(state.orders[0].status, "filled");
        assert_eq!(state.orders[0].remaining, 0.0);
    }

    #[test]
    fn test_paper_fill_sell() {
        let mut state = PaperState::default();
        place_paper_order(&mut state, "btc_idr", "sell", Some(100_000_000.0), 0.5).unwrap();
        let order_id = state.orders[0].id;
        
        let result = paper_fill(&mut state, Some(order_id), None, false);
        assert!(result.is_ok());
        assert_eq!(state.orders[0].status, "filled");
    }

    #[test]
    fn test_paper_fill_not_found() {
        let mut state = PaperState::default();
        let result = paper_fill(&mut state, Some(999), None, false);
        assert!(result.is_err());
    }

    #[test]
    fn test_paper_fill_already_filled() {
        let mut state = PaperState::default();
        place_paper_order(&mut state, "btc_idr", "buy", Some(100_000.0), 0.5).unwrap();
        let order_id = state.orders[0].id;
        
        paper_fill(&mut state, Some(order_id), None, false).unwrap();
        let result = paper_fill(&mut state, Some(order_id), None, false);
        assert!(result.is_err());
    }

    #[test]
    fn test_paper_fill_with_custom_price() {
        let mut state = PaperState::default();
        place_paper_order(&mut state, "btc_idr", "buy", Some(100_000.0), 0.5).unwrap();
        let order_id = state.orders[0].id;
        
        // Buy @ 100k, fill at 90k (better price) -> OK
        let result = paper_fill(&mut state, Some(order_id), Some(90_000.0), false);
        assert!(result.is_ok());
        assert_eq!(state.orders[0].status, "filled");
        assert_eq!(state.orders[0].filled_price, 90_000.0);

        // Try to fill already filled order
        let result2 = paper_fill(&mut state, Some(order_id), Some(80_000.0), false);
        assert!(result2.is_err());
    }

    #[test]
    fn test_paper_fill_invalid_price_condition() {
        let mut state = PaperState::default();
        place_paper_order(&mut state, "btc_idr", "buy", Some(100_000.0), 0.5).unwrap();
        let order_id = state.orders[0].id;
        
        // Buy @ 100k, try to fill at 110k (worse price) -> Error
        let result = paper_fill(&mut state, Some(order_id), Some(110_000.0), false);
        assert!(result.is_err());
        assert_eq!(state.orders[0].status, "open");
    }

    #[test]
    fn test_paper_fill_all() {
        let mut state = PaperState::default();
        place_paper_order(&mut state, "btc_idr", "buy", Some(100_000.0), 0.5).unwrap();
        place_paper_order(&mut state, "btc_idr", "sell", Some(110_000_000.0), 0.3).unwrap();
        
        let result = paper_fill(&mut state, None, None, true);
        assert!(result.is_ok());
        assert_eq!(state.orders[0].status, "filled");
        assert_eq!(state.orders[1].status, "filled");
    }

    #[test]
    fn test_paper_fill_all_no_open_orders() {
        let mut state = PaperState::default();
        let result = paper_fill(&mut state, None, None, true);
        assert!(result.is_ok());
    }

    #[test]
    fn test_paper_topup_negative() {
        let mut state = PaperState::default();
        let result = paper_topup(&mut state, "idr", -5000.0);
        assert!(result.is_err(), "Negative topup should be rejected");
        assert!(result.unwrap_err().to_string().contains("positive"));
    }

    #[test]
    fn test_place_paper_order_negative_amount() {
        let mut state = PaperState::default();
        let result = place_paper_order(&mut state, "btc_idr", "buy", Some(100_000.0), -0.5);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("positive"));
    }

    #[test]
    fn test_place_paper_order_negative_price() {
        let mut state = PaperState::default();
        let result = place_paper_order(&mut state, "btc_idr", "buy", Some(-100.0), 0.5);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("positive"));
    }

    #[test]
    fn test_place_paper_order_zero_amount() {
        let mut state = PaperState::default();
        let result = place_paper_order(&mut state, "btc_idr", "buy", Some(100_000.0), 0.0);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("positive"));
    }

    #[test]
    fn test_execute_fill_buy() {
        let mut state = PaperState::default();
        state.balances.insert("btc".into(), 0.0);
        state.balances.insert("idr".into(), 100_000_000.0);
        
        let result = execute_fill(&mut state, 1, "btc", "idr", "buy", 100_000.0, 0.5);
        assert!(result.is_ok());
        assert_eq!(state.balances.get("btc").unwrap(), &0.5);
    }

    #[test]
    fn test_execute_fill_sell() {
        let mut state = PaperState::default();
        state.balances.insert("btc".into(), 1.0);
        state.balances.insert("idr".into(), 0.0);
        
        let result = execute_fill(&mut state, 1, "btc", "idr", "sell", 100_000_000.0, 0.5);
        assert!(result.is_ok());
        assert_eq!(state.balances.get("idr").unwrap(), &49870000.0);
    }

    #[test]
    fn test_paper_order_fields() {
        let order = PaperOrder {
            id: 1,
            pair: "btc_idr".into(),
            side: "buy".into(),
            price: 100_000.0,
            amount: 0.5,
            remaining: 0.0,
            order_type: "limit".into(),
            status: "filled".into(),
            created_at: 12345,
            fees_paid: 0.0,
            filled_price: 100_000.0,
            total_spent: 0.0,
        };
        
        assert_eq!(order.id, 1);
        assert_eq!(order.pair, "btc_idr");
        assert_eq!(order.side, "buy");
        assert_eq!(order.total_spent, 0.0);
    }

    #[tokio::test]
    async fn test_dispatch_paper_init() {
        let client = IndodaxClient::new(None).unwrap();
        let mut state = PaperState::default();
        let cmd = PaperCommand::Init { idr: None, btc: None };
        let result = dispatch_paper(&client, &mut state, &cmd).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_dispatch_paper_balance() {
        let client = IndodaxClient::new(None).unwrap();
        let state = PaperState::default();
        let cmd = PaperCommand::Balance;
        let result = dispatch_paper(&client, &mut state.clone(), &cmd).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_paper_check_fills_buy_match() {
        let client = IndodaxClient::new(None).unwrap();
        let mut state = PaperState::default();
        place_paper_order(&mut state, "btc_idr", "buy", Some(100_000_000.0), 0.5).unwrap();
        let prices = r#"{"btc_idr": 90000000}"#;

        let result = paper_check_fills(&client, &mut state, Some(prices), false).await;
        assert!(result.is_ok());
        assert_eq!(state.orders[0].status, "filled");
    }

    #[tokio::test]
    async fn test_paper_check_fills_buy_no_match() {
        let client = IndodaxClient::new(None).unwrap();
        let mut state = PaperState::default();
        place_paper_order(&mut state, "btc_idr", "buy", Some(100_000_000.0), 0.5).unwrap();
        let prices = r#"{"btc_idr": 110000000}"#;

        let result = paper_check_fills(&client, &mut state, Some(prices), false).await;
        assert!(result.is_ok());
        assert_eq!(state.orders[0].status, "open");
    }

    #[tokio::test]
    async fn test_paper_check_fills_sell_match() {
        let client = IndodaxClient::new(None).unwrap();
        let mut state = PaperState::default();
        place_paper_order(&mut state, "btc_idr", "sell", Some(100_000_000.0), 0.5).unwrap();
        let prices = r#"{"btc_idr": 110000000}"#;

        let result = paper_check_fills(&client, &mut state, Some(prices), false).await;
        assert!(result.is_ok());
        assert_eq!(state.orders[0].status, "filled");
    }

    #[tokio::test]
    async fn test_paper_check_fills_multiple_orders() {
        let client = IndodaxClient::new(None).unwrap();
        let mut state = PaperState::default();
        place_paper_order(&mut state, "btc_idr", "buy", Some(100_000_000.0), 0.5).unwrap();
        place_paper_order(&mut state, "eth_idr", "buy", Some(10_000_000.0), 1.0).unwrap();
        place_paper_order(&mut state, "btc_idr", "sell", Some(120_000_000.0), 0.3).unwrap();
        let prices = r#"{"btc_idr": 90000000, "eth_idr": 15000000}"#;

        let result = paper_check_fills(&client, &mut state, Some(prices), false).await;
        assert!(result.is_ok());
        // BTC buy matches (90M <= 100M), ETH buy doesn't (15M > 10M), BTC sell doesn't (90M < 120M)
        assert_eq!(state.orders[0].status, "filled");
        assert_eq!(state.orders[1].status, "open");
        assert_eq!(state.orders[2].status, "open");
    }

    #[tokio::test]
    async fn test_paper_check_fills_invalid_json() {
        let client = IndodaxClient::new(None).unwrap();
        let mut state = PaperState::default();
        let result = paper_check_fills(&client, &mut state, Some("not-json"), false).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_paper_check_fills_empty_prices() {
        let client = IndodaxClient::new(None).unwrap();
        let mut state = PaperState::default();
        place_paper_order(&mut state, "btc_idr", "buy", Some(100_000_000.0), 0.5).unwrap();
        let result = paper_check_fills(&client, &mut state, Some(r#"{}"#), false).await;
        assert!(result.is_ok());
        assert_eq!(state.orders[0].status, "open");
    }

    #[tokio::test]
    async fn test_paper_check_fills_no_open_orders() {
        let client = IndodaxClient::new(None).unwrap();
        let mut state = PaperState::default();
        let result = paper_check_fills(&client, &mut state, Some(r#"{"btc_idr": 90000000}"#), false).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_paper_check_fills_fetch_not_available() {
        let client = IndodaxClient::new(None).unwrap();
        let mut state = PaperState::default();
        place_paper_order(&mut state, "btc_idr", "buy", Some(100_000_000.0), 0.5).unwrap();
        // With --fetch but no network, the function handles gracefully — fetch errors
        // are non-fatal (printed as warnings), so the result should be Ok with no fills
        let result = paper_check_fills(&client, &mut state, None, true).await;
        assert!(result.is_ok(), "Should handle fetch failure without error: {:?}", result.err());
        assert_eq!(state.orders[0].status, "open", "Order should remain open when prices unavailable");
        assert_eq!(state.orders[0].remaining, 0.5, "Remaining amount should be unchanged");
    }

    #[test]
    fn test_paper_lifecycle_buy_fill_cancel() {
        let mut state = PaperState::default();
        let initial_idr = *state.balances.get("idr").unwrap();
        let initial_btc = *state.balances.get("btc").unwrap();

        // Buy order
        let result = place_paper_order(&mut state, "btc_idr", "buy", Some(100_000.0), 0.5);
        assert!(result.is_ok());
        let order_id = state.orders[0].id;
        assert_eq!(state.orders[0].status, "open");
        // IDR deducted
        assert!(*state.balances.get("idr").unwrap() < initial_idr);

        // Fill the buy
        let result = paper_fill(&mut state, Some(order_id), None, false);
        assert!(result.is_ok());
        assert_eq!(state.orders[0].status, "filled");
        // BTC received
        assert!(*state.balances.get("btc").unwrap() > initial_btc);

        // Already filled, cancel should fail
        let result = paper_cancel(&mut state, order_id);
        assert!(result.is_err());

        // Orders listing now only shows open orders
        let output = paper_orders(&state, None, None, None).unwrap();
        assert!(!output.render().contains("filled"));
        // History should still show all orders
        let history = paper_history(&state, None, None).unwrap();
        assert!(history.render().contains("filled"));
    }

    #[test]
    fn test_paper_lifecycle_sell_cancel_topup() {
        let mut state = PaperState::default();

        // Sell order
        let result = place_paper_order(&mut state, "btc_idr", "sell", Some(100_000_000.0), 0.5);
        assert!(result.is_ok());
        let order_id = state.orders[0].id;
        assert_eq!(state.orders[0].status, "open");

        // Cancel the sell - BTC should be refunded
        let btc_before = *state.balances.get("btc").unwrap();
        let result = paper_cancel(&mut state, order_id);
        assert!(result.is_ok());
        assert_eq!(state.orders[0].status, "cancelled");
        assert!(*state.balances.get("btc").unwrap() > btc_before);

        // Topup
        let result = paper_topup(&mut state, "usdt", 1000.0);
        assert!(result.is_ok());
        assert_eq!(*state.balances.get("usdt").unwrap(), 1000.0);

        // Status should show correct counts
        let output = paper_status(&state).unwrap();
        let rendered = output.render();
        assert!(rendered.contains("cancelled") || rendered.contains("Cancelled"));
    }

    #[tokio::test]
    async fn test_paper_lifecycle_multiple_orders_and_check_fills() {
        let client = IndodaxClient::new(None).unwrap();
        let mut state = PaperState::default();

        // Add ETH balance for selling
        state.balances.insert("eth".into(), 5.0);

        // Place multiple orders
        place_paper_order(&mut state, "btc_idr", "buy", Some(100_000_000.0), 0.5).unwrap();
        place_paper_order(&mut state, "eth_idr", "sell", Some(10_000_000.0), 2.0).unwrap();
        place_paper_order(&mut state, "btc_idr", "buy", Some(90_000_000.0), 0.3).unwrap();

        // Check fills with market prices - btc buy at 100M matches when market is 95M
        let prices = r#"{"btc_idr": 95000000, "eth_idr": 12000000}"#;
        let result = paper_check_fills(&client, &mut state, Some(prices), false).await;
        assert!(result.is_ok());

        // First BTC buy (100M) should fill (95M <= 100M)
        assert_eq!(state.orders[0].status, "filled");
        // ETH sell (10M) should fill (12M >= 10M)
        assert_eq!(state.orders[1].status, "filled");
        // Second BTC buy (90M) should NOT fill (95M > 90M)
        assert_eq!(state.orders[2].status, "open");

        // Now fill the remaining one with --all
        let result = paper_fill(&mut state, None, None, true);
        assert!(result.is_ok());
        assert_eq!(state.orders[2].status, "filled");
    }
}
