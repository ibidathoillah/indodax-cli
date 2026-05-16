use crate::client::IndodaxClient;
use crate::commands::helpers;
use crate::config::{IndodaxConfig, SecretValue};
use crate::output::CommandOutput;
use anyhow::Result;

#[derive(Debug, clap::Subcommand)]
pub enum AuthCommand {
    #[command(name = "set", about = "Set API key, secret and callback URL")]
    Set {
        #[arg(short = 'k', long = "api-key", help = "Your Indodax API key")]
        api_key: Option<String>,
        #[arg(short = 's', long = "api-secret", help = "Your Indodax API secret")]
        api_secret: Option<String>,
        #[arg(long = "api-secret-stdin", help = "Read API secret from stdin")]
        api_secret_stdin: bool,
        #[arg(long = "callback-url", help = "Your Indodax Callback URL")]
        callback_url: Option<String>,
    },

    #[command(name = "show", about = "Show current API configuration")]
    Show,

    #[command(name = "test", about = "Test API credentials")]
    Test,

    #[command(name = "reset", about = "Remove stored API credentials")]
    Reset,
}

pub async fn execute(
    client: &IndodaxClient,
    config: &mut IndodaxConfig,
    cmd: &AuthCommand,
) -> Result<CommandOutput> {
    match cmd {
        AuthCommand::Set { api_key, api_secret, api_secret_stdin, callback_url } => {
            if let Some(key) = api_key {
                config.api_key = Some(SecretValue::new(key));
            }

            if *api_secret_stdin {
                let mut buf = String::new();
                let mut stdin = tokio::io::BufReader::new(tokio::io::stdin());
                use tokio::io::AsyncBufReadExt;
                stdin.read_line(&mut buf).await?;
                config.api_secret = Some(SecretValue::new(buf.trim().to_string()));
            } else if let Some(s) = api_secret {
                config.api_secret = Some(SecretValue::new(s.clone()));
            }

            if let Some(url) = callback_url {
                config.callback_url = Some(url.clone());
            }

            config.save()?;

            let data = serde_json::json!({
                "status": "ok",
                "message": "API configuration updated"
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
            let callback_url = config
                .callback_url
                .as_deref()
                .unwrap_or("not set");
            let config_path = IndodaxConfig::config_path();

            let headers = vec!["Field".into(), "Value".into()];
            let rows = vec![
                vec!["Config path".into(), config_path.display().to_string()],
                vec!["API Key".into(), key_status.into()],
                vec!["API Secret".into(), secret_status.into()],
                vec!["Callback URL".into(), callback_url.into()],
            ];

            let masked_key = config.api_key.as_ref().map(|k| {
                let s = k.as_str();
                let visible_len = (s.len() / 4).min(4);
                if visible_len > 0 {
                    format!("{}****", &s[..visible_len])
                } else {
                    "****".to_string()
                }
            });

            let data = serde_json::json!({
                "config_path": config_path.to_string_lossy(),
                "api_key_set": config.api_key.is_some(),
                "api_secret_set": config.api_secret.is_some(),
                "masked_key": masked_key,
                "callback_url": config.callback_url,
            });

            Ok(CommandOutput::new(data, headers, rows))
        }

        AuthCommand::Test => {
            let test_params = std::collections::HashMap::new();
            let result: serde_json::Value = client.private_post_v1("getInfo", &test_params).await?;

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
                vec!["Name".into(), helpers::value_to_string(result.get("name").unwrap_or(&serde_json::Value::Null))],
                vec!["Server Time".into(), helpers::value_to_string(result.get("server_time").unwrap_or(&serde_json::Value::Null))],
                vec!["Balances (non-zero)".into(), bal_summary],
            ];

            Ok(CommandOutput::new(result, headers, rows))
        }

        AuthCommand::Reset => {
            config.api_key = None;
            config.api_secret = None;
            config.callback_url = None;
            config.save()?;

            let data = serde_json::json!({
                "status": "ok",
                "message": "API credentials removed"
            });
                Ok(CommandOutput::json(data))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_auth_command_set() {
        let cmd = AuthCommand::Set {
            api_key: Some("key123".into()),
            api_secret: Some("secret456".into()),
            api_secret_stdin: false,
            callback_url: Some("http://callback.test".into()),
        };
        match cmd {
            AuthCommand::Set { api_key, api_secret, api_secret_stdin, callback_url } => {
                assert_eq!(api_key, Some("key123".into()));
                assert_eq!(api_secret, Some("secret456".into()));
                assert!(!api_secret_stdin);
                assert_eq!(callback_url, Some("http://callback.test".into()));
            }
            _ => assert!(false, "Expected Set command, got {:?}", cmd),
        }
    }

    #[test]
    fn test_auth_command_show() {
        let cmd = AuthCommand::Show;
        match cmd {
            AuthCommand::Show => (),
            _ => assert!(false, "Expected Show command, got {:?}", cmd),
        }
    }

    #[test]
    fn test_auth_command_test() {
        let cmd = AuthCommand::Test;
        match cmd {
            AuthCommand::Test => (),
            _ => assert!(false, "Expected Test command, got {:?}", cmd),
        }
    }

    #[test]
    fn test_auth_command_reset() {
        let cmd = AuthCommand::Reset;
        match cmd {
            AuthCommand::Reset => (),
            _ => assert!(false, "Expected Reset command, got {:?}", cmd),
        }
    }

    #[test]
    fn test_auth_command_set_minimal() {
        let cmd = AuthCommand::Set {
            api_key: None,
            api_secret: None,
            api_secret_stdin: true,
            callback_url: None,
        };
        match cmd {
            AuthCommand::Set { api_key, api_secret, api_secret_stdin, callback_url } => {
                assert!(api_key.is_none());
                assert!(api_secret.is_none());
                assert!(api_secret_stdin);
                assert!(callback_url.is_none());
            }
            _ => assert!(false, "Expected Set command, got {:?}", cmd),
        }
    }

    #[test]
    fn test_auth_command_variants() {
        let _cmd1 = AuthCommand::Set { 
            api_key: None, 
            api_secret: None, 
            api_secret_stdin: false, 
            callback_url: None 
        };
        let _cmd2 = AuthCommand::Show;
        let _cmd3 = AuthCommand::Test;
        let _cmd4 = AuthCommand::Reset;
    }
}
