#[cfg(feature = "cli")]
use output::{CommandOutput, OutputFormat};

#[cfg(feature = "cli")]
pub mod alerts;
pub mod auth;
pub mod client;
#[cfg(feature = "cli")]
pub mod commands;
pub mod config;
pub mod errors;
pub mod integration;
#[cfg(feature = "mcp")]
pub mod mcp;
#[cfg(feature = "cli")]
pub mod output;

#[cfg(feature = "cli")]
use client::IndodaxClient;
#[cfg(feature = "cli")]
use errors::IndodaxError;

pub use integration::prelude;

pub(crate) fn now_millis() -> u64 {
    #[cfg(target_arch = "wasm32")]
    {
        js_sys::Date::now() as u64
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64
    }
}

#[cfg(feature = "cli")]
use clap::{Parser, Subcommand};

#[cfg(feature = "cli")]
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

    #[arg(
        short = 'o',
        long = "output",
        default_value = "table",
        help = "Output format: table or json",
        global = true
    )]
    pub output: OutputFormat,

    #[arg(
        long = "api-key",
        help = "API key (overrides config file and env var)",
        global = true
    )]
    pub api_key: Option<String>,

    #[arg(
        long = "api-secret",
        help = "API secret (overrides config file and env var)",
        global = true
    )]
    pub api_secret: Option<String>,

    #[arg(
        long = "api-secret-stdin",
        help = "Read API secret from stdin (more secure than --api-secret)",
        global = true
    )]
    pub api_secret_stdin: bool,

    #[arg(
        short = 'v',
        long = "verbose",
        help = "Enable verbose output",
        global = true
    )]
    pub verbose: bool,

    #[arg(
        long = "yes",
        alias = "force",
        help = "Skip confirmation prompts for destructive operations",
        global = true
    )]
    pub yes: bool,
}

#[cfg(feature = "cli")]
#[derive(Debug, Subcommand)]
pub enum Command {
    // === Legacy Hidden Commands for Backward Compatibility ===
    #[command(hide = true)]
    #[command(subcommand)]
    Market(commands::market::MarketCommand),
    #[command(hide = true)]
    #[command(subcommand)]
    Account(commands::account::AccountCommand),
    #[command(hide = true)]
    #[command(subcommand)]
    Trade(commands::trade::TradeCommand),
    #[command(hide = true)]
    #[command(subcommand)]
    Funding(commands::funding::FundingCommand),

    // === Flat Public Market Commands (originally nested under Market) ===
    /// Get server time
    ServerTime,

    /// List available trading pairs
    Pairs,

    /// Get ticker for a pair
    Ticker {
        #[arg(default_value = "btc_idr")]
        pair: String,
    },

    /// Get OHLCV history for a pair
    History {
        #[arg(default_value = "btc_idr")]
        pair: String,
        #[arg(short, long, default_value = "60")]
        timeframe: String,
        #[arg(short, long, help = "Start timestamp in seconds (default: 24h ago)")]
        from: Option<u64>,
        #[arg(long, help = "End timestamp in seconds (default: now)")]
        to: Option<u64>,
    },

    /// Get tickers for all pairs
    TickerAll,

    /// Get 24h and 7d summaries for all pairs
    Summaries,

    /// Get order book for a pair
    Orderbook {
        #[arg(default_value = "btc_idr")]
        pair: String,
        #[arg(long, default_value = "20", help = "Number of bid/ask levels to show")]
        count: usize,
    },

    /// Get recent trades for a pair
    Trades {
        #[arg(default_value = "btc_idr")]
        pair: String,
    },

    /// Get OHLCV candle data (default --from is 24h ago)
    Ohlc {
        #[arg(short, long, default_value = "btc_idr")]
        pair: String,
        #[arg(long, default_value = "60")]
        interval: String,
        #[arg(short, long, help = "Start timestamp in seconds (default: 24h ago)")]
        since: Option<u64>,
        #[arg(long, help = "End timestamp in seconds (default: now)")]
        to: Option<u64>,
    },

    /// Get market webdata for a pair
    Webdata {
        #[arg(default_value = "btc_idr")]
        pair: String,
    },

    /// Get chatroom history
    ChatHistory,

    /// Get detailed pairs info (V2)
    PairsV2 {
        #[arg(short, long)]
        pair: Option<String>,
    },

    /// Search markets (TradingView Search V2)
    SearchV2,

    /// Get terminal trading data
    TerminalTrade {
        #[arg(default_value = "btc_idr")]
        pair: String,
    },

    /// Get terminal market data
    TerminalMarket {
        #[arg(default_value = "btc_idr")]
        pair: String,
    },

    /// Get terminal market categories
    TerminalCategories,

    /// Get onramp config for a pair
    OnrampConfig {
        #[arg(default_value = "usdt_idr")]
        pair: String,
    },

    /// Get news for an asset
    News {
        #[arg(default_value = "btc")]
        asset: String,
        #[arg(short, long, default_value = "1")]
        page: u32,
    },

    /// Get price increments (tick sizes)
    PriceIncrements,

    // === Flat Private Account Commands (originally nested under Account) ===
    /// Get current account information (balances, permissions, etc.)
    AccountInfo,

    /// Get non-zero account balances
    Balance,

    /// Get your deposit/withdrawal transactions
    Transactions,

    /// Get your trade history for a specific symbol
    TradesHistory {
        /// Trading pair symbol (e.g., btc_idr)
        pair: String,

        /// Number of trades to return (default: 500)
        #[arg(short, long, default_value = "500")]
        limit: usize,

        /// Start from this trade ID (optional)
        #[arg(long)]
        from_id: Option<u64>,
    },

    // === Flat Trading Command (originally nested under Trade) ===
    /// Place and manage orders
    #[command(subcommand)]
    Order(commands::trade::TradeCommand),

    // === Flat Funding / Withdrawal Commands (originally nested under Funding) ===
    /// Withdraw cryptocurrency
    Withdraw {
        #[arg(short, long)]
        asset: String,
        #[arg(short = 'v', long, help = "Amount to withdraw")]
        volume: f64,
        #[arg(
            long,
            help = "Crypto destination address (or Indodax username if --username is set)"
        )]
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

    /// Manage withdrawal fees and servers
    #[command(subcommand)]
    Withdrawal(WithdrawalSubcommand),

    // === Flat WebSocket streaming ===
    /// WebSocket streaming
    #[command(subcommand)]
    Ws(commands::websocket::WebSocketCommand),

    // === Flat Paper Trading ===
    /// Paper trading (simulated)
    #[command(subcommand)]
    Paper(commands::paper::PaperCommand),

    // === Flat API Credentials ===
    /// Manage API credentials
    #[command(subcommand)]
    Auth(commands::auth::AuthCommand),

    // === Flat Price Alert Management ===
    /// Price alert management
    #[command(subcommand)]
    Alert(commands::alert::AlertCommand),

    // === Direct Tools ===
    /// Interactive setup wizard
    Setup,

    /// Start interactive REPL
    Shell,

    /// Start MCP stdio server for AI agent integration
    #[cfg(feature = "mcp")]
    Mcp {
        #[arg(
            short = 's',
            long = "groups",
            default_value = "market,account,paper,auth",
            help = "Comma-separated service groups: market, account, trade, funding, paper, auth"
        )]
        groups: String,
        #[arg(
            long,
            help = "Allow dangerous operations (trade, funding) without acknowledged flag"
        )]
        allow_dangerous: bool,
        #[arg(long, default_value = "8000", help = "Port for HTTP server")]
        port: u16,
        #[arg(long, help = "Start as HTTP server instead of stdio")]
        http: bool,
    },
}

#[cfg(feature = "cli")]
#[derive(Debug, Subcommand)]
pub enum WithdrawalSubcommand {
    /// Check withdrawal fee for a currency
    Fee {
        #[arg(short, long)]
        asset: String,
        #[arg(short, long, help = "Blockchain network (optional)")]
        network: Option<String>,
    },

    /// Start a temporary HTTP server to handle Indodax withdrawal callback
    #[cfg(feature = "server")]
    ServeCallback {
        #[arg(short, long, default_value = "8080")]
        port: u16,
        #[arg(
            short,
            long,
            help = "When true, auto-confirms all callback requests. When false, prompts for each request.",
            default_value = "false"
        )]
        auto_ok: bool,
        #[arg(
            long,
            help = "Listen address (default: 127.0.0.1). Use 0.0.0.0 for network access"
        )]
        listen: Option<String>,
    },
}

#[cfg(feature = "cli")]
pub async fn dispatch(
    cli: Cli,
    client: &IndodaxClient,
    config: &mut config::IndodaxConfig,
) -> Result<CommandOutput, IndodaxError> {
    let output = match cli.command {
        // === Legacy Hidden Commands ===
        Command::Market(ref cmd) => commands::market::execute(client, cmd)
            .await
            .map_err(map_anyhow_error)?,
        Command::Account(ref cmd) => commands::account::execute(client, cmd)
            .await
            .map_err(map_anyhow_error)?,
        Command::Trade(ref cmd) => commands::trade::execute(client, cmd, cli.yes)
            .await
            .map_err(map_anyhow_error)?,
        Command::Funding(ref cmd) => commands::funding::execute(client, config, cmd, cli.output)
            .await
            .map_err(map_anyhow_error)?,

        // === Public Market Commands ===
        Command::ServerTime => {
            commands::market::execute(client, &commands::market::MarketCommand::ServerTime)
                .await
                .map_err(map_anyhow_error)?
        }
        Command::Pairs => {
            commands::market::execute(client, &commands::market::MarketCommand::Pairs)
                .await
                .map_err(map_anyhow_error)?
        }
        Command::Ticker { pair } => {
            commands::market::execute(client, &commands::market::MarketCommand::Ticker { pair })
                .await
                .map_err(map_anyhow_error)?
        }
        Command::History {
            pair,
            timeframe,
            from,
            to,
        } => commands::market::execute(
            client,
            &commands::market::MarketCommand::Ohlc {
                symbol: pair,
                timeframe,
                from,
                to,
            },
        )
        .await
        .map_err(map_anyhow_error)?,
        Command::TickerAll => {
            commands::market::execute(client, &commands::market::MarketCommand::TickerAll)
                .await
                .map_err(map_anyhow_error)?
        }
        Command::Summaries => {
            commands::market::execute(client, &commands::market::MarketCommand::Summaries)
                .await
                .map_err(map_anyhow_error)?
        }
        Command::Orderbook { pair, count } => commands::market::execute(
            client,
            &commands::market::MarketCommand::Orderbook {
                pair,
                levels: count,
            },
        )
        .await
        .map_err(map_anyhow_error)?,
        Command::Trades { pair } => {
            commands::market::execute(client, &commands::market::MarketCommand::Trades { pair })
                .await
                .map_err(map_anyhow_error)?
        }
        Command::Ohlc {
            pair,
            interval,
            since,
            to,
        } => commands::market::execute(
            client,
            &commands::market::MarketCommand::Ohlc {
                symbol: pair,
                timeframe: interval,
                from: since,
                to,
            },
        )
        .await
        .map_err(map_anyhow_error)?,
        Command::Webdata { pair } => {
            commands::market::execute(client, &commands::market::MarketCommand::WebData { pair })
                .await
                .map_err(map_anyhow_error)?
        }
        Command::ChatHistory => {
            commands::market::execute(client, &commands::market::MarketCommand::ChatHistory)
                .await
                .map_err(map_anyhow_error)?
        }
        Command::PairsV2 { pair } => {
            commands::market::execute(client, &commands::market::MarketCommand::PairsV2 { pair })
                .await
                .map_err(map_anyhow_error)?
        }
        Command::SearchV2 => {
            commands::market::execute(client, &commands::market::MarketCommand::SearchV2)
                .await
                .map_err(map_anyhow_error)?
        }
        Command::TerminalTrade { pair } => commands::market::execute(
            client,
            &commands::market::MarketCommand::TerminalTrade { pair },
        )
        .await
        .map_err(map_anyhow_error)?,
        Command::TerminalMarket { pair } => commands::market::execute(
            client,
            &commands::market::MarketCommand::TerminalMarket { pair },
        )
        .await
        .map_err(map_anyhow_error)?,
        Command::TerminalCategories => {
            commands::market::execute(client, &commands::market::MarketCommand::TerminalCategories)
                .await
                .map_err(map_anyhow_error)?
        }
        Command::OnrampConfig { pair } => commands::market::execute(
            client,
            &commands::market::MarketCommand::OnrampConfig { pair },
        )
        .await
        .map_err(map_anyhow_error)?,
        Command::News { asset, page } => commands::market::execute(
            client,
            &commands::market::MarketCommand::News { asset, page },
        )
        .await
        .map_err(map_anyhow_error)?,
        Command::PriceIncrements => {
            commands::market::execute(client, &commands::market::MarketCommand::PriceIncrements)
                .await
                .map_err(map_anyhow_error)?
        }

        // === Account & Balances Commands ===
        Command::AccountInfo => {
            commands::account::execute(client, &commands::account::AccountCommand::Info)
                .await
                .map_err(map_anyhow_error)?
        }
        Command::Balance => {
            commands::account::execute(client, &commands::account::AccountCommand::Balance)
                .await
                .map_err(map_anyhow_error)?
        }
        Command::Transactions => {
            commands::account::execute(client, &commands::account::AccountCommand::TransHistory)
                .await
                .map_err(map_anyhow_error)?
        }
        Command::TradesHistory {
            pair,
            limit,
            from_id: _,
        } => commands::account::execute(
            client,
            &commands::account::AccountCommand::TradeHistory {
                symbol: pair,
                limit: limit as u32,
            },
        )
        .await
        .map_err(map_anyhow_error)?,

        // === Order Execution ===
        Command::Order(ref cmd) => commands::trade::execute(client, cmd, cli.yes)
            .await
            .map_err(map_anyhow_error)?,

        // === Funding / Withdrawal Operations ===
        Command::Withdraw {
            asset,
            volume,
            address,
            username,
            memo,
            network,
            callback_url,
        } => {
            let funding_cmd = commands::funding::FundingCommand::Withdraw {
                currency: asset,
                amount: volume,
                address,
                username,
                memo,
                network,
                callback_url,
            };
            commands::funding::execute(client, config, &funding_cmd, cli.output)
                .await
                .map_err(map_anyhow_error)?
        }
        Command::Withdrawal(ref sub) => {
            let funding_cmd = match sub {
                WithdrawalSubcommand::Fee { asset, network } => {
                    commands::funding::FundingCommand::WithdrawFee {
                        currency: asset.clone(),
                        network: network.clone(),
                    }
                }
                #[cfg(feature = "server")]
                WithdrawalSubcommand::ServeCallback {
                    port,
                    auto_ok,
                    listen,
                } => commands::funding::FundingCommand::ServeCallback {
                    port: *port,
                    auto_ok: *auto_ok,
                    listen: listen.clone(),
                },
            };
            commands::funding::execute(client, config, &funding_cmd, cli.output)
                .await
                .map_err(map_anyhow_error)?
        }

        // === WS, Paper, Auth, Alert ===
        Command::Ws(ref cmd) => commands::websocket::execute(client, cmd, cli.output)
            .await
            .map_err(map_anyhow_error)?,
        Command::Paper(ref cmd) => commands::paper::execute(client, config, cmd)
            .await
            .map_err(map_anyhow_error)?,
        Command::Auth(ref cmd) => commands::auth::execute(client, config, cmd)
            .await
            .map_err(map_anyhow_error)?,
        Command::Alert(ref cmd) => commands::alert::execute(client, &None, cmd)
            .await
            .map_err(map_anyhow_error)?,

        Command::Setup | Command::Shell => {
            return Err(IndodaxError::Other(
                "This command is handled separately".into(),
            ));
        }
        #[cfg(feature = "mcp")]
        Command::Mcp { .. } => {
            return Err(IndodaxError::Other(
                "This command is handled separately".into(),
            ));
        }
    };

    Ok(output.with_format(cli.output))
}

#[cfg(feature = "cli")]
pub fn map_anyhow_error(e: anyhow::Error) -> IndodaxError {
    e.downcast::<IndodaxError>()
        .unwrap_or_else(|e| IndodaxError::Other(e.to_string()))
}

#[cfg(all(test, feature = "cli"))]
mod tests {
    use super::*;

    #[test]
    fn test_cli_parse_ticker() {
        let args = vec!["indodax", "ticker", "btc_idr"];
        let cli = Cli::try_parse_from(args).unwrap();
        match cli.command {
            Command::Ticker { pair: _ } => {
                // Just verify it parsed
            }
            _ => panic!("Expected Ticker command, got {:?}", cli.command),
        }
    }

    #[test]
    fn test_cli_parse_output_json() {
        let args = vec!["indodax", "-o", "json", "ticker"];
        let cli = Cli::try_parse_from(args).unwrap();
        assert_eq!(cli.output, OutputFormat::Json);
    }

    #[test]
    fn test_cli_parse_api_key() {
        let args = vec!["indodax", "--api-key", "mykey", "ticker"];
        let cli = Cli::try_parse_from(args).unwrap();
        assert_eq!(cli.api_key, Some("mykey".into()));
    }

    #[test]
    fn test_cli_parse_api_secret() {
        let args = vec!["indodax", "--api-secret", "mysecret", "ticker"];
        let cli = Cli::try_parse_from(args).unwrap();
        assert_eq!(cli.api_secret, Some("mysecret".into()));
    }

    #[test]
    fn test_cli_parse_verbose() {
        let args = vec!["indodax", "-v", "ticker"];
        let cli = Cli::try_parse_from(args).unwrap();
        assert!(cli.verbose);
    }

    #[test]
    fn test_command_variants() {
        let _cmd1 = Command::ServerTime;
        let _cmd2 = Command::AccountInfo;
        let _cmd3 = Command::Order(crate::commands::trade::TradeCommand::Buy {
            pair: "btc_idr".into(),
            idr: 100_000.0,
            price: None,
            order_type: None,
        });
        let _cmd4 = Command::Withdrawal(WithdrawalSubcommand::Fee {
            asset: "btc".into(),
            network: None,
        });
        let _cmd5 = Command::Ws(crate::commands::websocket::WebSocketCommand::Ticker {
            pair: "btc_idr".into(),
        });
        let _cmd6 = Command::Paper(crate::commands::paper::PaperCommand::Balance);
        let _cmd7 = Command::Auth(crate::commands::auth::AuthCommand::Show);
        let _cmd8 = Command::Setup;
        let _cmd9 = Command::Shell;
        #[cfg(feature = "mcp")]
        let _cmd10 = Command::Mcp {
            groups: "market,paper".into(),
            allow_dangerous: false,
        };
    }

    #[test]
    fn test_output_format_clap() {
        let args = vec!["indodax", "-o", "table", "ticker"];
        let cli = Cli::try_parse_from(args).unwrap();
        assert_eq!(cli.output, OutputFormat::Table);
    }

    #[test]
    fn test_cli_parse_default_output() {
        let args = vec!["indodax", "ticker"];
        let cli = Cli::try_parse_from(args).unwrap();
        assert_eq!(cli.output, OutputFormat::Table);
    }

    #[test]
    fn test_command_display() {
        let cli = Cli::try_parse_from(vec!["indodax", "ticker"]).unwrap();
        let _ = format!("{:?}", cli);
    }
}
