use crate::client::IndodaxClient;
use crate::commands::helpers;
use crate::output::CommandOutput;
use anyhow::Result;
use std::collections::HashMap;

const BALANCE_EPSILON: f64 = 1e-8;

async fn get_account_info(client: &IndodaxClient) -> Result<serde_json::Value> {
    let params = HashMap::new();
    Ok(client.private_post_v1("getInfo", &params).await?)
}

#[derive(Debug, clap::Subcommand)]
pub enum TradeCommand {
    #[command(name = "buy", about = "Place a buy order")]
    Buy {
        #[arg(short, long)]
        pair: String,
        #[arg(short = 'i', long, help = "The total IDR amount to spend.")]
        idr: f64,
        #[arg(long, help = "Limit price. If omitted, a market order will be placed.")]
        price: Option<f64>,
    },

    #[command(name = "sell", about = "Place a sell order")]
    Sell {
        #[arg(short, long)]
        pair: String,
        #[arg(short = 'a', long, help = "Amount in base currency (e.g. BTC)")]
        amount: f64,
        #[arg(short = 'r', long, help = "Limit price. If omitted, a market order will be placed.")]
        price: Option<f64>,
    },

    #[command(name = "cancel", about = "Cancel an order by ID")]
    Cancel {
        #[arg(short = 'i', long)]
        order_id: u64,
        #[arg(short = 'p', long)]
        pair: String,
        #[arg(short = 't', long, help = "Order side: buy or sell")]
        order_type: String,
    },

    #[command(name = "cancel-by-client-id", about = "Cancel an order by client order ID")]
    CancelByClientId {
        #[arg(long)]
        client_order_id: String,
    },

    #[command(name = "cancel-all", about = "Cancel all open orders, optionally filtered by pair")]
    CancelAll {
        #[arg(short, long, help = "Only cancel orders for this trading pair (e.g. btc_idr)")]
        pair: Option<String>,
    },

    #[command(name = "countdown", about = "Start deadman switch countdown")]
    CountdownCancelAll {
        #[arg(short, long)]
        pair: Option<String>,
        #[arg(short, long, help = "Countdown in milliseconds (0 to disable)")]
        countdown_time: u64,
    },
}

pub async fn execute(
    client: &IndodaxClient,
    cmd: &TradeCommand,
) -> Result<CommandOutput> {
    match cmd {
        TradeCommand::Buy { pair, idr, price } => {
            let pair = helpers::normalize_pair(pair);
            place_buy_order(client, &pair, *idr, *price).await
        }
        TradeCommand::Sell { pair, price, amount } => {
            let pair = helpers::normalize_pair(pair);
            place_sell_order(client, &pair, *price, *amount).await
        }
        TradeCommand::Cancel { order_id, pair, order_type } => {
            let pair = helpers::normalize_pair(pair);
            cancel_order(client, *order_id, &pair, order_type).await
        }
        TradeCommand::CancelByClientId { client_order_id } => {
            cancel_by_client_id(client, client_order_id).await
        }
        TradeCommand::CancelAll { pair } => {
            let pair = pair.as_ref().map(|p| helpers::normalize_pair(p));
            cancel_all_orders(client, pair.as_deref()).await
        }
        TradeCommand::CountdownCancelAll { pair, countdown_time } => {
            let pair = pair.as_ref().map(|p| helpers::normalize_pair(p));
            countdown_cancel_all(client, pair.as_deref(), *countdown_time).await
        }
    }
}

async fn place_buy_order(
    client: &IndodaxClient,
    pair: &str,
    idr_amount: f64,
    price: Option<f64>,
) -> Result<CommandOutput> {
    let info = get_account_info(client).await?;

    if idr_amount <= 0.0 {
        return Err(anyhow::anyhow!("IDR amount must be positive, got {}", idr_amount));
    }

    let idr_balance = info["balance"]["idr"]
        .as_str()
        .and_then(|s| s.parse::<f64>().ok())
        .or_else(|| info["balance"]["idr"].as_f64())
        .unwrap_or(0.0);

    if idr_balance + BALANCE_EPSILON < idr_amount {
        return Err(anyhow::anyhow!(
            "Insufficient IDR balance. Need {:.2}, have {:.2}",
            idr_amount, idr_balance
        ));
    }

    let mut params = HashMap::new();
    params.insert("pair".to_string(), pair.to_string());
    params.insert("type".to_string(), "buy".to_string());
    params.insert("idr".to_string(), idr_amount.to_string());

    let order_type_str = if let Some(p) = price {
        if p <= 0.0 {
            return Err(anyhow::anyhow!("Price must be positive, got {}", p));
        }
        params.insert("price".to_string(), p.to_string());
        "limit"
    } else {
        params.insert("order_type".to_string(), "market".to_string());
        "market"
    };

    let data: serde_json::Value =
        client.private_post_v1("trade", &params).await?;

    let headers = vec!["Field".into(), "Value".into()];
    let mut rows: Vec<Vec<String>> = Vec::new();
    if let serde_json::Value::Object(ref map) = data {
        for (k, v) in map {
            rows.push(vec![k.clone(), helpers::value_to_string(v)]);
        }
    }

    Ok(CommandOutput::new(data, headers, rows)
        .with_addendum(format!("Buy order ({}) placed for {} IDR on pair {}", order_type_str, idr_amount, pair)))
}

async fn place_sell_order(
    client: &IndodaxClient,
    pair: &str,
    price: Option<f64>,
    amount: f64,
) -> Result<CommandOutput> {
    let base_currency = pair.split('_').next().unwrap_or_default();
    if base_currency.is_empty() {
        return Err(anyhow::anyhow!("Invalid pair format: {}", pair));
    }

    let info = get_account_info(client).await?;

    if amount <= 0.0 {
        return Err(anyhow::anyhow!("Amount must be positive, got {}", amount));
    }

    let base_balance = info["balance"][base_currency]
        .as_str()
        .and_then(|s| s.parse::<f64>().ok())
        .or_else(|| info["balance"][base_currency].as_f64())
        .unwrap_or(0.0);

    if base_balance + BALANCE_EPSILON < amount {
        return Err(anyhow::anyhow!(
            "Insufficient {} balance. Need {:.8}, have {:.8}",
            base_currency.to_uppercase(), amount, base_balance
        ));
    }

    let mut params = HashMap::new();
    params.insert("pair".to_string(), pair.to_string());
    params.insert("type".to_string(), "sell".to_string());
    params.insert(base_currency.to_string(), amount.to_string());

    let order_type = if let Some(p) = price {
        if p <= 0.0 {
            return Err(anyhow::anyhow!("Price must be positive, got {}", p));
        }
        params.insert("price".to_string(), p.to_string());
        "limit"
    } else {
        params.insert("order_type".to_string(), "market".to_string());
        "market"
    };

    let data: serde_json::Value =
        client.private_post_v1("trade", &params).await?;

    let headers = vec!["Field".into(), "Value".into()];
    let mut rows: Vec<Vec<String>> = Vec::new();
    if let serde_json::Value::Object(ref map) = data {
        for (k, v) in map {
            rows.push(vec![k.clone(), helpers::value_to_string(v)]);
        }
    }

    let addendum = if let Some(p) = price {
        format!("Sell order placed: {} {} @ {} ({})", amount, pair, p, order_type)
    } else {
        format!("Sell order ({}) placed for {} {} on pair {}", order_type, amount, pair.split('_').next().unwrap_or(""), pair)
    };

    Ok(CommandOutput::new(data, headers, rows).with_addendum(addendum))
}

async fn cancel_order(
    client: &IndodaxClient,
    order_id: u64,
    pair: &str,
    order_type: &str,
) -> Result<CommandOutput> {
    let normalized = order_type.to_lowercase();
    if normalized != "buy" && normalized != "sell" {
        return Err(anyhow::anyhow!(
            "Invalid order type '{}'. Must be 'buy' or 'sell'", order_type
        ));
    }

    let mut params = HashMap::new();
    params.insert("order_id".into(), order_id.to_string());
    params.insert("pair".into(), pair.to_string());
    params.insert("type".into(), normalized);

    let data: serde_json::Value =
        client.private_post_v1("cancelOrder", &params).await?;

    let headers = vec!["Field".into(), "Value".into()];
    let mut rows: Vec<Vec<String>> = Vec::new();
    if let serde_json::Value::Object(ref map) = data {
        for (k, v) in map {
            rows.push(vec![k.clone(), helpers::value_to_string(v)]);
        }
    }

    Ok(CommandOutput::new(data, headers, rows)
        .with_addendum(format!("Cancelled order {} on {}", order_id, pair)))
}

async fn cancel_by_client_id(
    client: &IndodaxClient,
    client_order_id: &str,
) -> Result<CommandOutput> {
    let mut params = HashMap::new();
    params.insert("client_order_id".into(), client_order_id.to_string());

    let data: serde_json::Value =
        client.private_post_v1("cancelByClientOrderId", &params).await?;

    let headers = vec!["Field".into(), "Value".into()];
    let mut rows: Vec<Vec<String>> = Vec::new();
    if let serde_json::Value::Object(ref map) = data {
        for (k, v) in map {
            rows.push(vec![k.clone(), helpers::value_to_string(v)]);
        }
    }

    Ok(CommandOutput::new(data, headers, rows)
        .with_addendum(format!("Cancelled order by client order ID: {}", client_order_id)))
}

async fn cancel_all_orders(
    client: &IndodaxClient,
    pair: Option<&str>,
) -> Result<CommandOutput> {
    if pair.is_none() {
        use dialoguer::Confirm;
        let confirmed = Confirm::new()
            .with_prompt("No --pair filter specified. This will cancel ALL orders across ALL pairs. Continue?")
            .default(false)
            .interact()
            .unwrap_or(false);
        if !confirmed {
            return Ok(CommandOutput::json(serde_json::json!({
                "cancelled": false,
                "reason": "user_cancelled",
            })).with_addendum("Cancel all orders aborted by user."));
        }
    }
    let mut params = std::collections::HashMap::new();
    if let Some(p) = pair {
        params.insert("pair".to_string(), p.to_string());
    }
    let data: serde_json::Value = client.private_post_v1("openOrders", &params).await?;

    let orders = &data["orders"];
    let mut cancelled_ids: Vec<String> = Vec::new();
    let mut failed_ids: Vec<String> = Vec::new();

    if let serde_json::Value::Object(orders_map) = orders {
        for (order_id, order_val) in orders_map {
            let order_pair = helpers::value_to_string(
                order_val.get("pair")
                    .or_else(|| order_val.get("market"))
                    .or_else(|| order_val.get("symbol"))
                    .unwrap_or(&serde_json::Value::Null),
            );
            let order_type = order_val.get("type")
                .or_else(|| order_val.get("order_type"))
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();

            let mut cancel_params = std::collections::HashMap::new();
            cancel_params.insert("order_id".to_string(), order_id.clone());
            cancel_params.insert("pair".to_string(), order_pair);
            cancel_params.insert("type".to_string(), order_type);
            match client.private_post_v1::<serde_json::Value>("cancelOrder", &cancel_params).await {
                Ok(_) => cancelled_ids.push(order_id.clone()),
                Err(e) => failed_ids.push(format!("{} ({})", order_id, e)),
            }
        }
    }

    let headers = vec!["Metric".into(), "Value".into()];
    let mut rows = vec![
        vec!["Cancelled".into(), cancelled_ids.len().to_string()],
        vec!["Order IDs".into(), cancelled_ids.join(", ")],
    ];
    if !failed_ids.is_empty() {
        rows.push(vec!["Failed".into(), failed_ids.len().to_string()]);
        rows.push(vec!["Failed IDs".into(), failed_ids.join(", ")]);
    }

    let data = serde_json::json!({
        "cancelled_count": cancelled_ids.len(),
        "cancelled_ids": cancelled_ids,
        "failed_count": failed_ids.len(),
        "failed_ids": failed_ids,
    });

    let addendum = if failed_ids.is_empty() {
        format!("Cancelled {} order(s)", cancelled_ids.len())
    } else {
        format!("Cancelled {} order(s), {} failed", cancelled_ids.len(), failed_ids.len())
    };

    Ok(CommandOutput::new(data, headers, rows)
        .with_addendum(addendum))
}

async fn countdown_cancel_all(
    client: &IndodaxClient,
    pair: Option<&str>,
    countdown_time: u64,
) -> Result<CommandOutput> {
    let data = client.countdown_cancel_all(pair, countdown_time).await
        .map_err(|e| anyhow::anyhow!("{}", e))?;

    let msg = if countdown_time == 0 {
        "Deadman switch disabled".into()
    } else {
        format!("Deadman switch active: {}ms countdown", countdown_time)
    };

    Ok(CommandOutput::json(data).with_addendum(msg))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_trade_command_variants() {
        let _cmd1 = TradeCommand::Buy { 
            pair: "btc_idr".into(), 
            idr: 100_000.0, 
            price: Some(100_000_000.0) 
        };
        let _cmd2 = TradeCommand::Sell { 
            pair: "btc_idr".into(), 
            price: Some(100_000_000.0), 
            amount: 0.5, 
        };
        let _cmd3 = TradeCommand::Cancel { 
            order_id: 123, 
            pair: "btc_idr".into(), 
            order_type: "buy".into() 
        };
        let _cmd4 = TradeCommand::CancelByClientId { 
            client_order_id: "client_123".into() 
        };
        let _cmd5 = TradeCommand::CancelAll { 
            pair: Some("btc_idr".into()),
        };
        let _cmd6 = TradeCommand::CountdownCancelAll { 
            pair: Some("btc_idr".into()), 
            countdown_time: 60000 
        };
    }

    #[test]
    fn test_trade_command_buy_market_order() {
        let cmd = TradeCommand::Buy { 
            pair: "btc_idr".into(), 
            idr: 100_000.0, 
            price: None 
        };
        match cmd {
            TradeCommand::Buy { pair, idr, price } => {
                assert_eq!(pair, "btc_idr");
                assert_eq!(idr, 100_000.0);
                assert!(price.is_none());
            }
            _ => assert!(false, "Expected Buy command, got {:?}", cmd),
        }
    }

    #[test]
    fn test_trade_command_sell_market_order() {
        let cmd = TradeCommand::Sell { 
            pair: "eth_idr".into(), 
            price: None, 
            amount: 1.0, 
        };
        match cmd {
            TradeCommand::Sell { price, .. } => {
                assert!(price.is_none());
            }
            _ => assert!(false, "Expected Sell command, got {:?}", cmd),
        }
    }

    #[test]
    fn test_trade_command_sell_limit_order() {
        let cmd = TradeCommand::Sell { 
            pair: "btc_idr".into(), 
            price: Some(100_000_000.0), 
            amount: 0.5, 
        };
        match cmd {
            TradeCommand::Sell { price, .. } => {
                assert_eq!(price, Some(100_000_000.0));
            }
            _ => assert!(false, "Expected Sell command, got {:?}", cmd),
        }
    }

    #[test]
    fn test_trade_command_cancel_all_no_pair() {
        let cmd = TradeCommand::CountdownCancelAll { 
            pair: None, 
            countdown_time: 0 
        };
        match cmd {
            TradeCommand::CountdownCancelAll { pair, countdown_time } => {
                assert!(pair.is_none());
                assert_eq!(countdown_time, 0);
            }
            _ => assert!(false, "Expected CountdownCancelAll command, got {:?}", cmd),
        }
    }

    #[test]
    fn test_trade_cancel_all_parse() {
        let cmd = TradeCommand::CancelAll { 
            pair: Some("btc_idr".into()),
        };
        match cmd {
            TradeCommand::CancelAll { pair } => {
                assert_eq!(pair, Some("btc_idr".into()));
            }
            _ => assert!(false, "Expected CancelAll command, got {:?}", cmd),
        }
    }

    #[test]
    fn test_trade_cancel_all_no_pair_filter() {
        let cmd = TradeCommand::CancelAll { 
            pair: None,
        };
        match cmd {
            TradeCommand::CancelAll { pair } => {
                assert!(pair.is_none());
            }
            _ => assert!(false, "Expected CancelAll command, got {:?}", cmd),
        }
    }
}
