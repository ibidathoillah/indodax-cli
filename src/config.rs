use serde::{Deserialize, Serialize};
use std::fmt;
use std::fs;
use std::path::PathBuf;

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SecretValue(String);

impl fmt::Debug for SecretValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.0.is_empty() {
            f.debug_struct("SecretValue").field("value", &"").finish()
        } else {
            f.debug_struct("SecretValue")
                .field("value", &"****")
                .finish()
        }
    }
}

impl SecretValue {
    pub fn new(s: impl Into<String>) -> Self {
        Self(s.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl fmt::Display for SecretValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.0.is_empty() {
            write!(f, "")
        } else {
            write!(f, "********")
        }
    }
}

impl From<String> for SecretValue {
    fn from(s: String) -> Self {
        SecretValue(s)
    }
}

impl From<&str> for SecretValue {
    fn from(s: &str) -> Self {
        SecretValue(s.to_string())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct IndodaxConfig {
    pub api_key: Option<SecretValue>,
    pub api_secret: Option<SecretValue>,
    pub ws_token: Option<SecretValue>,
    pub callback_url: Option<String>,
    pub paper_balances: Option<serde_json::Value>,
    pub default_output: Option<String>,
    pub default_pair: Option<String>,
    pub default_mcp_groups: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ResolvedCredentials {
    pub api_key: SecretValue,
    pub api_secret: SecretValue,
}

impl IndodaxConfig {
    fn get_base_dir() -> PathBuf {
        match dirs::config_dir() {
            Some(dir) => dir,
            None => std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
        }
    }

    pub fn config_path() -> PathBuf {
        Self::get_base_dir().join("indodax").join("config.toml")
    }

    pub fn config_dir() -> PathBuf {
        Self::get_base_dir().join("indodax")
    }

    pub fn paper_state_path() -> PathBuf {
        Self::config_dir().join("paper_state.json")
    }

    pub fn load() -> Result<Self, anyhow::Error> {
        if dirs::config_dir().is_none() {
            eprintln!("Warning: Could not determine user config directory. Falling back to current directory.");
        }
        let path = Self::config_path();
        if !path.exists() {
            return Ok(Self::default());
        }
        let content = fs::read_to_string(&path)?;
        let config: IndodaxConfig = toml::from_str(&content)?;
        Ok(config)
    }

    pub fn save(&self) -> Result<(), anyhow::Error> {
        let dir = Self::config_dir();
        fs::create_dir_all(&dir)?;
        let path = Self::config_path();
        let content = toml::to_string_pretty(self)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            let mut file = std::fs::OpenOptions::new()
                .write(true)
                .create(true)
                .truncate(true)
                .mode(0o600)
                .open(&path)?;
            use std::io::Write;
            file.write_all(content.as_bytes())?;
        }
        #[cfg(not(unix))]
        {
            fs::write(&path, content)?;
        }
        Ok(())
    }

    pub fn resolve_credentials(
        &self,
        cli_key: Option<String>,
        cli_secret: Option<String>,
    ) -> Result<Option<ResolvedCredentials>, anyhow::Error> {
        let api_key = if let Some(ref key) = cli_key {
            let trimmed = key.trim();
            if trimmed.is_empty() {
                None
            } else {
                Some(SecretValue::new(trimmed.to_string()))
            }
        } else {
            std::env::var("INDODAX_API_KEY")
                .ok()
                .map(|k| k.trim().to_string())
                .filter(|k| !k.is_empty())
                .map(SecretValue::new)
                .or_else(|| self.api_key.clone())
        };

        let api_secret = if let Some(ref secret) = cli_secret {
            let trimmed = secret.trim();
            if trimmed.is_empty() {
                None
            } else {
                Some(SecretValue::new(trimmed.to_string()))
            }
        } else {
            std::env::var("INDODAX_API_SECRET")
                .ok()
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .map(SecretValue::new)
                .or_else(|| self.api_secret.clone())
        };

        match (api_key, api_secret) {
            (Some(key), Some(secret)) => Ok(Some(ResolvedCredentials {
                api_key: key,
                api_secret: secret,
            })),
            _ => Ok(None),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;
    use std::env;

    #[test]
    fn test_secret_value_new() {
        let sv = SecretValue::new("test_secret");
        assert_eq!(sv.as_str(), "test_secret");
    }

    #[test]
    fn test_secret_value_as_str() {
        let sv = SecretValue::new("mykey");
        assert_eq!(sv.as_str(), "mykey");
    }

    #[test]
    fn test_secret_value_is_empty() {
        let sv_empty = SecretValue::new("");
        assert!(sv_empty.is_empty());

        let sv_non_empty = SecretValue::new("value");
        assert!(!sv_non_empty.is_empty());
    }

    #[test]
    fn test_secret_value_display_masked() {
        let sv = SecretValue::new("secret123");
        let display = format!("{}", sv);
        assert_eq!(display, "********");
    }

    #[test]
    fn test_secret_value_display_empty() {
        let sv = SecretValue::new("");
        let display = format!("{}", sv);
        assert_eq!(display, "");
    }

    #[test]
    fn test_secret_value_serialize_raw() {
        let sv = SecretValue::new("serialize_me");
        let serialized = serde_json::to_string(&sv).unwrap();
        assert!(serialized.contains("serialize_me"));
    }

    #[test]
    fn test_secret_value_serialize_empty() {
        let sv = SecretValue::new("");
        let serialized = serde_json::to_string(&sv).unwrap();
        assert_eq!(serialized, "\"\"");
    }

    #[test]
    fn test_secret_value_deserialize() {
        // JSON string format
        let json_str = "\"deserialize_me\"";
        let sv: SecretValue = serde_json::from_str(json_str).unwrap();
        assert_eq!(sv.as_str(), "deserialize_me");
    }

    #[test]
    fn test_secret_value_from_string() {
        let s = String::from("from_string");
        let sv: SecretValue = s.into();
        assert_eq!(sv.as_str(), "from_string");
    }

    #[test]
    fn test_secret_value_from_str() {
        let sv: SecretValue = "from_str".into();
        assert_eq!(sv.as_str(), "from_str");
    }

    #[test]
    fn test_secret_value_equality() {
        let sv1 = SecretValue::new("same");
        let sv2 = SecretValue::new("same");
        let sv3 = SecretValue::new("different");
        assert_eq!(sv1, sv2);
        assert_ne!(sv1, sv3);
    }

    #[test]
    fn test_indodax_config_default() {
        let config = IndodaxConfig::default();
        assert!(config.api_key.is_none());
        assert!(config.api_secret.is_none());
        assert!(config.callback_url.is_none());
        assert!(config.paper_balances.is_none());
    }

    #[test]
    #[serial]
    fn test_indodax_config_save_and_load() {
        let config = IndodaxConfig {
            api_key: Some(SecretValue::new("test_key")),
            api_secret: Some(SecretValue::new("test_secret")),
            callback_url: Some("http://callback.test".into()),
            ..Default::default()
        };

        let config_path = IndodaxConfig::config_path();
        config.save().unwrap();
        assert!(config_path.exists());

        let loaded = IndodaxConfig::load().unwrap();
        assert_eq!(loaded.api_key.as_ref().unwrap().as_str(), "test_key");
        assert_eq!(loaded.api_secret.as_ref().unwrap().as_str(), "test_secret");
        assert_eq!(
            loaded.callback_url.as_ref().unwrap(),
            "http://callback.test"
        );

        // Clean up
        fs::remove_file(&config_path).ok();
    }

    #[test]
    #[serial]
    fn test_indodax_config_load_no_file() {
        // Remove any existing config file to ensure clean state
        let config_path = IndodaxConfig::config_path();
        if config_path.exists() {
            fs::remove_file(&config_path).ok();
        }

        // Just test that load() works when no config file exists
        let config = IndodaxConfig::load().unwrap();
        assert!(config.api_key.is_none());
        assert!(config.api_secret.is_none());
    }

    #[test]
    #[serial]
    fn test_indodax_config_config_path() {
        let path = IndodaxConfig::config_path();
        assert!(path.to_string_lossy().contains("indodax"));
        assert!(path.to_string_lossy().contains("config.toml"));
    }

    #[test]
    #[serial]
    fn test_indodax_config_config_dir() {
        let dir = IndodaxConfig::config_dir();
        assert!(!dir.to_string_lossy().is_empty());
    }

    #[test]
    #[serial]
    fn test_resolve_credentials_cli_override() {
        env::remove_var("INDODAX_API_KEY");
        env::remove_var("INDODAX_API_SECRET");

        let config = IndodaxConfig {
            api_key: Some(SecretValue::new("test_key")),
            api_secret: Some(SecretValue::new("test_secret")),
            ..Default::default()
        };

        let result = config
            .resolve_credentials(Some("cli_key".into()), Some("cli_secret".into()))
            .unwrap();

        assert!(result.is_some());
        let creds = result.unwrap();
        assert_eq!(creds.api_key.as_str(), "cli_key");
        assert_eq!(creds.api_secret.as_str(), "cli_secret");
    }

    #[test]
    #[serial]
    fn test_resolve_credentials_env_variable() {
        // Clean up any existing env vars first
        env::remove_var("INDODAX_API_KEY");
        env::remove_var("INDODAX_API_SECRET");

        env::set_var("INDODAX_API_KEY", "env_key");
        env::set_var("INDODAX_API_SECRET", "env_secret");

        let config = IndodaxConfig::default();

        let result = config.resolve_credentials(None, None).unwrap();
        assert!(result.is_some());
        let creds = result.unwrap();
        assert_eq!(creds.api_key.as_str(), "env_key");
        assert_eq!(creds.api_secret.as_str(), "env_secret");

        env::remove_var("INDODAX_API_KEY");
        env::remove_var("INDODAX_API_SECRET");
    }

    #[test]
    #[serial]
    fn test_resolve_credentials_env_overrides_config() {
        // Clean up any existing env vars first
        env::remove_var("INDODAX_API_KEY");
        env::remove_var("INDODAX_API_SECRET");

        env::set_var("INDODAX_API_KEY", "env_key");
        env::set_var("INDODAX_API_SECRET", "env_secret");

        let config = IndodaxConfig {
            api_key: Some(SecretValue::new("test_key")),
            api_secret: Some(SecretValue::new("test_secret")),
            ..Default::default()
        };

        let result = config.resolve_credentials(None, None).unwrap();
        assert!(result.is_some());
        let creds = result.unwrap();
        assert_eq!(creds.api_key.as_str(), "env_key");
        assert_eq!(creds.api_secret.as_str(), "env_secret");

        env::remove_var("INDODAX_API_KEY");
        env::remove_var("INDODAX_API_SECRET");
    }

    #[test]
    #[serial]
    fn test_resolve_credentials_empty_cli() {
        env::remove_var("INDODAX_API_KEY");
        env::remove_var("INDODAX_API_SECRET");

        let config = IndodaxConfig::default();

        let result = config
            .resolve_credentials(Some("".into()), Some("".into()))
            .unwrap();

        assert!(result.is_none());
    }

    #[test]
    #[serial]
    fn test_resolve_credentials_empty_env_var() {
        // Clean up any existing env vars first
        env::remove_var("INDODAX_API_KEY");
        env::remove_var("INDODAX_API_SECRET");

        env::set_var("INDODAX_API_KEY", "");
        env::set_var("INDODAX_API_SECRET", "");

        let config = IndodaxConfig::default();

        let result = config.resolve_credentials(None, None).unwrap();
        assert!(result.is_none());

        env::remove_var("INDODAX_API_KEY");
        env::remove_var("INDODAX_API_SECRET");
    }

    #[test]
    #[serial]
    fn test_resolve_credentials_no_credentials() {
        env::remove_var("INDODAX_API_KEY");
        env::remove_var("INDODAX_API_SECRET");

        let config = IndodaxConfig::default();

        let result = config.resolve_credentials(None, None).unwrap();
        assert!(result.is_none());
    }

    #[test]
    #[serial]
    fn test_resolve_credentials_partial_none() {
        env::remove_var("INDODAX_API_KEY");
        env::remove_var("INDODAX_API_SECRET");

        let config = IndodaxConfig {
            api_key: Some(SecretValue::new("key_only")),
            api_secret: None,
            ..Default::default()
        };

        let result = config.resolve_credentials(None, None).unwrap();
        assert!(result.is_none());
    }
}
