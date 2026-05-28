use clap::Parser;
use indodax_cli::errors::IndodaxError;
use indodax_cli::mcp;
use indodax_cli::output::{CommandOutput, OutputFormat};
use indodax_cli::{
    client::IndodaxClient,
    commands::utility::{execute as utility_execute, UtilityCommand},
    config::IndodaxConfig,
    dispatch, map_anyhow_error, Cli, Command, Language,
};
use std::io::BufRead;
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
        let location = info
            .location()
            .map(|l| format!(" at {}:{}", l.file(), l.line()))
            .unwrap_or_default();
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
    let language = cli.lang;

    let mut config = match IndodaxConfig::load() {
        Ok(c) => c,
        Err(e) => {
            report_error(&IndodaxError::Other(e.to_string()), output_format, language);
            process::exit(1);
        }
    };

    // Handle --api-secret-stdin: read secret from stdin (more secure than CLI args)
    let api_secret = if cli.api_secret_stdin {
        let mut secret = String::new();
        if let Err(e) = std::io::stdin().lock().read_line(&mut secret) {
            report_error(
                &IndodaxError::Other(format!("Failed to read API secret from stdin: {}", e)),
                output_format,
                language,
            );
            process::exit(1);
        }
        let trimmed = secret.trim().to_string();
        if trimmed.is_empty() {
            eprintln!("Warning: --api-secret-stdin used but no input received (empty line).");
            cli.api_secret.clone()
        } else {
            Some(trimmed)
        }
    } else {
        cli.api_secret.clone()
    };

    let creds = match config.resolve_credentials(cli.api_key.clone(), api_secret) {
        Ok(c) => c,
        Err(e) => {
            report_error(&IndodaxError::Other(e.to_string()), output_format, language);
            process::exit(1);
        }
    };

    let signer = creds
        .as_ref()
        .map(|c| indodax_cli::auth::Signer::new(c.api_key.as_str(), c.api_secret.as_str()));

    let client = match IndodaxClient::new(signer) {
        Ok(c) => c.with_ws_token(config.ws_token.as_ref().map(|t| t.as_str().to_string())),
        Err(e) => {
            report_error(&e, output_format, language);
            process::exit(1);
        }
    };

    // Handle MCP server separately — it runs indefinitely on stdio or http
    if let Command::Mcp {
        groups,
        allow_dangerous,
        port,
        http,
    } = &cli.command
    {
        #[cfg(feature = "mcp")]
        {
            if *http {
                #[cfg(feature = "server")]
                {
                    match mcp::run_http(*port, groups, *allow_dangerous).await {
                        Ok(()) => process::exit(0),
                        Err(e) => {
                            report_error(&e, output_format, language);
                            process::exit(1);
                        }
                    }
                }
                #[cfg(not(feature = "server"))]
                {
                    report_error(
                        &IndodaxError::Other(
                            "HTTP server feature not enabled. Rebuild with --features server"
                                .into(),
                        ),
                        output_format,
                        language,
                    );
                    process::exit(1);
                }
            } else {
                match mcp::run(groups, *allow_dangerous, client, config).await {
                    Ok(()) => process::exit(0),
                    Err(e) => {
                        report_error(&e, output_format, language);
                        process::exit(1);
                    }
                }
            }
        }
        #[cfg(not(feature = "mcp"))]
        {
            let _ = (groups, allow_dangerous, port, http);
            report_error(
                &IndodaxError::Other("MCP feature not enabled".into()),
                output_format,
                language,
            );
            process::exit(1);
        }
    }

    let result: Result<CommandOutput, IndodaxError> = match &cli.command {
        Command::Setup => utility_execute(&client, &creds, &UtilityCommand::Setup)
            .await
            .map_err(map_anyhow_error),
        Command::Shell => utility_execute(&client, &creds, &UtilityCommand::Shell)
            .await
            .map_err(map_anyhow_error),
        _ => dispatch(cli, &client, &mut config).await,
    };

    match result {
        Ok(output) => {
            if !output.suppress_final_output || output_format == OutputFormat::Table {
                println!("{}", output.render());
            }
        }
        Err(e) => {
            report_error(&e, output_format, language);
            process::exit(1);
        }
    }
}

/// Report an error in the appropriate format.
///
/// In JSON mode, prints a parseable error envelope to stdout.
/// In table mode, prints human-readable error to stderr.
fn report_error(err: &IndodaxError, format: OutputFormat, language: Language) {
    let message = localized_error_message(err, language);
    if format == OutputFormat::Json {
        let envelope = serde_json::json!({
            "success": false,
            "data": null,
            "error": true,
            "message": message,
            "error_type": err.category(),
            "retryable": err.is_retryable(),
            "language": match language { Language::En => "en", Language::Id => "id" },
        });
        match serde_json::to_string_pretty(&envelope) {
            Ok(s) => println!("{}", s),
            Err(_) => eprintln!("Error: {}", message),
        }
    } else {
        let label = match language {
            Language::En => "Error",
            Language::Id => "Kesalahan",
        };
        eprintln!("{}: {}", label, message);
    }
}

fn localized_error_message(err: &IndodaxError, language: Language) -> String {
    if language == Language::En {
        return err.to_string();
    }

    match err.category().as_str() {
        "authentication" => format!("Autentikasi gagal: {}", err),
        "authorization" => format!("Akses ditolak: {}", err),
        "validation" => format!("Parameter tidak valid: {}", err),
        "not_found" => format!("Data tidak ditemukan: {}", err),
        "rate_limit" => format!("Batas permintaan tercapai, coba lagi nanti: {}", err),
        "server" => format!("Server Indodax bermasalah sementara: {}", err),
        "connection" => format!("Koneksi gagal: {}", err),
        "config" => format!("Konfigurasi belum lengkap: {}", err),
        _ => format!("Terjadi kesalahan: {}", err),
    }
}
