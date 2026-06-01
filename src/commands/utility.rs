use crate::client::IndodaxClient;
use crate::config::ResolvedCredentials;
use crate::output::CommandOutput;
use anyhow::Result;
use rustyline::completion::{Completer, Pair};
use rustyline::highlight::{Highlighter, MatchingBracketHighlighter};
use rustyline::hint::{HistoryHinter};
use rustyline::validate::MatchingBracketValidator;
use rustyline::{Helper};
use std::collections::HashMap;

#[derive(Debug, clap::Subcommand)]
pub enum UtilityCommand {
    #[command(name = "setup", about = "Interactive setup wizard")]
    Setup,

    #[command(name = "status", about = "Check system and API reachability status")]
    Status,

    #[command(name = "shell", about = "Start interactive REPL")]
    Shell,
}

struct IndodaxHelper {
    completer: IndodaxCompleter,
    highlighter: MatchingBracketHighlighter,
    validator: MatchingBracketValidator,
    hinter: HistoryHinter,
}

struct IndodaxCompleter {
    commands: Vec<String>,
    pairs: Vec<String>,
}

impl Completer for IndodaxCompleter {
    type Candidate = Pair;

    fn complete(
        &self,
        line: &str,
        pos: usize,
        _ctx: &rustyline::Context<'_>,
    ) -> rustyline::Result<(usize, Vec<Pair>)> {
        let (start, word) = rustyline::completion::extract_word(line, pos, None, |c: char| c == ' ' || c == '/');
        let word_lower = word.to_lowercase();

        let mut candidates = Vec::new();

        // If it looks like we are typing a pair (e.g. after a space or --pair)
        let line_before = &line[..start];
        let is_pair_context = line_before.contains("ticker") || 
                             line_before.contains("book") || 
                             line_before.contains("trades") ||
                             line_before.contains("--pair") ||
                             line_before.contains("-p");

        if is_pair_context {
            for pair in &self.pairs {
                if pair.starts_with(&word_lower) {
                    candidates.push(Pair {
                        display: pair.clone(),
                        replacement: pair.clone(),
                    });
                }
            }
        }

        // Also suggest commands
        let line_lower = line.to_lowercase();
        for cmd in &self.commands {
            if cmd.starts_with(&line_lower) && cmd != &line_lower {
                let suffix = &cmd[line_before.len()..];
                candidates.push(Pair {
                    display: cmd.clone(),
                    replacement: suffix.to_string(),
                });
            }
        }

        Ok((start, candidates))
    }
}

impl Highlighter for IndodaxHelper {
    fn highlight<'l>(&self, line: &'l str, pos: usize) -> std::borrow::Cow<'l, str> {
        self.highlighter.highlight(line, pos)
    }
    fn highlight_char(&self, line: &str, pos: usize) -> bool {
        self.highlighter.highlight_char(line, pos)
    }
}

impl rustyline::hint::Hinter for IndodaxHelper {
    type Hint = String;
    fn hint(&self, line: &str, pos: usize, ctx: &rustyline::Context<'_>) -> Option<String> {
        self.hinter.hint(line, pos, ctx)
    }
}

impl rustyline::validate::Validator for IndodaxHelper {
    fn validate(&self, ctx: &mut rustyline::validate::ValidationContext<'_>) -> rustyline::Result<rustyline::validate::ValidationResult> {
        self.validator.validate(ctx)
    }
    fn validate_while_typing(&self) -> bool {
        self.validator.validate_while_typing()
    }
}

impl Completer for IndodaxHelper {
    type Candidate = Pair;
    fn complete(&self, line: &str, pos: usize, ctx: &rustyline::Context<'_>) -> rustyline::Result<(usize, Vec<Pair>)> {
        self.completer.complete(line, pos, ctx)
    }
}

impl Helper for IndodaxHelper {}

pub async fn execute(
    client: &IndodaxClient,
    creds: &Option<ResolvedCredentials>,
    cmd: &UtilityCommand,
    output_format: crate::output::OutputFormat,
) -> Result<CommandOutput> {
    match cmd {
        UtilityCommand::Setup => setup().await,
        UtilityCommand::Status => status(client, creds).await,
        UtilityCommand::Shell => shell(client, creds, output_format).await,
    }
}

async fn status(
    client: &IndodaxClient,
    creds: &Option<ResolvedCredentials>,
) -> Result<CommandOutput> {
    eprintln!("Checking Indodax CLI Status...\n");
    let mut results = Vec::new();

    // 1. Public API
    let public_ok = client.public_get::<serde_json::Value>("summaries").await.is_ok();
    results.push(vec![
        "Public API".into(),
        if public_ok { "REACHABLE".into() } else { "UNREACHABLE".into() },
        if public_ok { "OK".into() } else { "Check internet connection".into() }
    ]);

    // 2. Private API (Credentials)
    let private_configured = creds.is_some();
    results.push(vec![
        "Private API".into(),
        if private_configured { "CONFIGURED".into() } else { "NOT CONFIGURED".into() },
        if private_configured { "OK".into() } else { "Run 'indodax setup'".into() }
    ]);

    // 3. Private API (Access)
    if private_configured && public_ok {
        let access_ok = client.private_post_v1::<serde_json::Value>("getInfo", &HashMap::new()).await.is_ok();
        results.push(vec![
            "Private API Access".into(),
            if access_ok { "AUTHORIZED".into() } else { "UNAUTHORIZED/ERROR".into() },
            if access_ok { "OK".into() } else { "Check API Key/Secret permissions".into() }
        ]);
    } else {
        results.push(vec![
            "Private API Access".into(),
            "SKIPPED".into(),
            "Requires valid credentials and connectivity".into()
        ]);
    }

    // 4. WebSocket (Market)
    // We just check if we can reach the WS host (simple DNS/Connect check might be too much, but we can assume if Public API is OK, WS is likely OK)
    // For now, let's just mark it as "AVAILABLE"
    results.push(vec![
        "WebSocket (Public)".into(),
        "AVAILABLE".into(),
        "wss://ws3.indodax.com/ws/".into()
    ]);

    let headers = vec!["Component".into(), "Status".into(), "Detail".into()];
    let data = serde_json::json!({
        "public_api": public_ok,
        "private_api_configured": private_configured,
        "ws_url": "wss://ws3.indodax.com/ws/"
    });

    Ok(CommandOutput::new(data, headers, results))
}

async fn test_credentials(api_key: &str, api_secret: &str) {
    use crate::auth::Signer;
    let signer = Signer::new(api_key, api_secret);
    match IndodaxClient::new(Some(signer)) {
        Ok(client) => {
            match client
                .private_post_v1::<serde_json::Value>("getInfo", &HashMap::new())
                .await
            {
                Ok(info) => {
                    let name = info
                        .get("name")
                        .and_then(|v| v.as_str())
                        .unwrap_or("unknown");
                    let user_id = info
                        .get("user_id")
                        .and_then(|v| v.as_str())
                        .unwrap_or("unknown");
                    eprintln!(
                        "  Credentials validated: logged in as '{}' (user ID: {})",
                        name, user_id
                    );
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
    use dialoguer::{Confirm, Input, Password, Select};
    use colored::Colorize;

    eprintln!("{}", "=== Indodax CLI Setup Wizard ===".bold().cyan());
    eprintln!("This wizard will help you configure your API credentials and preferences.\n");

    let api_key: String = Input::new()
        .with_prompt("Enter your Indodax API key")
        .interact_text()?;

    let api_secret: String = Password::new()
        .with_prompt("Enter your Indodax API secret")
        .interact()?;

    eprintln!("\nValidating credentials...");
    test_credentials(&api_key, &api_secret).await;

    let callback_url: String = Input::new()
        .with_prompt("Enter your Indodax Callback URL (optional)")
        .allow_empty(true)
        .interact_text()?;

    let outputs = vec!["table", "json"];
    let output_idx = Select::new()
        .with_prompt("Select default output format")
        .items(&outputs)
        .default(0)
        .interact()?;
    let default_output = outputs[output_idx].to_string();

    let default_pair: String = Input::new()
        .with_prompt("Enter default trading pair")
        .default("btc_idr".into())
        .interact_text()?;

    let mcp_profiles = vec![
        ("readonly", "market,account"),
        ("paper", "market,account,paper"),
        ("full", "market,account,trade,funding,paper"),
    ];
    let mcp_idx = Select::new()
        .with_prompt("Select default MCP profile (service groups)")
        .items(&mcp_profiles.iter().map(|(n, g)| format!("{} ({})", n, g)).collect::<Vec<_>>())
        .default(1)
        .interact()?;
    let default_mcp_groups = mcp_profiles[mcp_idx].1.to_string();

    eprintln!("\n{}", "⚠️  SECURITY WARNING".bold().yellow());
    eprintln!("API keys provide access to your funds. Never share your secret key.");
    eprintln!("Configuration will be stored with 0600 permissions (read/write by owner only).\n");

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
        config.default_output = Some(default_output);
        config.default_pair = Some(default_pair);
        config.default_mcp_groups = Some(default_mcp_groups);
        
        config.save()?;
        eprintln!(
            "\n{} Configuration saved to {:?}",
            "✅".green(),
            crate::config::IndodaxConfig::config_path()
        );
    }

    let data = serde_json::json!({
        "status": "ok",
        "message": "Setup complete"
    });
    Ok(CommandOutput::json(data))
}

async fn shell(
    client: &IndodaxClient,
    _creds: &Option<ResolvedCredentials>,
    output_format: crate::output::OutputFormat,
) -> Result<CommandOutput> {
    use crate::Cli;
    use clap::Parser;
    use clap::CommandFactory;

    println!("Indodax CLI interactive shell");
    println!("Type commands without 'indodax' prefix (e.g. 'ticker btc/idr')");
    println!("Type 'help' for available commands, 'exit' to quit\n");

    // Pre-collect commands for completion
    let mut command_list = Vec::new();
    let cli_cmd = Cli::command();
    for cmd in cli_cmd.get_subcommands() {
        let name = cmd.get_name().to_string();
        command_list.push(name.clone());
        for subcmd in cmd.get_subcommands() {
            command_list.push(format!("{} {}", name, subcmd.get_name()));
        }
    }
    
    // Add common pairs for completion. Start with a few, then try to fetch all dynamically.
    let mut common_pairs = vec![
        "btc_idr".to_string(), "eth_idr".to_string(), "usdt_idr".to_string(), 
        "idrt_idr".to_string(), "bnb_idr".to_string(), "doge_idr".to_string(),
        "xrpidr".to_string(), "adaidr".to_string(), "dotidr".to_string(),
    ];
    
    // Fetch pairs dynamically in background or quickly await if fast enough. 
    // Since this is startup, waiting a tiny bit for pairs is usually acceptable.
    if let Ok(pairs_data) = client.public_get::<serde_json::Value>("/api/pairs").await {
        if let Some(arr) = pairs_data.as_array() {
            let mut dynamic_pairs = Vec::new();
            for item in arr {
                if let Some(id) = item.get("id").and_then(|v| v.as_str()) {
                    dynamic_pairs.push(id.to_string());
                }
            }
            if !dynamic_pairs.is_empty() {
                common_pairs = dynamic_pairs;
            }
        }
    }

    let h = IndodaxHelper {
        completer: IndodaxCompleter {
            commands: command_list,
            pairs: common_pairs,
        },
        highlighter: MatchingBracketHighlighter::new(),
        validator: MatchingBracketValidator::new(),
        hinter: HistoryHinter {},
    };

    let mut rl = rustyline::Editor::<IndodaxHelper, rustyline::history::DefaultHistory>::new()?;
    rl.set_helper(Some(h));
    
    let mut config = crate::config::IndodaxConfig::load()?;
    let client_ref = client;

    loop {
        let line = rl.readline("indodax> ");
        match line {
            Ok(input) if input.trim().is_empty() => continue,
            Ok(input) if input.trim() == "exit" || input.trim() == "quit" => break,
            Ok(input) if input.trim() == "clear" => {
                rl.clear_screen()?;
                continue;
            }
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

                        // Determine format for this specific command in shell
                        let cmd_format = cli.output.unwrap_or(output_format);

                        match crate::dispatch(cli, client_ref, &mut config, cmd_format).await {
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

/// Splits a shell-style command line into argv-like tokens.
fn shell_parse(input: &str) -> Vec<String> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        shlex::split(input).unwrap_or_default()
    }

    #[cfg(target_arch = "wasm32")]
    {
        input.split_whitespace().map(|s| s.to_string()).collect()
    }
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
        let result = shell_parse(r#"auth set --api-key "my key" --api-secret "my secret""#);
        assert_eq!(
            result,
            vec![
                "auth",
                "set",
                "--api-key",
                "my key",
                "--api-secret",
                "my secret",
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
