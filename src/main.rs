use clap::Parser;
use indodax_cli::{
    client::IndodaxClient,
    commands::utility::{UtilityCommand, execute as utility_execute},
    config::IndodaxConfig,
    dispatch, map_anyhow_error, Cli, Command,
};
use indodax_cli::errors::IndodaxError;
use indodax_cli::mcp;
use indodax_cli::output::{CommandOutput, OutputFormat};
use std::process;

#[tokio::main]
async fn main() {
    // Custom panic hook for cleaner error output
    std::panic::set_hook(Box::new(|info| {
        let message = if let Some(s) = info.payload().downcast_ref::<&str>() {
            s.to_string()
        } else if let Some(s) = info.payload().downcast_ref::<String>() {
            s.clone()
        } else {
            "unexpected internal error".to_string()
        };
        let location = info.location().map(|l| format!(" at {}:{}", l.file(), l.line())).unwrap_or_default();
        eprintln!("Internal error: {}{}", message, location);
        std::process::exit(1);
    }));

    // Initialize tracing (logs to stderr, never stdout)
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "indodax_cli=info".into()),
        )
        .with_target(true)
        .init();

    let cli = Cli::parse();

    // Capture output format before cli is consumed by dispatch
    let output_format = cli.output;

    let mut config = match IndodaxConfig::load() {
        Ok(c) => c,
        Err(e) => {
            report_error(&IndodaxError::Other(e.to_string()), output_format);
            process::exit(1);
        }
    };

    let creds = match config.resolve_credentials(cli.api_key.clone(), cli.api_secret.clone()) {
        Ok(c) => c,
        Err(e) => {
            report_error(&IndodaxError::Other(e.to_string()), output_format);
            process::exit(1);
        }
    };

    let signer = creds.as_ref().map(|c| {
        indodax_cli::auth::Signer::new(c.api_key.as_str(), c.api_secret.as_str())
    });

    let client = IndodaxClient::new(signer);

    // Handle MCP server separately — it runs indefinitely on stdio
    if let Command::Mcp { groups, allow_dangerous } = &cli.command {
        match mcp::run(groups, *allow_dangerous, client, config).await {
            Ok(()) => process::exit(0),
            Err(e) => {
                report_error(&e, output_format);
                process::exit(1);
            }
        }
    }

    let result: Result<CommandOutput, IndodaxError> = match &cli.command {
        Command::Setup => {
            utility_execute(&client, &creds, &UtilityCommand::Setup).await
                .map_err(map_anyhow_error)
        }
        Command::Shell => {
            utility_execute(&client, &creds, &UtilityCommand::Shell).await
                .map_err(map_anyhow_error)
        }
        _ => dispatch(cli, &client, &mut config).await,
    };

    match result {
        Ok(output) => {
            println!("{}", output.render());
        }
        Err(e) => {
            report_error(&e, output_format);
            process::exit(1);
        }
    }
}

/// Report an error in the appropriate format.
///
/// In JSON mode, prints a parseable error envelope to stdout.
/// In table mode, prints human-readable error to stderr.
fn report_error(err: &IndodaxError, format: OutputFormat) {
    if format == OutputFormat::Json {
        let envelope = serde_json::json!({
            "success": false,
            "data": null,
            "error": true,
            "message": err.to_string(),
            "error_type": err.category(),
            "retryable": err.is_retryable(),
        });
        match serde_json::to_string_pretty(&envelope) {
            Ok(s) => println!("{}", s),
            Err(_) => eprintln!("Error: {}", err),
        }
    } else {
        eprintln!("Error: {}", err);
    }
}
