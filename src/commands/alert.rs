use crate::client::IndodaxClient;
use crate::commands::helpers;
use crate::output::CommandOutput;
use crate::alerts::{PriceAlert, AlertCondition, AlertStatus, load_alerts, save_alerts};
use anyhow::Result;
use colored::*;
use futures_util::{SinkExt, StreamExt};
use tokio_tungstenite::{connect_async, tungstenite::Message};

#[derive(Debug, clap::Subcommand)]
pub enum AlertCommand {
    #[command(name = "add", about = "Add a price alert")]
    Add {
        #[arg(short = 'p', long, help = "Trading pair (e.g. btc_idr)")]
        pair: String,
        #[arg(long, help = "Alert when price goes above this value")]
        above: Option<f64>,
        #[arg(long, help = "Alert when price goes below this value")]
        below: Option<f64>,
        #[arg(long, help = "Alert when price increases by this percent")]
        percent_up: Option<f64>,
        #[arg(long, help = "Alert when price decreases by this percent")]
        percent_down: Option<f64>,
        #[arg(short = 'n', long, help = "Note for this alert")]
        note: Option<String>,
    },

    #[command(name = "list", about = "List all price alerts")]
    List {
        #[arg(long, help = "Include triggered and cancelled alerts")]
        history: bool,
    },

    #[command(name = "cancel", about = "Cancel a price alert")]
    Cancel {
        #[arg(short = 'i', long, help = "Alert ID to cancel")]
        id: Option<u64>,
        #[arg(long, help = "Cancel all alerts")]
        all: bool,
    },

    #[command(name = "check", about = "Check alerts against current prices")]
    Check {
        #[arg(short = 'i', long, help = "Check specific alert by ID")]
        id: Option<u64>,
        #[arg(short = 'p', long, help = "Filter by pair (e.g. btc_idr)")]
        pair: Option<String>,
    },

    #[command(name = "watch", about = "Monitor alerts in real-time via WebSocket")]
    Watch {
        #[arg(short = 'i', long, help = "Filter by alert ID")]
        id: Option<u64>,
        #[arg(short = 'p', long, help = "Filter by pair (e.g. btc_idr)")]
        pair: Option<String>,
        #[arg(long, default_value = "60", help = "Price change threshold (%) to trigger")]
        threshold: f64,
    },

    #[command(name = "triggered", about = "Show triggered alerts")]
    Triggered,
}

pub async fn execute(
    client: &IndodaxClient,
    _creds: &Option<crate::config::ResolvedCredentials>,
    cmd: &AlertCommand,
) -> Result<CommandOutput> {
    match cmd {
        AlertCommand::Add { pair, above, below, percent_up, percent_down, note } => {
            let pair = helpers::normalize_pair(pair);
            alert_add(&pair, *above, *below, *percent_up, *percent_down, note.clone(), client).await
        }
        AlertCommand::List { history } => alert_list(*history),
        AlertCommand::Cancel { id, all } => alert_cancel(*id, *all),
        AlertCommand::Check { id, pair } => {
            let pair = pair.as_ref().map(|p| helpers::normalize_pair(p));
            alert_check(client, *id, pair.as_deref()).await
        }
        AlertCommand::Watch { id, pair, threshold } => {
            let pair = pair.as_ref().map(|p| helpers::normalize_pair(p));
            alert_watch(client, *id, pair.as_deref(), *threshold).await
        }
        AlertCommand::Triggered => alert_triggered(),
    }
}

fn get_next_id(alerts: &[PriceAlert]) -> u64 {
    alerts.iter().map(|a| a.id).max().unwrap_or(0) + 1
}

async fn alert_add(
    pair: &str,
    above: Option<f64>,
    below: Option<f64>,
    percent_up: Option<f64>,
    percent_down: Option<f64>,
    note: Option<String>,
    client: &IndodaxClient,
) -> Result<CommandOutput> {
    let condition = if let Some(price) = above {
        if price <= 0.0 {
            return Err(anyhow::anyhow!("Price must be positive, got {}", price));
        }
        AlertCondition::Above { price }
    } else if let Some(price) = below {
        if price <= 0.0 {
            return Err(anyhow::anyhow!("Price must be positive, got {}", price));
        }
        AlertCondition::Below { price }
    } else if let Some(percent) = percent_up {
        if percent <= 0.0 || percent > 1000.0 {
            return Err(anyhow::anyhow!("Percent must be between 0 and 1000, got {}", percent));
        }
        let from_price = fetch_price(client, pair).await?;
        AlertCondition::ChangeUp { percent, from_price }
    } else if let Some(percent) = percent_down {
        if percent <= 0.0 || percent > 1000.0 {
            return Err(anyhow::anyhow!("Percent must be between 0 and 1000, got {}", percent));
        }
        let from_price = fetch_price(client, pair).await?;
        AlertCondition::ChangeDown { percent, from_price }
    } else {
        return Err(anyhow::anyhow!(
            "Must specify one of: --above, --below, --percent-up, or --percent-down"
        ));
    };

    let mut alerts = load_alerts()?;
    let id = get_next_id(&alerts);
    let alert = PriceAlert {
        id,
        pair: pair.to_string(),
        condition,
        created_at: helpers::now_millis(),
        triggered_at: None,
        status: AlertStatus::Active,
        note,
    };

    alerts.push(alert.clone());
    save_alerts(&alerts)?;

    let condition_str = match &alert.condition {
        AlertCondition::Above { price } => format!("above {}", format_number(*price)),
        AlertCondition::Below { price } => format!("below {}", format_number(*price)),
        AlertCondition::ChangeUp { percent, from_price } => format!("+{:.1}% from {}", percent, format_number(*from_price)),
        AlertCondition::ChangeDown { percent, from_price } => format!("-{:.1}% from {}", percent, format_number(*from_price)),
    };

    let data = serde_json::json!({
        "status": "ok",
        "id": id,
        "pair": pair,
        "condition": condition_str,
        "created_at": alert.created_at,
    });

    let headers = vec!["Field".into(), "Value".into()];
    let rows = vec![
        vec!["Alert ID".into(), id.to_string()],
        vec!["Pair".into(), pair.to_string()],
        vec!["Condition".into(), condition_str.clone()],
        vec!["Created".into(), chrono::DateTime::from_timestamp_millis(alert.created_at.min(i64::MAX as u64) as i64)
            .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string())
            .unwrap_or_default()],
    ];

    Ok(CommandOutput::new(data, headers, rows)
        .with_addendum(format!("[ALERT] Created {} alert for {} @ {}", id, pair, condition_str)))
}

fn alert_list(include_history: bool) -> Result<CommandOutput> {
    let alerts = load_alerts()?;

    let filtered: Vec<&PriceAlert> = if include_history {
        alerts.iter().collect()
    } else {
        alerts.iter().filter(|a| a.status == AlertStatus::Active).collect()
    };

    if filtered.is_empty() {
        return Ok(CommandOutput::json(serde_json::json!({
            "status": "ok",
            "message": if include_history { "No alerts" } else { "No active alerts" },
            "alerts": [],
        })));
    }

    let mut headers = vec!["ID".into(), "Pair".into(), "Condition".into(), "Status".into(), "Created".into()];
    if include_history {
        headers.push("Triggered".into());
    }

    let mut rows: Vec<Vec<String>> = Vec::new();
    for alert in &filtered {
        let condition_str = match &alert.condition {
            AlertCondition::Above { price } => format!("> {}", format_number(*price)),
            AlertCondition::Below { price } => format!("< {}", format_number(*price)),
            AlertCondition::ChangeUp { percent, from_price } => format!("+{:.1}% from {}", percent, format_number(*from_price)),
            AlertCondition::ChangeDown { percent, from_price } => format!("-{:.1}% from {}", percent, format_number(*from_price)),
        };

        let mut row = vec![
            alert.id.to_string(),
            alert.pair.clone(),
            condition_str.clone(),
            format!("{:?}", alert.status),
            chrono::DateTime::from_timestamp_millis(alert.created_at.min(i64::MAX as u64) as i64)
                .map(|dt| dt.format("%Y-%m-%d %H:%M").to_string())
                .unwrap_or_default(),
        ];

        if include_history {
            let triggered = alert.triggered_at.map(|t| {
                chrono::DateTime::from_timestamp_millis(t.min(i64::MAX as u64) as i64)
                    .map(|dt| dt.format("%Y-%m-%d %H:%M").to_string())
                    .unwrap_or_default()
            }).unwrap_or_else(|| "-".to_string());
            row.push(triggered);
        }

        rows.push(row);
    }

    let data = serde_json::json!({
        "status": "ok",
        "count": filtered.len(),
    });

    Ok(CommandOutput::new(data, headers, rows)
        .with_addendum(format!("[ALERT] {} alert(s)", filtered.len())))
}

fn alert_cancel(id: Option<u64>, cancel_all: bool) -> Result<CommandOutput> {
    let mut alerts = load_alerts()?;

    if cancel_all {
        let count = alerts.iter().filter(|a| a.status == AlertStatus::Active).count();
        for alert in alerts.iter_mut() {
            if alert.status == AlertStatus::Active {
                alert.status = AlertStatus::Cancelled;
            }
        }
        save_alerts(&alerts)?;

        return Ok(CommandOutput::json(serde_json::json!({
            "status": "ok",
            "message": format!("Cancelled {} alert(s)", count),
            "cancelled": count,
        })).with_addendum(format!("[ALERT] Cancelled {} alert(s)", count)));
    }

    if let Some(target_id) = id {
        let alert = alerts.iter_mut().find(|a| a.id == target_id);
        match alert {
            Some(a) if a.status == AlertStatus::Active => {
                a.status = AlertStatus::Cancelled;
                save_alerts(&alerts)?;

                Ok(CommandOutput::json(serde_json::json!({
                    "status": "ok",
                    "message": format!("Cancelled alert {}", target_id),
                    "id": target_id,
                })).with_addendum(format!("[ALERT] Cancelled alert {}", target_id)))
            }
            Some(_) => Err(anyhow::anyhow!("Alert {} is already cancelled or triggered", target_id)),
            None => Err(anyhow::anyhow!("Alert {} not found", target_id)),
        }
    } else {
        Err(anyhow::anyhow!("Must specify --id or --all"))
    }
}

async fn alert_check(
    client: &IndodaxClient,
    id: Option<u64>,
    pair_filter: Option<&str>,
) -> Result<CommandOutput> {
    let mut alerts = load_alerts()?;

    let to_check: Vec<&mut PriceAlert> = if let Some(target_id) = id {
        alerts.iter_mut().filter(|a| a.id == target_id && a.status == AlertStatus::Active).collect()
    } else {
        let filter = pair_filter.unwrap_or("*");
        alerts.iter_mut()
            .filter(|a| a.status == AlertStatus::Active && (filter == "*" || a.pair == filter))
            .collect()
    };

    if to_check.is_empty() {
        return Ok(CommandOutput::json(serde_json::json!({
            "status": "ok",
            "message": "No active alerts to check",
            "triggered": [],
        })));
    }

    let mut triggered_alerts: Vec<PriceAlert> = Vec::new();

    for alert in to_check {
        let price = match fetch_price(client, &alert.pair).await {
            Ok(p) => p,
            Err(_) => continue,
        };

        let should_trigger = match &alert.condition {
            AlertCondition::Above { price: threshold } => price >= *threshold,
            AlertCondition::Below { price: threshold } => price <= *threshold,
            AlertCondition::ChangeUp { percent, from_price } => {
                let change = ((price - from_price) / from_price) * 100.0;
                change >= *percent
            }
            AlertCondition::ChangeDown { percent, from_price } => {
                let change = ((from_price - price) / from_price) * 100.0;
                change >= *percent
            }
        };

        if should_trigger {
            alert.status = AlertStatus::Triggered;
            alert.triggered_at = Some(helpers::now_millis());
            triggered_alerts.push(alert.clone());
        }
    }

    save_alerts(&alerts)?;

    if triggered_alerts.is_empty() {
        return Ok(CommandOutput::json(serde_json::json!({
            "status": "ok",
            "message": "No alerts triggered",
            "triggered": [],
        })).with_addendum("[ALERT] No alerts triggered"));
    }

    let headers = vec!["ID".into(), "Pair".into(), "Condition".into(), "Price".into(), "Triggered At".into()];
    let mut rows: Vec<Vec<String>> = Vec::new();

    for alert in &triggered_alerts {
        let current_price = fetch_price(client, &alert.pair).await.unwrap_or(0.0);
        let condition_str = match &alert.condition {
            AlertCondition::Above { price } => format!("> {}", format_number(*price)),
            AlertCondition::Below { price } => format!("< {}", format_number(*price)),
            AlertCondition::ChangeUp { percent, from_price } => format!("+{:.1}% from {}", percent, format_number(*from_price)),
            AlertCondition::ChangeDown { percent, from_price } => format!("-{:.1}% from {}", percent, format_number(*from_price)),
        };

        rows.push(vec![
            alert.id.to_string(),
            alert.pair.clone(),
            condition_str.clone(),
            format_number(current_price),
            chrono::DateTime::from_timestamp_millis(alert.triggered_at.unwrap_or(0).min(i64::MAX as u64) as i64)
                .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string())
                .unwrap_or_default(),
        ]);
    }

    let data = serde_json::json!({
        "status": "ok",
        "triggered": triggered_alerts,
        "count": triggered_alerts.len(),
    });

    Ok(CommandOutput::new(data, headers, rows)
        .with_addendum(format!("[ALERT] {} alert(s) triggered!", triggered_alerts.len())))
}

fn alert_triggered() -> Result<CommandOutput> {
    let alerts = load_alerts()?;
    let triggered: Vec<&PriceAlert> = alerts.iter()
        .filter(|a| a.status == AlertStatus::Triggered)
        .collect();

    if triggered.is_empty() {
        return Ok(CommandOutput::json(serde_json::json!({
            "status": "ok",
            "message": "No triggered alerts",
            "count": 0,
        })));
    }

    let headers = vec!["ID".into(), "Pair".into(), "Condition".into(), "Triggered At".into()];
    let mut rows: Vec<Vec<String>> = Vec::new();

    for alert in &triggered {
        let condition_str = match &alert.condition {
            AlertCondition::Above { price } => format!("> {}", format_number(*price)),
            AlertCondition::Below { price } => format!("< {}", format_number(*price)),
            AlertCondition::ChangeUp { percent, from_price } => format!("+{:.1}% from {}", percent, format_number(*from_price)),
            AlertCondition::ChangeDown { percent, from_price } => format!("-{:.1}% from {}", percent, format_number(*from_price)),
        };

        rows.push(vec![
            alert.id.to_string(),
            alert.pair.clone(),
            condition_str.clone(),
            chrono::DateTime::from_timestamp_millis(alert.triggered_at.unwrap_or(0).min(i64::MAX as u64) as i64)
                .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string())
                .unwrap_or_default(),
        ]);
    }

    Ok(CommandOutput::new(
        serde_json::json!({"status": "ok", "count": triggered.len()}),
        headers,
        rows,
    ).with_addendum(format!("[ALERT] {} triggered alert(s)", triggered.len())))
}

async fn fetch_price(client: &IndodaxClient, pair: &str) -> Result<f64> {
    let response: serde_json::Value = client.public_get(&format!("/api/ticker/{}", pair)).await?;

    let price = response.get("ticker")
        .and_then(|t| t.get("last"))
        .and_then(|v| v.as_str())
        .and_then(|s| s.parse::<f64>().ok())
        .or_else(|| {
            response.get("ticker")
                .and_then(|t| t.get("last"))
                .and_then(|v| v.as_f64())
        })
        .ok_or_else(|| anyhow::anyhow!("Failed to parse price for {}", pair))?;

    Ok(price)
}

async fn alert_watch(
    client: &IndodaxClient,
    id: Option<u64>,
    pair_filter: Option<&str>,
    threshold: f64,
) -> Result<CommandOutput> {
    let mut alerts = load_alerts()?;

    let target_ids: std::collections::HashSet<u64> = if let Some(target_id) = id {
        alerts.iter().filter(|a| a.id == target_id && a.status == AlertStatus::Active).map(|a| a.id).collect()
    } else {
        let filter = pair_filter.unwrap_or("*");
        alerts.iter()
            .filter(|a| a.status == AlertStatus::Active && (filter == "*" || a.pair == filter))
            .map(|a| a.id)
            .collect()
    };

    if target_ids.is_empty() {
        return Ok(CommandOutput::json(serde_json::json!({
            "status": "ok",
            "message": "No active alerts to watch",
            "watching": [],
        })));
    }

    let pairs: Vec<String> = alerts.iter()
        .filter(|a| target_ids.contains(&a.id))
        .map(|a| a.pair.clone())
        .collect();
    let pair_set: std::collections::HashSet<String> = pairs.iter().cloned().collect();
    let watching = pair_set.len();

    eprintln!("[ALERT] Watching {} alerts for {} pair(s): {}", target_ids.len(), watching, pairs.join(", "));
    eprintln!("[ALERT] Press Ctrl+C to stop monitoring");
    eprintln!();

    const PUBLIC_WS_URL: &str = "wss://ws3.indodax.com/ws/";

    let token = helpers::fetch_public_ws_token(client).await?;

    let (ws_stream, _) = tokio::time::timeout(std::time::Duration::from_secs(10), connect_async(PUBLIC_WS_URL)).await
        .map_err(|_| anyhow::anyhow!("WebSocket connection timed out after 10s"))?
        .map_err(|e| anyhow::anyhow!("Failed to connect to WebSocket: {}", e))?;

    let (mut write, mut read) = ws_stream.split();

    let auth_msg = serde_json::json!({
        "params": { "token": token },
        "id": 1
    });
    write.send(Message::Text(auth_msg.to_string())).await
        .map_err(|e| anyhow::anyhow!("Failed to authenticate: {}", e))?;

    let mut authed = false;
    let mut last_prices: std::collections::HashMap<String, f64> = std::collections::HashMap::new();
    let mut triggered_count = 0;

    let mut triggered_ids = std::collections::HashSet::new();

    while let Some(msg) = read.next().await {
        match msg {
            Ok(Message::Text(text)) => {
                if let Ok(data) = serde_json::from_str::<serde_json::Value>(&text) {
                    if !authed {
                        if data.get("id").and_then(|v| v.as_i64()) == Some(1)
                            && data.get("result").is_some()
                        {
                            authed = true;
                            eprintln!("[WS] Authenticated, subscribing to pairs...");
                            for pair in &pair_set {
                                let sub_msg = serde_json::json!({
                                    "method": "subscribe",
                                    "params": { "channel": format!("chart:tick-{}", pair) },
                                    "id": 2
                                });
                                write.send(Message::Text(sub_msg.to_string())).await.ok();
                            }
                        }
                        continue;
                    }

                    if let Some(result) = data.get("result").or(data.get("data")) {
                        let pair = result.get("pair").or(data.get("pair")).and_then(|v| v.as_str()).unwrap_or("");
                        let price = result.get("price").or(result.get("c")).or(result.get("close"))
                            .and_then(|v| v.as_str().and_then(|s| s.parse::<f64>().ok()))
                            .or_else(|| result.get("price").or(result.get("c")).and_then(|v| v.as_f64()));

                        if let Some(price) = price {
                            let prev_price = last_prices.get(pair).copied();
                            last_prices.insert(pair.to_string(), price);

                            if let Some(prev) = prev_price {
                                let change_pct = ((price - prev) / prev * 100.0).abs();
                                if change_pct > threshold {
                                    eprintln!("[PRICE] {} {} (change: {:.2}%)",
                                        pair,
                                        format_number(price),
                                        if price > prev { '+' } else { '-' });
                                }
                            }

                            for alert in alerts.iter_mut().filter(|a| a.pair == pair && target_ids.contains(&a.id) && a.status == AlertStatus::Active) {
                                let should_trigger = match &alert.condition {
                                    AlertCondition::Above { price: threshold } => price >= *threshold,
                                    AlertCondition::Below { price: threshold } => price <= *threshold,
                                    AlertCondition::ChangeUp { percent, from_price } => {
                                        let change = ((price - from_price) / from_price) * 100.0;
                                        change >= *percent
                                    }
                                    AlertCondition::ChangeDown { percent, from_price } => {
                                        let change = ((from_price - price) / from_price) * 100.0;
                                        change >= *percent
                                    }
                                };

                                if should_trigger {
                                    alert.status = AlertStatus::Triggered;
                                    alert.triggered_at = Some(helpers::now_millis());
                                    triggered_ids.insert(alert.id);
                                    triggered_count += 1;
                                    let condition_str = match &alert.condition {
                                        AlertCondition::Above { price } => format!("> {}", format_number(*price)),
                                        AlertCondition::Below { price } => format!("< {}", format_number(*price)),
                                        AlertCondition::ChangeUp { percent, from_price } => format!("+{:.1}% from {}", percent, format_number(*from_price)),
                                        AlertCondition::ChangeDown { percent, from_price } => format!("-{:.1}% from {}", percent, format_number(*from_price)),
                                    };
                                    eprintln!();
                                    eprintln!("{}", "=".repeat(60).yellow());
                                    eprintln!("{} TRIGGERED {} {}", "[ALERT]".bold().green(), format!("#{}", alert.id).bold(), "!".green().bold());
                                    eprintln!("  Pair:      {}", pair);
                                    eprintln!("  Condition: {}", condition_str);
                                    eprintln!("  Price:     {} (triggered)", format_number(price));
                                    if let Some(note) = &alert.note {
                                        eprintln!("  Note:      {}", note);
                                    }
                                    eprintln!("{}", "=".repeat(60).yellow());
                                    eprintln!();
                                }
                            }
                        }
                    }
                }
            }
            Ok(Message::Ping(data)) => {
                write.send(Message::Pong(data)).await.ok();
            }
            Ok(Message::Close(_)) => {
                break;
            }
            Err(e) => {
                eprintln!("[WARN] WebSocket error: {}", e);
                break;
            }
            _ => {}
        }
    }

    eprintln!("\n[ALERT] Monitoring stopped. {} alert(s) triggered.", triggered_count);

    if triggered_count > 0 {
        save_alerts(&alerts)?;
    }

    let data = serde_json::json!({
        "status": "ok",
        "watching": target_ids.len(),
        "pairs": pairs,
        "triggered": triggered_count,
    });

    Ok(CommandOutput::new(data, vec![], vec![]).with_addendum(format!(
        "[ALERT] Watched {} alert(s) for {} pair(s). {} triggered.",
        target_ids.len(), watching, triggered_count
    )))
}

fn format_number(n: f64) -> String {
    if n >= 1_000_000_000.0 {
        format!("{:.2}B", n / 1_000_000_000.0)
    } else if n >= 1_000_000.0 {
        format!("{:.2}M", n / 1_000_000.0)
    } else if n >= 1_000.0 {
        format!("{:.2}K", n / 1_000.0)
    } else if n >= 1.0 {
        format!("{:.2}", n)
    } else {
        format!("{:.8}", n)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_number() {
        assert_eq!(format_number(1_500_000_000.0), "1.50B");
        assert_eq!(format_number(100_000_000.0), "100.00M");
        assert_eq!(format_number(50_000.0), "50.00K");
        assert_eq!(format_number(1_000.0), "1.00K");
        assert_eq!(format_number(100.0), "100.00");
        assert_eq!(format_number(0.00001), "0.00001000");
    }

    #[test]
    fn test_alert_condition_serialization() {
        let above = AlertCondition::Above { price: 100000000.0 };
        let json = serde_json::to_string(&above).unwrap();
        assert!(json.contains("\"type\":\"above\""));

        let below = AlertCondition::Below { price: 50000000.0 };
        let json = serde_json::to_string(&below).unwrap();
        assert!(json.contains("\"type\":\"below\""));

        let change_up = AlertCondition::ChangeUp { percent: 5.0, from_price: 100000000.0 };
        let json = serde_json::to_string(&change_up).unwrap();
        assert!(json.contains("\"type\":\"change_up\""));
        assert!(json.contains("5.0"));

        let change_down = AlertCondition::ChangeDown { percent: 10.0, from_price: 150000000.0 };
        let json = serde_json::to_string(&change_down).unwrap();
        assert!(json.contains("\"type\":\"change_down\""));
    }

    #[test]
    fn test_alert_status_serialization() {
        let active = AlertStatus::Active;
        let json = serde_json::to_string(&active).unwrap();
        assert_eq!(json, "\"active\"");

        let triggered = AlertStatus::Triggered;
        let json = serde_json::to_string(&triggered).unwrap();
        assert_eq!(json, "\"triggered\"");
    }
}