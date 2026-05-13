use crate::client::IndodaxClient;
use crate::commands::helpers;
use crate::output::CommandOutput;
use anyhow::Result;
use std::collections::HashMap;

#[derive(Debug, clap::Subcommand)]
pub enum FundingCommand {
    #[command(name = "withdraw-fee", about = "Check withdrawal fee for a currency")]
    WithdrawFee {
        #[arg(short, long)]
        currency: String,
        #[arg(short, long, help = "Blockchain network (optional)")]
        network: Option<String>,
    },

    #[command(name = "withdraw", about = "Withdraw cryptocurrency")]
    Withdraw {
        #[arg(short, long)]
        currency: String,
        #[arg(short, long, help = "Amount to withdraw")]
        amount: f64,
        #[arg(long, help = "Crypto destination address (or Indodax username if --username is set)")]
        address: String,
        #[arg(long, help = "Withdraw to Indodax username instead of blockchain")]
        username: bool,
        #[arg(long, help = "Memo/tag (for currencies that require it)")]
        memo: Option<String>,
        #[arg(long, help = "Blockchain network")]
        network: Option<String>,
        #[arg(long, help = "Callback URL for withdrawal confirmation")]
        callback_url: Option<String>,
    },

    #[command(name = "serve-callback", about = "Start a temporary HTTP server to handle Indodax withdrawal callback")]
    ServeCallback {
        #[arg(short, long, default_value = "8080")]
        port: u16,
        #[arg(short, long, help = "When true, auto-confirms all callback requests. When false, prompts for each request.", default_value = "false")]
        auto_ok: bool,
        #[arg(long, help = "Listen address (default: 127.0.0.1). Use 0.0.0.0 for network access")]
        listen: Option<String>,
    },
}

pub async fn execute(
    client: &IndodaxClient,
    config: &crate::config::IndodaxConfig,
    cmd: &FundingCommand,
    output_format: crate::output::OutputFormat,
) -> Result<CommandOutput> {
    match cmd {
        FundingCommand::WithdrawFee { currency, network } => {
            withdraw_fee(client, currency, network.as_deref()).await
        }
        FundingCommand::Withdraw { currency, amount, address, username, memo, network, callback_url } => {
            let cb_url = callback_url.as_deref().or(config.callback_url.as_deref());
            withdraw(client, currency, *amount, address, *username, memo.as_deref(), network.as_deref(), cb_url).await
        }
        FundingCommand::ServeCallback { port, auto_ok, listen } => {
            serve_callback(*port, *auto_ok, listen.as_deref(), output_format).await
        }
    }
}

async fn withdraw_fee(
    client: &IndodaxClient,
    currency: &str,
    network: Option<&str>,
) -> Result<CommandOutput> {
    let mut params = HashMap::new();
    params.insert("currency".into(), currency.to_string());
    if let Some(n) = network {
        params.insert("network".into(), n.to_string());
    }

    let data: serde_json::Value =
        client.private_post_v1("withdrawFee", &params).await?;

    let (headers, rows) = helpers::flatten_json_to_table(&data);
    Ok(CommandOutput::new(data, headers, rows))
}

async fn withdraw(
    client: &IndodaxClient,
    currency: &str,
    amount: f64,
    address: &str,
    to_username: bool,
    memo: Option<&str>,
    network: Option<&str>,
    callback_url: Option<&str>,
) -> Result<CommandOutput> {
    if currency.is_empty() {
        return Err(anyhow::anyhow!("Currency cannot be empty"));
    }
    if address.is_empty() {
        return Err(anyhow::anyhow!("Address cannot be empty"));
    }
    if amount <= 0.0 || !amount.is_finite() {
        return Err(anyhow::anyhow!(
            "Amount must be positive and finite, got {}",
            amount
        ));
    }

    let mut params = HashMap::new();
    params.insert("currency".into(), currency.to_string());
    params.insert("amount".into(), amount.to_string());

    if to_username {
        params.insert(
            "request_id".into(),
            chrono::Utc::now().timestamp_millis().to_string(),
        );
        params.insert("withdraw_to".into(), address.to_string());
    } else {
        params.insert("address".into(), address.to_string());
    }

    if let Some(m) = memo {
        params.insert("memo".into(), m.to_string());
    }
    if let Some(n) = network {
        params.insert("network".into(), n.to_string());
    }
    if let Some(u) = callback_url {
        params.insert("callback_url".into(), u.to_string());
    }

    let data: serde_json::Value =
        client.private_post_v1("withdrawCoin", &params).await?;

    let headers = vec!["Field".into(), "Value".into()];
    let mut rows: Vec<Vec<String>> = Vec::new();
    if let serde_json::Value::Object(ref map) = data {
        for (k, v) in map {
            rows.push(vec![k.clone(), helpers::value_to_string(v)]);
        }
    }

    let dest_label = if to_username {
        format!("user {}", address)
    } else {
        address.to_string()
    };

    Ok(CommandOutput::new(data, headers, rows)
        .with_addendum(format!("Withdrew {} {} to {}", amount, currency, dest_label)))
}

async fn serve_callback(
    port: u16,
    auto_ok: bool,
    listen: Option<&str>,
    output_format: crate::output::OutputFormat,
) -> Result<CommandOutput> {
    use axum::{routing::post, Router};
    use colored::Colorize;
    use std::net::SocketAddr;

    let app = Router::new().route(
        "/callback",
        post(move |body: String| async move {
            if output_format == crate::output::OutputFormat::Json {
                println!(
                    "{}",
                    serde_json::json!({
                        "event": "callback_received",
                        "body": body,
                        "auto_ok": auto_ok
                    })
                );
            } else {
                eprintln!("\n{} Incoming Callback Request", ">>>".green());
                eprintln!("{}: {}", "Body".bold(), body);
            }

            if auto_ok {
                if output_format == crate::output::OutputFormat::Json {
                    println!("{}", serde_json::json!({"event": "callback_response", "response": "ok"}));
                } else {
                    eprintln!("{} Sent response: {}", "<<<".blue(), "ok".bold());
                }
                "ok".to_string()
            } else {
                if output_format == crate::output::OutputFormat::Json {
                    eprintln!("{}", "Waiting for manual confirmation (check stderr)...".yellow());
                } else {
                    eprintln!("{} Waiting for manual confirmation...", "???".yellow());
                }
                eprintln!(
                    "{} Type 'ok' to confirm, or anything else to cancel:",
                    ">>>".green()
                );
                let input = tokio::task::spawn_blocking(|| {
                    let mut buf = String::new();
                    std::io::stdin().read_line(&mut buf).unwrap_or_default();
                    buf.trim().to_lowercase()
                })
                .await
                .unwrap_or_default();
                if input == "ok" {
                    if output_format == crate::output::OutputFormat::Json {
                        println!("{}", serde_json::json!({"event": "callback_response", "response": "ok"}));
                    } else {
                        eprintln!("{} Sent response: {}", "<<<".blue(), "ok".bold());
                    }
                    "ok".to_string()
                } else {
                    if output_format == crate::output::OutputFormat::Json {
                        println!("{}", serde_json::json!({"event": "callback_response", "response": "cancel"}));
                    } else {
                        eprintln!("{} Sent response: {}", "<<<".blue(), "cancel".bold());
                    }
                    "cancel".to_string()
                }
            }
        }),
    );

    let ip = listen.unwrap_or("127.0.0.1");
    let addr: SocketAddr = ip
        .parse()
        .map(|ip: std::net::IpAddr| SocketAddr::new(ip, port))
        .unwrap_or_else(|_| SocketAddr::from(([127, 0, 0, 1], port)));
    eprintln!("\n{}", "Indodax Callback Server".bold().underline());
    eprintln!("{}: {}", "Listening on".cyan(), addr);
    eprintln!(
        "{}: {}",
        "Auto-confirm".cyan(),
        if auto_ok {
            "ENABLED (returns 'ok')"
        } else {
            "DISABLED"
        }
    );
    eprintln!("{}\n", "Press Ctrl+C to stop".dimmed());

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(CommandOutput::new_empty())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_funding_command_variants() {
        let _cmd1 = FundingCommand::WithdrawFee { 
            currency: "btc".into(), 
            network: Some("BTC".into()) 
        };
        let _cmd2 = FundingCommand::Withdraw { 
            currency: "btc".into(), 
            amount: 0.5, 
            address: "addr123".into(), 
            username: false, 
            memo: None, 
            network: Some("BTC".into()), 
            callback_url: None 
        };
        let _cmd3 = FundingCommand::ServeCallback { 
            port: 8080, 
            auto_ok: true,
            listen: None,
        };
    }

    #[test]
    fn test_funding_command_withdraw_to_username() {
        let cmd = FundingCommand::Withdraw { 
            currency: "btc".into(), 
            amount: 0.5, 
            address: "user123".into(), 
            username: true, 
            memo: None, 
            network: None, 
            callback_url: None 
        };
        match cmd {
            FundingCommand::Withdraw { username, .. } => {
                assert!(username);
            }
            _ => assert!(false, "Expected Withdraw command, got {:?}", cmd),
        }
    }

    #[test]
    fn test_funding_command_serve_callback_defaults() {
        let cmd = FundingCommand::ServeCallback { 
            port: 8080, 
            auto_ok: true,
            listen: None,
        };
        match cmd {
            FundingCommand::ServeCallback { port, auto_ok, .. } => {
                assert_eq!(port, 8080);
                assert!(auto_ok);
            }
            _ => assert!(false, "Expected ServeCallback command, got {:?}", cmd),
        }
    }

    #[test]
    fn test_funding_command_withdraw_fee_no_network() {
        let cmd = FundingCommand::WithdrawFee { 
            currency: "eth".into(), 
            network: None 
        };
        match cmd {
            FundingCommand::WithdrawFee { network, .. } => {
                assert!(network.is_none());
            }
            _ => assert!(false, "Expected WithdrawFee command, got {:?}", cmd),
        }
    }

    #[test]
    fn test_funding_command_with_memo() {
        let cmd = FundingCommand::Withdraw { 
            currency: "xrp".into(), 
            amount: 100.0, 
            address: "rAddress".into(), 
            username: false, 
            memo: Some("123456".into()), 
            network: None, 
            callback_url: None 
        };
        match cmd {
            FundingCommand::Withdraw { memo, .. } => {
                assert_eq!(memo, Some("123456".into()));
            }
            _ => assert!(false, "Expected Withdraw command, got {:?}", cmd),
        }
    }
}
