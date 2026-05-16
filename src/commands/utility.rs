use std::collections::HashMap;
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

async fn test_credentials(api_key: &str, api_secret: &str) {
    use crate::auth::Signer;
    let signer = Signer::new(api_key, api_secret);
    match IndodaxClient::new(Some(signer)) {
        Ok(client) => {
            match client.private_post_v1::<serde_json::Value>("getInfo", &HashMap::new()).await {
                Ok(info) => {
                    let name = info.get("name").and_then(|v| v.as_str()).unwrap_or("unknown");
                    let user_id = info.get("user_id").and_then(|v| v.as_str()).unwrap_or("unknown");
                    eprintln!("  Credentials validated: logged in as '{}' (user ID: {})", name, user_id);
                }
                Err(e) => {
                    eprintln!("  Warning: Credentials saved but validation failed: {}", e);
                    eprintln!("  Check that your API key and secret are correct.");
                }
            }
        }
        Err(e) => {
            eprintln!("  Warning: Could not create client for validation: {}", e);
        }
    }
}

async fn setup() -> Result<CommandOutput> {
    use dialoguer::{Confirm, Input, Password};

    eprintln!("=== Indodax CLI Setup Wizard ===\n");

    let api_key: String = Input::new()
        .with_prompt("Enter your Indodax API key")
        .interact_text()?;

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
        eprintln!("\nConfiguration saved to {:?}", crate::config::IndodaxConfig::config_path());
    }

    eprintln!("\nValidating credentials...");
    test_credentials(&api_key, &api_secret).await;

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
    let client_ref = &client;

    loop {
        let line = rl.readline("indodax> ");
        match line {
            Ok(input) if input.trim().is_empty() => continue,
            Ok(input) if input.trim() == "exit" || input.trim() == "quit" => break,
            Ok(input) => {
                let _ = rl.add_history_entry(&input);
                let args = format!("indodax {}", input);
                let args: Vec<String> = shell_parse(&args);
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
                        match crate::dispatch(cli, client_ref, &mut config).await {
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

/// Splits a shell-style command line into argv-like tokens using shlex.
fn shell_parse(input: &str) -> Vec<String> {
    shlex::split(input).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shell_parse_simple() {
        let result = shell_parse("market ticker btc_idr");
        assert_eq!(result, vec!["market", "ticker", "btc_idr"]);
    }

    #[test]
    fn test_shell_parse_single_word() {
        let result = shell_parse("help");
        assert_eq!(result, vec!["help"]);
    }

    #[test]
    fn test_shell_parse_empty() {
        let result = shell_parse("");
        assert!(result.is_empty());
    }

    #[test]
    fn test_shell_parse_with_quotes() {
        let result =
            shell_parse(r#"auth set --api-key "my key" --api-secret "my secret""#);
        assert_eq!(
            result,
            vec![
                "auth", "set", "--api-key", "my key", "--api-secret", "my secret",
            ]
        );
    }

    #[test]
    fn test_shell_parse_quoted_value_with_dash() {
        let result = shell_parse(r#"market ticker --pair "btc_idr""#);
        assert_eq!(result, vec!["market", "ticker", "--pair", "btc_idr"]);
    }

    #[test]
    fn test_shell_parse_multiple_spaces() {
        let result = shell_parse("market   ticker   btc_idr");
        assert_eq!(result, vec!["market", "ticker", "btc_idr"]);
    }

    #[test]
    fn test_shell_parse_leading_trailing_spaces() {
        let result = shell_parse("  market ticker btc_idr  ");
        assert_eq!(result, vec!["market", "ticker", "btc_idr"]);
    }

    #[test]
    fn test_shell_parse_only_whitespace() {
        let result = shell_parse("    ");
        assert!(result.is_empty());
    }

    #[test]
    fn test_shell_parse_quoted_empty_string() {
        let result = shell_parse(r#"set key """#);
        assert_eq!(result, vec!["set", "key", ""]);
    }

    #[test]
    fn test_shell_parse_quoted_whitespace_only() {
        let result = shell_parse(r#"echo "   ""#);
        assert_eq!(result, vec!["echo", "   "]);
    }

    #[test]
    fn test_shell_parse_escaped_quote_inside_quotes() {
        let result = shell_parse(r#"echo "he said \"hi\"""#);
        assert_eq!(result, vec!["echo", r#"he said "hi""#]);
    }

    #[test]
    fn test_shell_parse_escaped_backslash_inside_quotes() {
        let result = shell_parse(r#"path "a\\b""#);
        assert_eq!(result, vec!["path", r#"a\b"#]);
    }

    #[test]
    fn test_shell_parse_unclosed_quote_returns_empty() {
        let result = shell_parse(r#"foo "bar baz"#);
        // shlex returns None on parse error (unclosed quotes), unwrap_or_default gives empty vec
        assert!(result.is_empty());
    }

    #[test]
    fn test_shell_parse_adjacent_quoted_and_bare() {
        let result = shell_parse(r#"x="hello world""#);
        assert_eq!(result, vec!["x=hello world"]);
    }

    #[test]
    fn test_shell_parse_tab_separator() {
        let result = shell_parse("a\tb\tc");
        assert_eq!(result, vec!["a", "b", "c"]);
    }

    #[test]
    fn test_utility_command_variants() {
        let _cmd1 = UtilityCommand::Setup;
        let _cmd2 = UtilityCommand::Shell;
    }

    #[test]
    fn test_shell_parse_with_dash_args() {
        let result = shell_parse("account balance -v");
        assert_eq!(result, vec!["account", "balance", "-v"]);
    }
}
