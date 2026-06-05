use crate::errors::IndodaxError;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PriceAlert {
    pub id: u64,
    pub pair: String,
    pub condition: AlertCondition,
    pub created_at: u64,
    pub triggered_at: Option<u64>,
    pub status: AlertStatus,
    pub note: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type")]
pub enum AlertCondition {
    #[serde(rename = "above")]
    Above { price: f64 },
    #[serde(rename = "below")]
    Below { price: f64 },
    #[serde(rename = "change_up")]
    ChangeUp { percent: f64, from_price: f64 },
    #[serde(rename = "change_down")]
    ChangeDown { percent: f64, from_price: f64 },
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum AlertStatus {
    Active,
    Triggered,
    Cancelled,
}

impl PriceAlert {
    pub fn new(id: u64, pair: String, condition: AlertCondition, note: Option<String>) -> Self {
        Self {
            id,
            pair,
            condition,
            created_at: web_time::SystemTime::now()
                .duration_since(web_time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64,
            triggered_at: None,
            status: AlertStatus::Active,
            note,
        }
    }
}

pub fn get_alerts_path() -> Result<PathBuf, IndodaxError> {
    let config_dir = dirs::config_dir()
        .ok_or_else(|| IndodaxError::Config("Could not find config directory".into()))?;
    let path = config_dir.join("indodax");
    if !path.exists() {
        fs::create_dir_all(&path).map_err(|e| {
            IndodaxError::Config(format!("Failed to create config directory: {}", e))
        })?;
    }
    Ok(path.join("alerts.json"))
}

pub fn load_alerts() -> Result<Vec<PriceAlert>, IndodaxError> {
    let path = get_alerts_path()?;
    if !path.exists() {
        return Ok(Vec::new());
    }
    let content = fs::read_to_string(&path)
        .map_err(|e| IndodaxError::Config(format!("Failed to read alerts file: {}", e)))?;
    if content.trim().is_empty() {
        return Ok(Vec::new());
    }
    serde_json::from_str::<Vec<PriceAlert>>(&content)
        .map_err(|e| IndodaxError::Config(format!("Failed to parse alerts: {}", e)))
}

pub fn save_alerts(alerts: &[PriceAlert]) -> Result<(), IndodaxError> {
    let path = get_alerts_path()?;
    let content = serde_json::to_string_pretty(alerts)
        .map_err(|e| IndodaxError::Config(format!("Failed to serialize alerts: {}", e)))?;

    #[cfg(unix)]
    {
        use std::io::Write;
        use std::os::unix::fs::OpenOptionsExt;
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .mode(0o600)
            .open(&path)
            .map_err(|e| IndodaxError::Config(format!("Failed to open alerts file: {}", e)))?;
        file.write_all(content.as_bytes())
            .map_err(|e| IndodaxError::Config(format!("Failed to write alerts: {}", e)))?;
    }
    #[cfg(not(unix))]
    {
        fs::write(&path, content)
            .map_err(|e| IndodaxError::Config(format!("Failed to write alerts file: {}", e)))?;
    }
    Ok(())
}
