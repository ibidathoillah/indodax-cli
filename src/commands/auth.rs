use crate::client::IndodaxClient;
use crate::commands::helpers;
use crate::config::{IndodaxConfig, SecretValue};
use crate::output::CommandOutput;
use anyhow::Result;

#[derive(Debug, clap::Subcommand)]
pub enum AuthCommand {
    #[command(name = "set", about = "Set API key and secret")]
    Set {
        #[arg(short = 'k', long = "api-key", help = "Your Indodax API key")]
        api_key: String,
        #[arg(short = 's', long = "api-secret", help = "Your Indodax API secret")]
        api_secret: Option<String>,
        #[arg(long = "api-secret-stdin", help = "Read API secret from stdin")]
        api_secret_stdin: bool,
    },

    #[command(name = "show", about = "Show current API configuration")]
    Show,

    #[command(name = "test", about = "Test API credentials")]
    Test,

    #[command(name = "reset", about = "Remove stored API credentials")]
    Reset,
}

pub async fn execute(
    _client: &IndodaxClient,
    config: &mut IndodaxConfig,
    cmd: &AuthCommand,
) -> Result<CommandOutput> {
    match cmd {
        AuthCommand::Set { api_key, api_secret, api_secret_stdin } => {
            let secret = if *api_secret_stdin {
                let mut buf = String::new();
                std::io::stdin().read_line(&mut buf)?;
                buf.trim().to_string()
            } else if let Some(s) = api_secret {
                s.clone()
            } else {
                return Err(anyhow::anyhow!(
                    "API secret is required. Use --api-secret or --api-secret-stdin"
                ));
            };

            config.api_key = Some(SecretValue::new(api_key));
            config.api_secret = Some(SecretValue::new(secret));
            config.save()?;

            let data = serde_json::json!({
                "status": "ok",
                "message": "API credentials saved"
            });
            Ok(CommandOutput::json(data))
        }

        AuthCommand::Show => {
            let key_status = config
                .api_key
                .as_ref()
                .map_or("not set", |_| "set");
            let secret_status = config
                .api_secret
                .as_ref()
                .map_or("not set", |_| "set");
            let config_path = IndodaxConfig::config_path();

            let headers = vec!["Field".into(), "Value".into()];
            let rows = vec![
                vec!["Config path".into(), config_path.display().to_string()],
                vec!["API Key".into(), key_status.into()],
                vec!["API Secret".into(), secret_status.into()],
            ];

            let data = serde_json::json!({
                "config_path": config_path.to_string_lossy(),
                "api_key_set": config.api_key.is_some(),
                "api_secret_set": config.api_secret.is_some(),
            });

            Ok(CommandOutput::new(data, headers, rows))
        }

        AuthCommand::Test => {
            if config.api_key.is_none() || config.api_secret.is_none() {
                return Err(anyhow::anyhow!(
                    "No API credentials configured. Use 'indodax auth set' first."
                ));
            }

            let signer = crate::auth::Signer::new(
                config.api_key.as_ref().unwrap().as_str(),
                config.api_secret.as_ref().unwrap().as_str(),
            );
            let test_client = IndodaxClient::new(Some(signer));
            let test_params = std::collections::HashMap::new();
            let result: serde_json::Value = test_client.private_post_v1("getInfo", &test_params).await?;

            let balance = &result["balance"];
            let bal_summary = if balance.is_object() {
                balance
                    .as_object()
                    .map(|obj| {
                        obj.iter()
                            .filter(|(_, v)| v.as_f64().unwrap_or(0.0) > 0.0)
                            .map(|(k, v)| format!("{}: {}", k, v))
                            .collect::<Vec<_>>()
                            .join(", ")
                    })
                    .unwrap_or_default()
            } else {
                "N/A".into()
            };

            let headers = vec!["Field".into(), "Value".into()];
            let rows = vec![
                vec!["Status".into(), "OK - Credentials valid".into()],
                vec!["Name".into(), helpers::value_to_string(&result.get("name").unwrap_or(&serde_json::Value::Null))],
                vec!["Server Time".into(), helpers::value_to_string(&result.get("server_time").unwrap_or(&serde_json::Value::Null))],
                vec!["Balances (non-zero)".into(), bal_summary],
            ];

            Ok(CommandOutput::new(result, headers, rows))
        }

        AuthCommand::Reset => {
            config.api_key = None;
            config.api_secret = None;
            config.save()?;

            let data = serde_json::json!({
                "status": "ok",
                "message": "API credentials removed"
            });
            Ok(CommandOutput::json(data))
        }
    }
}
