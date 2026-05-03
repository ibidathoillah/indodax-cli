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
        #[arg(long)]
        price: f64,
        #[arg(short, long, help = "Amount in base currency (e.g. BTC)")]
        amount: f64,
        #[arg(long, default_value = "limit")]
        order_type: String,
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
        TradeCommand::Buy { pair, price, amount, order_type } => {
            place_order(client, pair, *price, *amount, order_type, "buy").await
        }
        TradeCommand::Sell { pair, price, amount, order_type } => {
            place_order(client, pair, *price, *amount, order_type, "sell").await
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

async fn place_order(
    client: &IndodaxClient,
    pair: &str,
    price: f64,
    amount: f64,
    order_type: &str,
    side: &str,
) -> Result<CommandOutput> {
    let price_str = if order_type == "market" {
        "0".to_string()
    } else {
        let pair_lower = pair.to_lowercase();
        let precision = match pair_lower.as_str() {
            p if p.contains("idr") => 0,
            p if p.contains("usdt") => 2,
            _ => 8,
        };
        format!("{:.precision$}", price, precision = precision)
    };

    let amount_precision = 8;
    let amount_str = format!("{:.precision$}", amount, precision = amount_precision);

    let mut params = HashMap::new();
    params.insert("pair".into(), pair.to_string());
    params.insert("type".into(), side.to_string());
    params.insert("price".into(), price_str);
    params.insert(order_type.to_string().into(), amount_str);
    if order_type == "market" {
        params.insert("type".into(), format!("{}_{}", side, order_type));
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
        .with_addendum(format!("Order placed: {} {} {} @ {} ({})", side, amount, pair, price, order_type)))
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
