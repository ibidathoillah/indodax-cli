use serde::{Deserialize, Serialize};
use std::fmt;
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SecretValue(String);

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

impl Serialize for SecretValue {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.0)
    }
}

impl<'de> Deserialize<'de> for SecretValue {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;
        Ok(SecretValue(s))
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

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct IndodaxConfig {
    pub api_key: Option<SecretValue>,
    pub api_secret: Option<SecretValue>,
    pub paper_balances: Option<serde_json::Value>,
}

impl Default for IndodaxConfig {
    fn default() -> Self {
        Self {
            api_key: None,
            api_secret: None,
            paper_balances: None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ResolvedCredentials {
    pub api_key: SecretValue,
    pub api_secret: SecretValue,
}

impl IndodaxConfig {
    pub fn config_path() -> PathBuf {
        let base = dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."));
        base.join("indodax").join("config.toml")
    }

    pub fn config_dir() -> PathBuf {
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("indodax")
    }

    pub fn load() -> Result<Self, anyhow::Error> {
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
        fs::write(&path, content)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(&path)?.permissions();
            perms.set_mode(0o600);
            fs::set_permissions(&path, perms)?;
        }
        Ok(())
    }

    pub fn resolve_credentials(
        &self,
        cli_key: Option<String>,
        cli_secret: Option<String>,
    ) -> Result<Option<ResolvedCredentials>, anyhow::Error> {
        let api_key = if let Some(ref key) = cli_key {
            if key.is_empty() {
                None
            } else {
                Some(SecretValue::new(key.clone()))
            }
        } else {
            std::env::var("INDODAX_API_KEY")
                .ok()
                .filter(|k| !k.is_empty())
                .map(SecretValue::new)
                .or_else(|| self.api_key.clone())
        };

        let api_secret = if let Some(ref secret) = cli_secret {
            if secret.is_empty() {
                None
            } else {
                Some(SecretValue::new(secret.clone()))
            }
        } else {
            std::env::var("INDODAX_API_SECRET")
                .ok()
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
