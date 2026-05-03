use clap::{Parser, Subcommand};
use output::{CommandOutput, OutputFormat};

pub mod auth;
pub mod client;
pub mod commands;
pub mod config;
pub mod errors;
pub mod output;
pub mod telemetry;

use client::IndodaxClient;

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

    #[command(name = "setup", about = "Interactive setup wizard")]
    Setup,

    #[command(name = "shell", about = "Start interactive REPL")]
    Shell,
}

pub async fn dispatch(
    cli: Cli,
    client: &IndodaxClient,
    config: &mut config::IndodaxConfig,
) -> Result<CommandOutput, anyhow::Error> {
    let output = match cli.command {
        Command::Market { cmd } => commands::market::execute(client, &cmd).await?,
        Command::Account { cmd } => commands::account::execute(client, &cmd).await?,
        Command::Trade { cmd } => commands::trade::execute(client, &cmd).await?,
        Command::Funding { cmd } => commands::funding::execute(client, config, &cmd).await?,
        Command::Ws { cmd } => commands::websocket::execute(client, &cmd).await?,
        Command::Paper { cmd } => commands::paper::execute(config, &cmd).await?,
        Command::Auth { cmd } => commands::auth::execute(client, config, &cmd).await?,
        Command::Setup | Command::Shell => {
            return Err(anyhow::anyhow!("This command is handled separately"));
        }
    };

    Ok(output.with_format(cli.output))
}
