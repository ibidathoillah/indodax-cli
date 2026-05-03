use clap::Parser;
use indodax_cli::{
    client::IndodaxClient,
    commands::utility::{UtilityCommand, execute as utility_execute},
    config::IndodaxConfig,
    dispatch, Cli, Command,
};
use std::process;

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    let mut config = match IndodaxConfig::load() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Error loading config: {}", e);
            process::exit(1);
        }
    };

    let creds = match config.resolve_credentials(cli.api_key.clone(), cli.api_secret.clone()) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Error resolving credentials: {}", e);
            process::exit(1);
        }
    };

    let signer = creds.as_ref().map(|c| {
        indodax_cli::auth::Signer::new(c.api_key.as_str(), c.api_secret.as_str())
    });

    let client = IndodaxClient::new(signer);

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
            eprintln!("Error: {}", e);
            if e.chain().count() > 1 {
                for cause in e.chain().skip(1) {
                    eprintln!("  caused by: {}", cause);
                }
            }
            process::exit(1);
        }
    }
}
