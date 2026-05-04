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
        #[arg(long, help = "Destination address (or Indodax username)")]
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
        #[arg(short, long, help = "Auto-confirm all requests (returns 'ok')", default_value = "true")]
        auto_ok: bool,
    },
}

pub async fn execute(
    client: &IndodaxClient,
    config: &crate::config::IndodaxConfig,
    cmd: &FundingCommand,
) -> Result<CommandOutput> {
    match cmd {
        FundingCommand::WithdrawFee { currency, network } => {
            withdraw_fee(client, currency, network.as_deref()).await
        }
        FundingCommand::Withdraw { currency, amount, address, username, memo, network, callback_url } => {
            let cb_url = callback_url.as_deref().or(config.callback_url.as_deref());
            withdraw(client, currency, *amount, address, *username, memo.as_deref(), network.as_deref(), cb_url).await
        }
        FundingCommand::ServeCallback { port, auto_ok } => {
            serve_callback(*port, *auto_ok).await
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
    let mut params = HashMap::new();
    params.insert("currency".into(), currency.to_string());
    params.insert("amount".into(), amount.to_string());

    if to_username {
        params.insert("request_id".into(), "1".to_string());
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

async fn serve_callback(port: u16, auto_ok: bool) -> Result<CommandOutput> {
    use axum::{routing::post, Router};
    use colored::Colorize;
    use std::net::SocketAddr;

    let app = Router::new().route(
        "/callback",
        post(move |body: String| async move {
            println!("\n{} Incoming Callback Request", ">>>".green());
            println!("{}: {}", "Body".bold(), body);

            if auto_ok {
                println!("{} Sent response: {}", "<<<".blue(), "ok".bold());
                "ok"
            } else {
                println!("{} Waiting for manual confirmation...", "???".yellow());
                "cancel" // Default if not auto
            }
        }),
    );

    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    println!("\n{}", "Indodax Callback Server".bold().underline());
    println!("{}: {}", "Listening on".cyan(), addr);
    println!("{}: {}", "Auto-confirm".cyan(), if auto_ok { "ENABLED (returns 'ok')" } else { "DISABLED" });
    println!("{}\n", "Press Ctrl+C to stop".dimmed());

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
            auto_ok: true 
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
            _ => panic!("Expected Withdraw command"),
        }
    }

    #[test]
    fn test_funding_command_serve_callback_defaults() {
        let cmd = FundingCommand::ServeCallback { 
            port: 8080, 
            auto_ok: true 
        };
        match cmd {
            FundingCommand::ServeCallback { port, auto_ok } => {
                assert_eq!(port, 8080);
                assert!(auto_ok);
            }
            _ => panic!("Expected ServeCallback command"),
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
            _ => panic!("Expected WithdrawFee command"),
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
            _ => panic!("Expected Withdraw command"),
        }
    }
}
