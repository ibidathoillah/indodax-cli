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
}

pub async fn execute(
    client: &IndodaxClient,
    cmd: &FundingCommand,
) -> Result<CommandOutput> {
    match cmd {
        FundingCommand::WithdrawFee { currency, network } => {
            withdraw_fee(client, currency, network.as_deref()).await
        }
        FundingCommand::Withdraw { currency, amount, address, username, memo, network, callback_url } => {
            withdraw(client, currency, *amount, address, *username, memo.as_deref(), network.as_deref(), callback_url.as_deref()).await
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
