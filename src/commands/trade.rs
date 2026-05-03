use crate::client::IndodaxClient;
use crate::commands::helpers;
use crate::output::CommandOutput;
use anyhow::Result;
use std::collections::HashMap;

#[derive(Debug, clap::Subcommand)]
pub enum TradeCommand {
    #[command(name = "buy", about = "Place a buy order")]
    Buy {
        #[arg(short, long)]
        pair: String,
        #[arg(long, help = "The total IDR amount to spend.")]
        idr: f64,
        #[arg(long, help = "Limit price. If omitted, a market order will be placed.")]
        price: Option<f64>,
    },

    #[command(name = "sell", about = "Place a sell order")]
    Sell {
        #[arg(short, long)]
        pair: String,
        #[arg(long)]
        price: f64,
        #[arg(short, long, help = "Amount in base currency (e.g. BTC)")]
        amount: f64,
        #[arg(long, default_value = "limit")]
        order_type: String,
    },

    #[command(name = "cancel", about = "Cancel an order by ID")]
    Cancel {
        #[arg(long)]
        order_id: u64,
        #[arg(short, long)]
        pair: String,
        #[arg(short, long)]
        order_type: String,
    },

    #[command(name = "cancel-by-client-id", about = "Cancel an order by client order ID")]
    CancelByClientId {
        #[arg(long)]
        client_order_id: String,
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
            place_buy_order(client, pair, *idr, *price).await
        }
        TradeCommand::Sell { pair, price, amount, order_type } => {
            place_sell_order(client, pair, *price, *amount, order_type).await
        }
        TradeCommand::Cancel { order_id, pair, order_type } => {
            cancel_order(client, *order_id, pair, order_type).await
        }
        TradeCommand::CancelByClientId { client_order_id } => {
            cancel_by_client_id(client, client_order_id).await
        }
        TradeCommand::CountdownCancelAll { pair, countdown_time } => {
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
    let mut params = HashMap::new();
    params.insert("pair".to_string(), pair.to_string());
    params.insert("type".to_string(), "buy".to_string());
    params.insert("idr".to_string(), idr_amount.to_string());

    let order_type_str = if let Some(p) = price {
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
    price: f64,
    amount: f64,
    order_type: &str,
) -> Result<CommandOutput> {
    let base_currency = pair.split('_').next().unwrap_or_default();
    if base_currency.is_empty() {
        return Err(anyhow::anyhow!("Invalid pair format: {}", pair));
    }
    
    let mut params = HashMap::new();
    params.insert("pair".to_string(), pair.to_string());
    params.insert("type".to_string(), "sell".to_string());
    params.insert("price".to_string(), price.to_string());
    params.insert(base_currency.to_string(), amount.to_string());
    
    if order_type == "market" {
        params.insert("order_type".to_string(), "market".to_string());
    }

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
        .with_addendum(format!("Sell order placed: {} {} @ {} ({})", amount, pair, price, order_type)))
}

async fn cancel_order(
    client: &IndodaxClient,
    order_id: u64,
    pair: &str,
    order_type: &str,
) -> Result<CommandOutput> {
    let mut params = HashMap::new();
    params.insert("order_id".into(), order_id.to_string());
    params.insert("pair".into(), pair.to_string());
    params.insert("type".into(), order_type.to_string());

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

async fn countdown_cancel_all(
    client: &IndodaxClient,
    pair: Option<&str>,
    countdown_time: u64,
) -> Result<CommandOutput> {
    let signer = match client.signer() {
        Some(s) => s,
        None => return Err(anyhow::anyhow!("API credentials required")),
    };

    let mut body_parts: Vec<String> = vec![
        format!("countdownTime={}", countdown_time),
    ];
    if let Some(p) = pair {
        body_parts.push(format!("pair={}", p));
    }

    let body = body_parts.join("&");
    let (payload, signature) = signer.sign_v1(&body, false);

    let http = reqwest::Client::new();
    let resp = http
        .post("https://indodax.com/tapi/countdownCancelAll")
        .header("Key", signer.api_key())
        .header("Sign", &signature)
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body(payload)
        .send()
        .await?;

    let data: serde_json::Value = resp.json().await?;

    let msg = if countdown_time == 0 {
        "Deadman switch disabled".into()
    } else {
        format!("Deadman switch active: {}ms countdown", countdown_time)
    };

    Ok(CommandOutput::json(data).with_addendum(msg))
}
