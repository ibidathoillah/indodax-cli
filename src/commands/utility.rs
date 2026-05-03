use crate::client::IndodaxClient;
use crate::config::ResolvedCredentials;
use crate::output::CommandOutput;
use anyhow::Result;

#[derive(Debug, clap::Subcommand)]
pub enum UtilityCommand {
    #[command(name = "setup", about = "Interactive setup wizard")]
    Setup,

    #[command(name = "shell", about = "Start interactive REPL")]
    Shell,
}

pub async fn execute(
    _client: &IndodaxClient,
    _creds: &Option<ResolvedCredentials>,
    cmd: &UtilityCommand,
) -> Result<CommandOutput> {
    match cmd {
        UtilityCommand::Setup => setup().await,
        UtilityCommand::Shell => shell().await,
    }
}

async fn setup() -> Result<CommandOutput> {
    use dialoguer::{Confirm, Input, Password};

    println!("=== Indodax CLI Setup Wizard ===\n");

    let api_key: String = Input::new()
        .with_prompt("Enter your Indodax API key")
        .interact_text()?;

    let api_secret: String = Password::new()
        .with_prompt("Enter your Indodax API secret")
        .interact()?;

    let save: bool = Confirm::new()
        .with_prompt("Save credentials to config?")
        .default(true)
        .interact()?;

    if save {
        let mut config = crate::config::IndodaxConfig::load()?;
        config.api_key = Some(crate::config::SecretValue::new(&api_key));
        config.api_secret = Some(crate::config::SecretValue::new(&api_secret));
        config.save()?;
        println!("\nCredentials saved to {:?}", crate::config::IndodaxConfig::config_path());
    }

    let data = serde_json::json!({
        "status": "ok",
        "message": "Setup complete"
    });
    Ok(CommandOutput::json(data))
}

async fn shell() -> Result<CommandOutput> {
    use crate::Cli;
    use clap::Parser;
    use rustyline::DefaultEditor;

    println!("Indodax CLI interactive shell");
    println!("Type commands without 'indodax' prefix (e.g. 'market ticker btc_idr')");
    println!("Type 'help' for available commands, 'exit' to quit\n");

    let mut rl = DefaultEditor::new()?;
    let mut config = crate::config::IndodaxConfig::load()?;
    let creds = config.resolve_credentials(None, None)?;
    let signer = creds.as_ref().map(|c| {
        crate::auth::Signer::new(c.api_key.as_str(), c.api_secret.as_str())
    });
    let client = crate::client::IndodaxClient::new(signer);

    loop {
        let line = rl.readline("indodax> ");
        match line {
            Ok(input) if input.trim().is_empty() => continue,
            Ok(input) if input.trim() == "exit" || input.trim() == "quit" => break,
            Ok(input) => {
                let _ = rl.add_history_entry(&input);
                let args = format!("indodax {}", input);
                let args: Vec<&str> = shell_parse(&args);
                match Cli::try_parse_from(args) {
                    Ok(cli) => {
                        if matches!(cli.command, crate::Command::Shell) {
                            println!("Already in shell mode");
                            continue;
                        }
                        if matches!(cli.command, crate::Command::Setup) {
                            println!("Setup is only available from the command line, not inside the shell");
                            continue;
                        }
                        match crate::dispatch(cli, &client, &mut config).await {
                            Ok(output) => println!("{}", output.render()),
                            Err(e) => {
                                eprintln!("Error: {}", e);
                            }
                        }
                    }
                    Err(e) => eprintln!("{}", e.render()),
                }
            }
            Err(_) => break,
        }
        let _ = rl.add_history_entry("");
    }

    let data = serde_json::json!({"status": "exited"});
    Ok(CommandOutput::json(data))
}

fn shell_parse(input: &str) -> Vec<&str> {
    let mut parts = Vec::new();
    let mut current = "";
    let mut in_quote = false;

    for word in input.split(' ') {
        if in_quote {
            if word.ends_with('"') {
                in_quote = false;
                parts.push(&input[current.len() + 1..current.len() + 1 + parts.last().map(|s: &&str| s.len()).unwrap_or(0) + word.len() - 1]);
            }
        } else if word.starts_with('"') {
            in_quote = true;
            current = word;
        } else {
            parts.push(word);
        }
    }

    if parts.is_empty() {
        input.split_whitespace().collect()
    } else {
        parts
    }
}
