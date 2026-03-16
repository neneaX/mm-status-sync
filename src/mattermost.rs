use anyhow::Result;
use chrono::Local;
use log::info;
use reqwest::Client;
use serde_json::{json, Value};
use crate::config::Config;

pub struct Mattermost<'a> {
    pub http: &'a Client,
    pub config: &'a Config,
}

impl<'a> Mattermost<'a> {
    pub fn new(http: &'a Client, config: &'a Config) -> Self {
        Self { http, config }
    }

    pub async fn get_availability(&self) -> Result<String> {
        let endpoint = format!("{}/api/v4/users/{}/status", self.config.url, self.config.user_id);
        let res = self.http.get(&endpoint)
            .header("Authorization", format!("Bearer {}", self.config.token))
            .send()
            .await?
            .json::<Value>()
            .await?;

        if let Some(status) = res["status"].as_str() {
            Ok(status.to_string())
        } else {
            Ok("".to_string())
        }
    }

    pub async fn is_user_offline(&self) -> bool {
        self.get_availability().await.unwrap_or_default() == "offline"
    }

    pub async fn get_current_state(&self) -> Result<Value> {
        let endpoint = format!("{}/api/v4/users/me", self.config.url);
        let res = self.http.get(&endpoint)
            .header("Authorization", format!("Bearer {}", self.config.token))
            .send()
            .await?
            .json::<Value>()
            .await?;

        let mut state = json!({"emoji": "", "text": ""});
        if let Some(status_str) = res["props"]["customStatus"].as_str() {
            if !status_str.is_empty() {
                if let Ok(parsed) = serde_json::from_str::<Value>(status_str) {
                    state = parsed;
                }
            }
        }
        
        let mut availability = "online".to_string();
        if let Ok(avail) = self.get_availability().await {
            if !avail.is_empty() {
                availability = avail;
            }
        }

        if let Some(obj) = state.as_object_mut() {
            obj.insert("availability".to_string(), json!(availability));
        }

        Ok(state)
    }

    pub async fn update_status_and_avail(&self, status_json: Value, availability: &str) -> Result<()> {
        let text = status_json["text"].as_str().unwrap_or("").trim();
        let emoji = status_json["emoji"].as_str().unwrap_or("").trim();

        let status_endpoint = format!("{}/api/v4/users/{}/status/custom", self.config.url, self.config.user_id);
        if text.is_empty() && emoji.is_empty() {
            self.http.delete(&status_endpoint)
                .header("Authorization", format!("Bearer {}", self.config.token))
                .send()
                .await?;
        } else {
            let payload = json!({ "emoji": emoji, "text": text });
            self.http.put(&status_endpoint)
                .header("Authorization", format!("Bearer {}", self.config.token))
                .json(&payload)
                .send()
                .await?;
        }

        let avail_endpoint = format!("{}/api/v4/users/{}/status", self.config.url, self.config.user_id);
        let avail_payload = json!({ "user_id": self.config.user_id, "status": availability });
        self.http.put(&avail_endpoint)
            .header("Authorization", format!("Bearer {}", self.config.token))
            .json(&avail_payload)
            .send()
            .await?;

        let timestamp = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
        info!("[{}] API Updated: Status -> '{}', Emoji: '{}', Avail -> '{}'", timestamp, text, emoji, availability);
        Ok(())
    }
}