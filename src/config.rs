use anyhow::{Context, Result};
use tokio::process::Command;

pub struct Config {
    pub url: String,
    pub token: String,
    pub user_id: String,
    pub sleep_seconds: u64,
    pub meeting_message: String,
    pub ignore_if_offline: bool,
}

impl Config {
    pub async fn load() -> Result<Self> {
        let url = Self::get_snap_config("url").await?;
        let token = Self::get_snap_config("token").await?;
        let user_id = Self::get_snap_config("user-id").await?;
        
        let sleep_str = Self::get_snap_config_opt("sleep-seconds", "300").await;
        let sleep_seconds = sleep_str.parse::<u64>().unwrap_or(300);
        let meeting_message = Self::get_snap_config_opt("meeting-message", " [In a meeting]").await;

        let ignore_str = Self::get_snap_config_opt("ignore-if-offline", "true").await;
        let ignore_if_offline = ignore_str.to_lowercase() == "true";

        Ok(Self { url, token, user_id, sleep_seconds, meeting_message, ignore_if_offline })
    }

    async fn get_snap_config(key: &str) -> Result<String> {
        let output = Command::new("snapctl").arg("get").arg(key).output().await
            .with_context(|| format!("Failed to call snapctl for key: {}", key))?;
        if output.status.success() {
            let val = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if val.is_empty() { anyhow::bail!("'{}' is empty.", key); }
            Ok(val)
        } else {
            anyhow::bail!("snapctl error: {}", String::from_utf8_lossy(&output.stderr))
        }
    }

    async fn get_snap_config_opt(key: &str, default: &str) -> String {
        if let Ok(output) = Command::new("snapctl").arg("get").arg(key).output().await {
            if output.status.success() {
                let val = String::from_utf8_lossy(&output.stdout).trim().to_string();
                if !val.is_empty() { return val; }
            }
        }
        default.to_string()
    }
}