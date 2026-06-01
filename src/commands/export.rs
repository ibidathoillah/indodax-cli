use crate::client::IndodaxClient;
use crate::output::CommandOutput;
use anyhow::Result;
use std::collections::HashMap;

#[derive(Debug, clap::Subcommand)]
pub enum ExportCommand {
    #[command(name = "transactions", about = "Export deposit/withdrawal history to CSV")]
    Transactions {
        #[arg(short, long, default_value = "csv")]
        format: String,
        #[arg(short, long, help = "Output file path")]
        output: Option<String>,
    },
    #[command(name = "trades", about = "Export trade history to CSV")]
    Trades {
        #[arg(default_value = "btc_idr")]
        pair: String,
        #[arg(short, long, default_value = "csv")]
        format: String,
        #[arg(short, long, help = "Output file path")]
        output: Option<String>,
    },
}

pub async fn execute(client: &IndodaxClient, cmd: &ExportCommand) -> Result<CommandOutput> {
    match cmd {
        ExportCommand::Transactions { format, output } => {
            export_transactions(client, format, output.as_deref()).await
        }
        ExportCommand::Trades { pair, format, output } => {
            export_trades(client, pair, format, output.as_deref()).await
        }
    }
}

async fn export_transactions(client: &IndodaxClient, format: &str, output: Option<&str>) -> Result<CommandOutput> {
    let data: serde_json::Value = client.private_post_v1("transHistory", &HashMap::new()).await?;
    
    if format == "csv" {
        let mut csv = String::new();
        csv.push_str("status,type,asset,amount,fee,timestamp\n");
        
        if let Some(success) = data["success"].as_array() {
            for item in success {
                csv.push_str(&format!("{},{},{},{},{},{}\n",
                    item["status"], item["type"], item["currency"], item["amount"], item["fee"], item["timestamp"]));
            }
        }
        
        if let Some(path) = output {
            std::fs::write(path, &csv)?;
            return Ok(CommandOutput::json(serde_json::json!({"status": "ok", "path": path}))
                .with_addendum(format!("Transactions exported to {}", path)));
        } else {
            println!("{}", csv);
            return Ok(CommandOutput::json(serde_json::json!({"status": "ok", "target": "stdout"}))
                .with_addendum("Transactions exported to stdout"));
        }
    }
    
    Ok(CommandOutput::json(data))
}

async fn export_trades(client: &IndodaxClient, pair: &str, format: &str, output: Option<&str>) -> Result<CommandOutput> {
    let mut params = HashMap::new();
    params.insert("pair".into(), pair.to_string());
    params.insert("count".into(), "1000".into());
    
    let data: serde_json::Value = client.private_post_v1("tradeHistory", &params).await?;
    
    if format == "csv" {
        let mut csv = String::new();
        csv.push_str("id,type,price,amount,total,fee,timestamp\n");
        
        if let Some(trades) = data["return"]["trades"].as_array() {
            for t in trades {
                csv.push_str(&format!("{},{},{},{},{},{},{}\n",
                    t["trade_id"], t["type"], t["price"], t["amount"], t["total"], t["fee"], t["timestamp"]));
            }
        }
        
        if let Some(path) = output {
            std::fs::write(path, &csv)?;
            return Ok(CommandOutput::json(serde_json::json!({"status": "ok", "path": path}))
                .with_addendum(format!("Trades exported to {}", path)));
        } else {
            println!("{}", csv);
            return Ok(CommandOutput::json(serde_json::json!({"status": "ok", "target": "stdout"}))
                .with_addendum("Trades exported to stdout"));
        }
    }
    
    Ok(CommandOutput::json(data))
}
