use clap::Parser;
use indodax_cli::{
    client::IndodaxClient,
    commands::utility::{UtilityCommand, execute as utility_execute},
    config::IndodaxConfig,
    dispatch, Cli, Command,
};
use indodax_cli::errors::IndodaxError;
use indodax_cli::mcp;
use indodax_cli::output::OutputFormat;
use std::process;

#[tokio::main]
async fn main() {
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
            report_error(&e, output_format);
            process::exit(1);
        }
    };

    let creds = match config.resolve_credentials(cli.api_key.clone(), cli.api_secret.clone()) {
        Ok(c) => c,
        Err(e) => {
            report_error(&e, output_format);
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

    let result = match &cli.command {
        Command::Setup => {
            utility_execute(&client, &creds, &UtilityCommand::Setup).await
        }
        Command::Shell => {
            utility_execute(&client, &creds, &UtilityCommand::Shell).await
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
fn report_error(err: &anyhow::Error, format: OutputFormat) {
    if format == OutputFormat::Json {
        let (error_type, retryable) = err
            .downcast_ref::<IndodaxError>()
            .map(|ie| (ie.category().to_string(), ie.is_retryable()))
            .unwrap_or(("unknown_error".to_string(), false));

        let envelope = serde_json::json!({
            "error": true,
            "message": err.to_string(),
            "error_type": error_type,
            "retryable": retryable,
        });
        println!("{}", serde_json::to_string_pretty(&envelope).unwrap());
    } else {
        eprintln!("Error: {}", err);
        if err.chain().count() > 1 {
            for cause in err.chain().skip(1) {
                eprintln!("  caused by: {}", cause);
            }
        }
    }
}
