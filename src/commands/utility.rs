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

    let api_key: String = loop {
        let key: String = Input::new()
            .with_prompt("Enter your Indodax API key")
            .interact_text()?;
        let trimmed = key.trim().to_string();
        if trimmed.len() < 10 || trimmed.contains(' ') {
            println!("API key should be at least 10 characters and contain no spaces. Please try again.");
            continue;
        }
        break trimmed;
    };

    let api_secret: String = Password::new()
        .with_prompt("Enter your Indodax API secret")
        .interact()?;

    let callback_url: String = Input::new()
        .with_prompt("Enter your Indodax Callback URL (optional, e.g., https://indodax.tep2.in/)")
        .allow_empty(true)
        .interact_text()?;

    let save: bool = Confirm::new()
        .with_prompt("Save configuration to config?")
        .default(true)
        .interact()?;

    if save {
        let mut config = crate::config::IndodaxConfig::load()?;
        config.api_key = Some(crate::config::SecretValue::new(&api_key));
        config.api_secret = Some(crate::config::SecretValue::new(&api_secret));
        if !callback_url.is_empty() {
            config.callback_url = Some(callback_url);
        }
        config.save()?;
        println!("\nConfiguration saved to {:?}", crate::config::IndodaxConfig::config_path());
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
    let client = crate::client::IndodaxClient::new(signer)?;

    loop {
        let line = rl.readline("indodax> ");
        match line {
            Ok(input) if input.trim().is_empty() => continue,
            Ok(input) if input.trim() == "exit" || input.trim() == "quit" => break,
            Ok(input) => {
                let _ = rl.add_history_entry(&input);
                let args = format!("indodax {}", input);
                let args: Vec<String> = shell_parse(&args);
                match Cli::try_parse_from(args.iter().map(|s| s.as_str())) {
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
    }

    let data = serde_json::json!({"status": "exited"});
    Ok(CommandOutput::json(data))
}

fn shell_parse(input: &str) -> Vec<String> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Vec::new();
    }
    shlex::split(trimmed).unwrap_or_else(|| {
        trimmed.split_whitespace().map(|s| s.to_string()).collect()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shell_parse_simple() {
        let input = "market ticker btc_idr";
        let result = shell_parse(input);
        assert_eq!(result, vec!["market", "ticker", "btc_idr"]);
    }

    #[test]
    fn test_shell_parse_single_word() {
        let input = "help";
        let result = shell_parse(input);
        assert_eq!(result, vec!["help"]);
    }

    #[test]
    fn test_shell_parse_empty() {
        let input = "";
        let result = shell_parse(input);
        assert!(result.is_empty());
    }

    #[test]
    fn test_shell_parse_with_quotes() {
        let input = r#"auth set --api-key "my key" --api-secret "my secret""#;
        let result = shell_parse(input);
        assert!(result.contains(&"auth".to_string()));
        assert!(result.contains(&"my key".to_string()));
        assert!(result.contains(&"my secret".to_string()));
    }

    #[test]
    fn test_shell_parse_multiple_spaces() {
        let input = "market   ticker   btc_idr";
        let result = shell_parse(input);
        assert_eq!(result, vec!["market", "ticker", "btc_idr"]);
    }

    #[test]
    fn test_shell_parse_leading_trailing_spaces() {
        let input = "  market ticker btc_idr  ";
        let result = shell_parse(input);
        assert_eq!(result, vec!["market", "ticker", "btc_idr"]);
    }

    #[test]
    fn test_utility_command_variants() {
        let _cmd1 = UtilityCommand::Setup;
        let _cmd2 = UtilityCommand::Shell;
    }

    #[test]
    fn test_shell_parse_whitespace_fallback() {
        let input = "simple";
        let result = shell_parse(input);
        assert_eq!(result.len(), 1);
    }

    #[test]
    fn test_shell_parse_with_dash_args() {
        let input = "account balance -v";
        let result = shell_parse(input);
        assert_eq!(result, vec!["account", "balance", "-v"]);
    }
}
