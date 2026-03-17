use anyhow::{Context, Result};
use tokio::process::Command;

pub struct Config {
    pub url: String,
    pub token: String,
    pub user_id: String,
    pub sleep_seconds: u64,
    pub meeting_message: String,
    pub meeting_status: String,
    pub ignore_if_offline: bool,
}

impl Config {
    pub async fn load() -> Result<Self> {
        let url = Self::get_snap_config("url").await?;
        let token = Self::get_snap_config("token").await?;
        let user_id = Self::get_snap_config("user-id").await?;
        
        // 1. Fetch, parse, and validate sleep seconds
        let sleep_str = Self::get_snap_config_opt("sleep-seconds", "300").await;
        let sleep_seconds = Self::validate_sleep_seconds(&sleep_str);
        
        let meeting_message = Self::get_snap_config_opt("meeting-message", " [In a meeting]").await;
        
        // 2. Fetch and validate the meeting status
        let raw_status = Self::get_snap_config_opt("meeting-status", "dnd").await;
        let meeting_status = match Self::validate_mattermost_status(&raw_status) {
            Ok(valid_status) => valid_status,
            Err(e) => {
                eprintln!("Configuration Warning: {} (Falling back to 'dnd')", e);
                "dnd".to_string()
            }
        };

        let ignore_str = Self::get_snap_config_opt("ignore-if-offline", "true").await;
        let ignore_if_offline = ignore_str.to_lowercase() == "true";

        Ok(Self { 
            url, 
            token, 
            user_id, 
            sleep_seconds, 
            meeting_message, 
            meeting_status, 
            ignore_if_offline 
        })
    }

    /// Validates the Mattermost status string
    fn validate_mattermost_status(status: &str) -> Result<String> {
        let allowed = ["online", "away", "dnd", "offline"];
        let lowercase_status = status.to_lowercase();
        
        if allowed.contains(&lowercase_status.as_str()) {
            Ok(lowercase_status)
        } else {
            anyhow::bail!(
                "Invalid status '{}'. Must be one of: online, away, dnd, offline",
                status
            )
        }
    }

    /// Parses and validates the sleep interval to prevent API spam
    fn validate_sleep_seconds(sleep_str: &str) -> u64 {
        let min_sleep = 10; // Absolute minimum allowed is 10 seconds
        let default_sleep = 300; // Default is 5 minutes

        match sleep_str.parse::<u64>() {
            Ok(seconds) => {
                if seconds < min_sleep {
                    eprintln!(
                        "Configuration Warning: 'sleep-seconds' is too low ({}). Enforcing minimum of {} seconds.", 
                        seconds, min_sleep
                    );
                    min_sleep
                } else {
                    seconds
                }
            }
            Err(_) => {
                eprintln!(
                    "Configuration Warning: Invalid 'sleep-seconds' value '{}'. Falling back to {} seconds.", 
                    sleep_str, default_sleep
                );
                default_sleep
            }
        }
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


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_mattermost_status_valid() {
        // It should accept exact matches
        assert_eq!(Config::validate_mattermost_status("online").unwrap(), "online");
        assert_eq!(Config::validate_mattermost_status("offline").unwrap(), "offline");
        
        // It should handle mixed-case gracefully
        assert_eq!(Config::validate_mattermost_status("DND").unwrap(), "dnd");
        assert_eq!(Config::validate_mattermost_status("Away").unwrap(), "away");
        assert_eq!(Config::validate_mattermost_status("oNlInE").unwrap(), "online");
    }

    #[test]
    fn test_validate_mattermost_status_invalid() {
        // It should reject completely wrong words
        assert!(Config::validate_mattermost_status("busy").is_err());
        assert!(Config::validate_mattermost_status("in-a-meeting").is_err());
        
        // It should reject empty strings
        assert!(Config::validate_mattermost_status("").is_err());
    }

    #[test]
    fn test_validate_sleep_seconds_valid() {
        // Normal values should pass through unchanged
        assert_eq!(Config::validate_sleep_seconds("300"), 300);
        assert_eq!(Config::validate_sleep_seconds("60"), 60);
        assert_eq!(Config::validate_sleep_seconds("10"), 10); // Exact minimum
    }

    #[test]
    fn test_validate_sleep_seconds_too_low() {
        // Values below 10 should be forced up to the minimum (10)
        assert_eq!(Config::validate_sleep_seconds("9"), 10);
        assert_eq!(Config::validate_sleep_seconds("1"), 10);
        assert_eq!(Config::validate_sleep_seconds("0"), 10);
    }

    #[test]
    fn test_validate_sleep_seconds_invalid_text() {
        // Garbage text or empty strings should trigger the default (300)
        assert_eq!(Config::validate_sleep_seconds("five"), 300);
        assert_eq!(Config::validate_sleep_seconds(""), 300);
        
        // Negative numbers will fail to parse as u64 and also trigger the default
        assert_eq!(Config::validate_sleep_seconds("-5"), 300);
    }
}