use clap::{Parser, Subcommand};
use output::{CommandOutput, OutputFormat};

pub mod auth;
pub mod client;
pub mod commands;
pub mod config;
pub mod errors;
pub mod mcp;
pub mod output;

use client::IndodaxClient;
use errors::IndodaxError;

#[derive(Debug, Parser)]
#[command(
    name = "indodax",
    version,
    about = "Command-line interface for the Indodax cryptocurrency exchange",
    long_about = None
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,

    #[arg(short = 'o', long = "output", default_value = "table", help = "Output format: table or json")]
    pub output: OutputFormat,

    #[arg(long = "api-key", help = "API key (overrides config file and env var)")]
    pub api_key: Option<String>,

    #[arg(long = "api-secret", help = "API secret (overrides config file and env var)")]
    pub api_secret: Option<String>,

    #[arg(short = 'v', long = "verbose", help = "Enable verbose output")]
    pub verbose: bool,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    #[command(name = "market", about = "Public market data")]
    Market {
        #[command(subcommand)]
        cmd: commands::market::MarketCommand,
    },

    #[command(name = "account", about = "Account information and balances")]
    Account {
        #[command(subcommand)]
        cmd: commands::account::AccountCommand,
    },

    #[command(name = "trade", about = "Place and manage orders")]
    Trade {
        #[command(subcommand)]
        cmd: commands::trade::TradeCommand,
    },

    #[command(name = "funding", about = "Deposit and withdrawal operations")]
    Funding {
        #[command(subcommand)]
        cmd: commands::funding::FundingCommand,
    },

    #[command(name = "ws", about = "WebSocket streaming")]
    Ws {
        #[command(subcommand)]
        cmd: commands::websocket::WebSocketCommand,
    },

    #[command(name = "paper", about = "Paper trading (simulated)")]
    Paper {
        #[command(subcommand)]
        cmd: commands::paper::PaperCommand,
    },

    #[command(name = "auth", about = "Manage API credentials")]
    Auth {
        #[command(subcommand)]
        cmd: commands::auth::AuthCommand,
    },

    #[command(name = "alert", about = "Price alert management")]
    Alert {
        #[command(subcommand)]
        cmd: commands::alert::AlertCommand,
    },

    #[command(name = "setup", about = "Interactive setup wizard")]
    Setup,

    #[command(name = "shell", about = "Start interactive REPL")]
    Shell,

    #[command(name = "mcp", about = "Start MCP stdio server for AI agent integration")]
    Mcp {
        #[arg(short = 's', long = "groups", default_value = "market,account,paper,auth", help = "Comma-separated service groups: market, account, trade, funding, paper, auth")]
        groups: String,
        #[arg(long, help = "Allow dangerous operations (trade, funding) without acknowledged flag")]
        allow_dangerous: bool,
    },
}

pub async fn dispatch(
    cli: Cli,
    client: &IndodaxClient,
    config: &mut config::IndodaxConfig,
) -> Result<CommandOutput, IndodaxError> {
    let output = match cli.command {
        Command::Market { cmd } => commands::market::execute(client, &cmd).await
            .map_err(map_anyhow_error)?,
        Command::Account { cmd } => commands::account::execute(client, &cmd).await
            .map_err(map_anyhow_error)?,
        Command::Trade { cmd } => commands::trade::execute(client, &cmd).await
            .map_err(map_anyhow_error)?,
        Command::Funding { cmd } => commands::funding::execute(client, config, &cmd, cli.output).await
            .map_err(map_anyhow_error)?,
        Command::Ws { cmd } => commands::websocket::execute(client, &cmd, cli.output).await
            .map_err(map_anyhow_error)?,
        Command::Paper { cmd } => commands::paper::execute(client, config, &cmd).await?,
        Command::Auth { cmd } => commands::auth::execute(client, config, &cmd).await
            .map_err(map_anyhow_error)?,
        Command::Alert { cmd } => commands::alert::execute(client, &None, &cmd).await
            .map_err(map_anyhow_error)?,
        Command::Setup | Command::Shell | Command::Mcp { .. } => {
            return Err(IndodaxError::Other("This command is handled separately".into()));
        }
    };

    Ok(output.with_format(cli.output))
}

pub fn map_anyhow_error(e: anyhow::Error) -> IndodaxError {
    e.downcast::<IndodaxError>()
        .unwrap_or_else(|e| IndodaxError::Other(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cli_parse_market_ticker() {
        let args = vec!["indodax", "market", "ticker", "btc_idr"];
        let cli = Cli::try_parse_from(args).unwrap();
        match cli.command {
            Command::Market { cmd: _ } => {
                // Just verify it parsed
            }
            _ => assert!(false, "Expected Market command, got {:?}", cli.command),
        }
    }

    #[test]
    fn test_cli_parse_output_json() {
        let args = vec!["indodax", "-o", "json", "market", "ticker"];
        let cli = Cli::try_parse_from(args).unwrap();
        assert_eq!(cli.output, OutputFormat::Json);
    }

    #[test]
    fn test_cli_parse_api_key() {
        let args = vec!["indodax", "--api-key", "mykey", "market", "ticker"];
        let cli = Cli::try_parse_from(args).unwrap();
        assert_eq!(cli.api_key, Some("mykey".into()));
    }

    #[test]
    fn test_cli_parse_api_secret() {
        let args = vec!["indodax", "--api-secret", "mysecret", "market", "ticker"];
        let cli = Cli::try_parse_from(args).unwrap();
        assert_eq!(cli.api_secret, Some("mysecret".into()));
    }

    #[test]
    fn test_cli_parse_verbose() {
        let args = vec!["indodax", "-v", "market", "ticker"];
        let cli = Cli::try_parse_from(args).unwrap();
        assert!(cli.verbose);
    }

    #[test]
    fn test_command_variants() {
        let _cmd1 = Command::Market { cmd: crate::commands::market::MarketCommand::ServerTime };
        let _cmd2 = Command::Account { cmd: crate::commands::account::AccountCommand::Info };
        let _cmd3 = Command::Trade { cmd: crate::commands::trade::TradeCommand::Buy { 
            pair: "btc_idr".into(), 
            idr: 100_000.0, 
            price: None 
        }};
        let _cmd4 = Command::Funding { cmd: crate::commands::funding::FundingCommand::WithdrawFee { 
            currency: "btc".into(), 
            network: None 
        }};
        let _cmd5 = Command::Ws { cmd: crate::commands::websocket::WebSocketCommand::Ticker { 
            pair: "btc_idr".into() 
        }};
        let _cmd6 = Command::Paper { cmd: crate::commands::paper::PaperCommand::Balance };
        let _cmd7 = Command::Auth { cmd: crate::commands::auth::AuthCommand::Show };
        let _cmd8 = Command::Setup;
        let _cmd9 = Command::Shell;
        let _cmd10 = Command::Mcp { groups: "market,paper".into(), allow_dangerous: false };
    }

    #[test]
    fn test_output_format_clap() {
        // Test that OutputFormat works with clap
        let args = vec!["indodax", "-o", "table", "market", "ticker"];
        let cli = Cli::try_parse_from(args).unwrap();
        assert_eq!(cli.output, OutputFormat::Table);
    }

    #[test]
    fn test_cli_parse_default_output() {
        let args = vec!["indodax", "market", "ticker"];
        let cli = Cli::try_parse_from(args).unwrap();
        assert_eq!(cli.output, OutputFormat::Table);
    }

    #[test]
    fn test_command_display() {
        let cli = Cli::try_parse_from(vec!["indodax", "market", "ticker"]).unwrap();
        // Just verify the struct can be created
        let _ = format!("{:?}", cli);
    }
}
