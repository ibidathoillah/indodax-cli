use crate::config::IndodaxConfig;
use crate::output::CommandOutput;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

const DEFAULT_BALANCE_IDR: f64 = 100_000_000.0;
const DEFAULT_BALANCE_BTC: f64 = 1.0;

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
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaperState {
    pub balances: HashMap<String, f64>,
    pub orders: Vec<PaperOrder>,
    pub next_order_id: u64,
    pub trade_count: u64,
}

impl Default for PaperState {
    fn default() -> Self {
        let mut balances = HashMap::new();
        balances.insert("idr".into(), DEFAULT_BALANCE_IDR);
        balances.insert("btc".into(), DEFAULT_BALANCE_BTC);
        Self {
            balances,
            orders: Vec::new(),
            next_order_id: 1,
            trade_count: 0,
        }
    }
}

impl PaperState {
    pub fn load(config: &IndodaxConfig) -> Self {
        config
            .paper_balances
            .as_ref()
            .and_then(|v| serde_json::from_value(v.clone()).ok())
            .unwrap_or_default()
    }

    pub fn save(&self, config: &mut IndodaxConfig) -> Result<()> {
        config.paper_balances = Some(serde_json::to_value(self)?);
        config.save()?;
        Ok(())
    }
}

#[derive(Debug, clap::Subcommand)]
pub enum PaperCommand {
    #[command(name = "init", about = "Initialize paper trading with default balances")]
    Init,

    #[command(name = "reset", about = "Reset paper trading state")]
    Reset,

    #[command(name = "balance", about = "Show paper trading balances")]
    Balance,

    #[command(name = "buy", about = "Place a simulated buy order")]
    Buy {
        #[arg(short, long, default_value = "btc_idr")]
        pair: String,
        #[arg(long)]
        price: f64,
        #[arg(short, long, help = "Amount in base currency")]
        amount: f64,
    },

    #[command(name = "sell", about = "Place a simulated sell order")]
    Sell {
        #[arg(short, long, default_value = "btc_idr")]
        pair: String,
        #[arg(long)]
        price: f64,
        #[arg(short, long, help = "Amount in base currency")]
        amount: f64,
    },

    #[command(name = "orders", about = "List paper trading orders")]
    Orders,

    #[command(name = "cancel", about = "Cancel a paper order")]
    Cancel {
        #[arg(long)]
        order_id: u64,
    },

    #[command(name = "cancel-all", about = "Cancel all paper orders")]
    CancelAll,

    #[command(name = "history", about = "Show paper trading history")]
    History,

    #[command(name = "status", about = "Show paper trading status summary")]
    Status,
}

pub async fn execute(
    config: &mut IndodaxConfig,
    cmd: &PaperCommand,
) -> Result<CommandOutput> {
    let mut state = PaperState::load(config);
    let result = dispatch_paper(&mut state, cmd);
    state.save(config)?;
    result
}

fn dispatch_paper(
    state: &mut PaperState,
    cmd: &PaperCommand,
) -> Result<CommandOutput> {
    match cmd {
        PaperCommand::Init => paper_init(state),
        PaperCommand::Reset => paper_reset(state),
        PaperCommand::Balance => paper_balance(state),
        PaperCommand::Buy { pair, price, amount } => {
            place_paper_order(state, pair, "buy", *price, *amount)
        }
        PaperCommand::Sell { pair, price, amount } => {
            place_paper_order(state, pair, "sell", *price, *amount)
        }
        PaperCommand::Orders => paper_orders(state),
        PaperCommand::Cancel { order_id } => paper_cancel(state, *order_id),
        PaperCommand::CancelAll => paper_cancel_all(state),
        PaperCommand::History => paper_history(state),
        PaperCommand::Status => paper_status(state),
    }
}

fn paper_init(state: &mut PaperState) -> Result<CommandOutput> {
    *state = PaperState::default();
    let data = serde_json::json!({
        "mode": "paper",
        "status": "initialized",
        "default_balances": {
            "idr": DEFAULT_BALANCE_IDR,
            "btc": DEFAULT_BALANCE_BTC,
        }
    });
    Ok(CommandOutput::json(data).with_addendum("[PAPER] Trading initialized with virtual balances"))
}

fn paper_reset(state: &mut PaperState) -> Result<CommandOutput> {
    *state = PaperState::default();
    let data = serde_json::json!({
        "mode": "paper",
        "status": "reset"
    });
    Ok(CommandOutput::json(data).with_addendum("[PAPER] Trading state reset"))
}

fn paper_balance(state: &PaperState) -> Result<CommandOutput> {
    let headers = vec!["Currency".into(), "Balance".into()];
    let mut rows: Vec<Vec<String>> = state
        .balances
        .iter()
        .map(|(k, v)| vec![k.to_uppercase(), format!("{:.8}", v)])
        .collect();
    rows.sort_by(|a, b| b[1].parse::<f64>().unwrap_or(0.0).partial_cmp(&a[1].parse::<f64>().unwrap_or(0.0)).unwrap_or(std::cmp::Ordering::Equal));

    let data = serde_json::json!({
        "mode": "paper",
        "balances": state.balances,
    });
    Ok(CommandOutput::new(data, headers, rows).with_addendum("[PAPER]"))
}

pub fn place_paper_order(
    state: &mut PaperState,
    pair: &str,
    side: &str,
    price: f64,
    amount: f64,
) -> Result<CommandOutput> {
    let base = pair.split('_').next().unwrap_or(pair);
    let quote = pair.split('_').last().unwrap_or("idr");
    let total_cost = price * amount;

    if side == "buy" {
        let quote_balance = state.balances.entry(quote.to_string()).or_insert(0.0);
        if *quote_balance < total_cost {
            return Err(anyhow::anyhow!(
                "[PAPER] Insufficient {} balance. Need {:.2}, have {:.2}",
                quote.to_uppercase(), total_cost, quote_balance
            ));
        }
        *quote_balance -= total_cost;
    } else {
        let base_balance = state.balances.entry(base.to_string()).or_insert(0.0);
        if *base_balance < amount {
            return Err(anyhow::anyhow!(
                "[PAPER] Insufficient {} balance. Need {:.8}, have {:.8}",
                base.to_uppercase(), amount, base_balance
            ));
        }
        *base_balance -= amount;
    }

    let order_id = state.next_order_id;
    state.next_order_id += 1;

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64;

    state.orders.push(PaperOrder {
        id: order_id,
        pair: pair.to_string(),
        side: side.to_string(),
        price,
        amount,
        remaining: amount,
        order_type: "limit".into(),
        status: "filled".into(),
        created_at: now,
    });

    execute_fill(state, order_id, base, quote, side, price, amount)?;

    state.trade_count += 1;

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
        vec!["Pair".into(), pair.to_string()],
        vec!["Side".into(), side.to_string()],
        vec!["Price".into(), price.to_string()],
        vec!["Amount".into(), amount.to_string()],
        vec!["Total".into(), total_cost.to_string()],
        vec!["Status".into(), "filled".into()],
    ];

    Ok(CommandOutput::new(data, headers, rows)
        .with_addendum(format!("[PAPER] {} {} {} @ {} — filled", side, amount, pair, price)))
}

fn execute_fill(
    state: &mut PaperState,
    order_id: u64,
    base: &str,
    quote: &str,
    side: &str,
    price: f64,
    amount: f64,
) -> Result<()> {
    let total = price * amount;
    if side == "buy" {
        let base_balance = state.balances.entry(base.to_string()).or_insert(0.0);
        *base_balance += amount;
    } else {
        let quote_balance = state.balances.entry(quote.to_string()).or_insert(0.0);
        *quote_balance += total;
    }

    if let Some(order) = state.orders.iter_mut().find(|o| o.id == order_id) {
        order.remaining = 0.0;
        order.status = "filled".to_string();
    }
    Ok(())
}

fn paper_orders(state: &PaperState) -> Result<CommandOutput> {
    let headers = vec![
        "Order ID".into(), "Pair".into(), "Side".into(), "Price".into(),
        "Amount".into(), "Remaining".into(), "Status".into(),
    ];
    let rows: Vec<Vec<String>> = state
        .orders
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
        "orders": state.orders.iter().filter(|o| o.status != "cancelled").collect::<Vec<_>>(),
        "count": state.orders.iter().filter(|o| o.status != "cancelled").count(),
    });

    Ok(CommandOutput::new(data, headers, rows).with_addendum("[PAPER]"))
}

fn paper_cancel(state: &mut PaperState, order_id: u64) -> Result<CommandOutput> {
    if let Some(order) = state.orders.iter_mut().find(|o| o.id == order_id) {
        if order.status == "filled" || order.status == "cancelled" {
            return Err(anyhow::anyhow!(
                "[PAPER] Order {} already {}",
                order_id, order.status
            ));
        }

        let base = order.pair.split('_').next().unwrap_or("btc");
        let quote = order.pair.split('_').last().unwrap_or("idr");
        let refund = order.price * order.remaining;
        if order.side == "buy" {
            *state.balances.entry(quote.to_string()).or_insert(0.0) += refund;
        } else {
            *state.balances.entry(base.to_string()).or_insert(0.0) += order.remaining;
        }
        order.status = "cancelled".to_string();
        order.remaining = 0.0;
    } else {
        return Err(anyhow::anyhow!("[PAPER] Order {} not found", order_id));
    }

    let data = serde_json::json!({
        "mode": "paper",
        "order_id": order_id,
        "status": "cancelled"
    });
    Ok(CommandOutput::json(data).with_addendum(format!("[PAPER] Order {} cancelled", order_id)))
}

fn paper_cancel_all(state: &mut PaperState) -> Result<CommandOutput> {
    let active_ids: Vec<u64> = state
        .orders
        .iter()
        .filter(|o| o.status != "filled" && o.status != "cancelled")
        .map(|o| o.id)
        .collect();

    let count = active_ids.len();
    for id in &active_ids {
        if let Some(order) = state.orders.iter_mut().find(|o| o.id == *id) {
            let base = order.pair.split('_').next().unwrap_or("btc");
            let quote = order.pair.split('_').last().unwrap_or("idr");
            let refund = order.price * order.remaining;
            if order.side == "buy" {
                *state.balances.entry(quote.to_string()).or_insert(0.0) += refund;
            } else {
                *state.balances.entry(base.to_string()).or_insert(0.0) += order.remaining;
            }
            order.status = "cancelled".to_string();
            order.remaining = 0.0;
        }
    }

    let data = serde_json::json!({
        "mode": "paper",
        "cancelled_count": count,
    });
    Ok(CommandOutput::json(data).with_addendum(format!("[PAPER] Cancelled {} orders", count)))
}

fn paper_history(state: &PaperState) -> Result<CommandOutput> {
    let headers = vec![
        "Order ID".into(), "Pair".into(), "Side".into(), "Price".into(),
        "Amount".into(), "Status".into(),
    ];
    let rows: Vec<Vec<String>> = state
        .orders
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

    let data = serde_json::json!({
        "mode": "paper",
        "orders": state.orders,
        "count": state.orders.len(),
    });

    Ok(CommandOutput::new(data, headers, rows).with_addendum("[PAPER]"))
}

fn paper_status(state: &PaperState) -> Result<CommandOutput> {
    let filled_count = state.orders.iter().filter(|o| o.status == "filled").count();
    let open_count = state.orders.iter().filter(|o| o.status != "filled" && o.status != "cancelled").count();
    let cancelled_count = state.orders.iter().filter(|o| o.status == "cancelled").count();

    let headers = vec!["Metric".into(), "Value".into()];
    let rows = vec![
        vec!["Total trades".into(), state.trade_count.to_string()],
        vec!["Orders filled".into(), filled_count.to_string()],
        vec!["Orders open".into(), open_count.to_string()],
        vec!["Orders cancelled".into(), cancelled_count.to_string()],
        vec!["Balances".into(), state.balances.iter()
            .map(|(k, v)| format!("{}: {:.8}", k.to_uppercase(), v))
            .collect::<Vec<_>>()
            .join("  ")],
    ];

    let data = serde_json::json!({
        "mode": "paper",
        "trade_count": state.trade_count,
        "filled_count": filled_count,
        "open_count": open_count,
        "cancelled_count": cancelled_count,
        "balances": state.balances,
    });

    Ok(CommandOutput::new(data, headers, rows).with_addendum("[PAPER]"))
}

// ──────────────────────────────────────────────
// Public helpers for MCP tools
// ──────────────────────────────────────────────

/// Cancel a paper order by ID (public wrapper for MCP tools).
pub fn cancel_paper_order(state: &mut PaperState, order_id: u64) -> Result<()> {
    if let Some(order) = state.orders.iter_mut().find(|o| o.id == order_id) {
        if order.status == "filled" || order.status == "cancelled" {
            return Err(anyhow::anyhow!(
                "[PAPER] Order {} already {}",
                order_id, order.status
            ));
        }

        let base = order.pair.split('_').next().unwrap_or("btc");
        let quote = order.pair.split('_').last().unwrap_or("idr");
        let refund = order.price * order.remaining;
        if order.side == "buy" {
            *state.balances.entry(quote.to_string()).or_insert(0.0) += refund;
        } else {
            *state.balances.entry(base.to_string()).or_insert(0.0) += order.remaining;
        }
        order.status = "cancelled".to_string();
        order.remaining = 0.0;
        Ok(())
    } else {
        Err(anyhow::anyhow!("[PAPER] Order {} not found", order_id))
    }
}

/// Cancel all paper orders that can be cancelled (public wrapper for MCP tools).
/// Returns the number of cancelled orders.
pub fn cancel_all_paper_orders(state: &mut PaperState) -> usize {
    let active_ids: Vec<u64> = state
        .orders
        .iter()
        .filter(|o| o.status != "filled" && o.status != "cancelled")
        .map(|o| o.id)
        .collect();

    let count = active_ids.len();
    for id in &active_ids {
        if let Some(order) = state.orders.iter_mut().find(|o| o.id == *id) {
            let base = order.pair.split('_').next().unwrap_or("btc");
            let quote = order.pair.split('_').last().unwrap_or("idr");
            let refund = order.price * order.remaining;
            if order.side == "buy" {
                *state.balances.entry(quote.to_string()).or_insert(0.0) += refund;
            } else {
                *state.balances.entry(base.to_string()).or_insert(0.0) += order.remaining;
            }
            order.status = "cancelled".to_string();
            order.remaining = 0.0;
        }
    }
    count
}

/// Initialize paper trading state (public wrapper for MCP tools).
pub async fn paper_init_cmd(_config: &IndodaxConfig) -> Option<CommandOutput> {
    Some(CommandOutput::json(serde_json::json!({
        "mode": "paper",
        "status": "initialized",
        "default_balances": {
            "idr": DEFAULT_BALANCE_IDR,
            "btc": DEFAULT_BALANCE_BTC,
        }
    })).with_addendum("[PAPER] Trading initialized with virtual balances"))
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
            "trade_count": 3
        });
        config.paper_balances = Some(state_json);
        
        let state = PaperState::load(&config);
        assert_eq!(state.balances.get("btc"), Some(&2.0));
        assert_eq!(state.next_order_id, 5);
        assert_eq!(state.trade_count, 3);
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
        
        let output = paper_init(&mut state).unwrap();
        assert_eq!(state.balances.get("idr"), Some(&100_000_000.0));
        assert_eq!(state.balances.get("btc"), Some(&1.0));
        assert_eq!(state.next_order_id, 1);
        assert!(output.render().contains("initialized"));
    }

    #[test]
    fn test_paper_reset() {
        let mut state = PaperState {
            balances: { let mut m = std::collections::HashMap::new(); m.insert("custom".into(), 999.0); m },
            orders: vec![PaperOrder {
                id: 1, pair: "test".into(), side: "buy".into(), price: 1.0,
                amount: 1.0, remaining: 0.0, order_type: "limit".into(),
                status: "filled".into(), created_at: 0,
            }],
            next_order_id: 50,
            trade_count: 10,
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
        let result = place_paper_order(&mut state, "btc_idr", "buy", 100_000.0, 0.5);
        
        assert!(result.is_ok());
        assert_eq!(state.balances.get("idr").unwrap(), &(100_000_000.0 - 50_000.0));
        assert_eq!(state.balances.get("btc").unwrap(), &1.5);
        assert_eq!(state.orders.len(), 1);
        assert_eq!(state.trade_count, 1);
    }

    #[test]
    fn test_place_paper_order_sell() {
        let mut state = PaperState::default();
        let result = place_paper_order(&mut state, "btc_idr", "sell", 100_000_000.0, 0.5);
        
        assert!(result.is_ok());
        assert_eq!(state.balances.get("btc").unwrap(), &0.5);
        assert_eq!(state.balances.get("idr").unwrap(), &150_000_000.0);
    }

    #[test]
    fn test_place_paper_order_insufficient_quote() {
        let mut state = PaperState::default();
        // Try to buy with insufficient IDR
        let result = place_paper_order(&mut state, "btc_idr", "buy", 200_000_000.0, 1.0);
        
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Insufficient"));
    }

    #[test]
    fn test_place_paper_order_insufficient_base() {
        let mut state = PaperState::default();
        // Try to sell more BTC than we have
        let result = place_paper_order(&mut state, "btc_idr", "sell", 100_000_000.0, 2.0);
        
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Insufficient"));
    }

    #[test]
    fn test_paper_orders_empty() {
        let state = PaperState::default();
        let output = paper_orders(&state).unwrap();
        assert!(output.render().len() > 0);
    }

    #[test]
    fn test_paper_orders_with_orders() {
        let mut state = PaperState::default();
        place_paper_order(&mut state, "btc_idr", "buy", 100_000.0, 0.5).unwrap();
        place_paper_order(&mut state, "btc_idr", "sell", 110_000_000.0, 0.3).unwrap();
        
        let output = paper_orders(&state).unwrap();
        let rendered = output.render();
        assert!(rendered.contains("btc_idr"));
    }

    #[test]
    fn test_paper_cancel() {
        let mut state = PaperState::default();
        place_paper_order(&mut state, "btc_idr", "buy", 100_000.0, 0.5).unwrap();
        let order_id = state.orders[0].id;
        
        // Order is already filled, so cancel should fail
        let output = paper_cancel(&mut state, order_id);
        assert!(output.is_err());
        assert!(output.unwrap_err().to_string().contains("already filled"));
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
        place_paper_order(&mut state, "btc_idr", "buy", 100_000.0, 0.5).unwrap();
        let order_id = state.orders[0].id;
        
        let output = paper_cancel(&mut state, order_id);
        assert!(output.is_err());
        assert!(output.unwrap_err().to_string().contains("already filled"));
    }

    #[test]
    fn test_paper_cancel_all() {
        let mut state = PaperState::default();
        place_paper_order(&mut state, "btc_idr", "buy", 100_000.0, 0.5).unwrap();
        place_paper_order(&mut state, "eth_idr", "buy", 10_000_000.0, 1.0).unwrap();
        
        // Orders are already filled, so cancel_all should not change their status
        let output = paper_cancel_all(&mut state);
        assert!(output.is_ok());
        // Filled orders remain filled (cancel_all only affects non-filled orders)
        assert_eq!(state.orders[0].status, "filled");
        assert_eq!(state.orders[1].status, "filled");
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
        place_paper_order(&mut state, "btc_idr", "buy", 100_000.0, 0.5).unwrap();
        
        let output = paper_history(&state).unwrap();
        assert!(output.render().len() > 0);
    }

    #[test]
    fn test_paper_status() {
        let mut state = PaperState::default();
        place_paper_order(&mut state, "btc_idr", "buy", 100_000.0, 0.5).unwrap();
        
        let output = paper_status(&state).unwrap();
        let rendered = output.render();
        assert!(rendered.contains("trade_count") || rendered.contains("Trade") || rendered.contains("BTC"));
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
        assert_eq!(state.balances.get("idr").unwrap(), &50_000_000.0);
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
        };
        
        assert_eq!(order.id, 1);
        assert_eq!(order.pair, "btc_idr");
        assert_eq!(order.side, "buy");
    }

    #[test]
    fn test_dispatch_paper_init() {
        let mut state = PaperState::default();
        let cmd = PaperCommand::Init;
        let result = dispatch_paper(&mut state, &cmd);
        assert!(result.is_ok());
    }

    #[test]
    fn test_dispatch_paper_balance() {
        let state = PaperState::default();
        let cmd = PaperCommand::Balance;
        let result = dispatch_paper(&mut state.clone(), &cmd);
        assert!(result.is_ok());
    }
}
