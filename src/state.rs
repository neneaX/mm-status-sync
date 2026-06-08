use chrono::Utc;
use log::{error, info};
use serde_json::{json, Value};
use std::path::Path;
use tokio::fs;

use crate::mattermost::Mattermost;

// --- Pure Logic Helpers ---

/// Returns true if the status has a non-zero `expires_at` that is already in the past.
/// Mattermost uses "0001-01-01T00:00:00Z" as a sentinel for "no expiry".
pub fn has_status_expired(status: &Value) -> bool {
    match status["expires_at"].as_str() {
        Some(expires_at) if !expires_at.is_empty() && !expires_at.starts_with("0001-01-01") => {
            chrono::DateTime::parse_from_rfc3339(expires_at)
                .map(|expiry| expiry < Utc::now())
                .unwrap_or(false)
        }
        _ => false,
    }
}

pub fn build_meeting_text(old_text: &str, meeting_message: &str) -> String {
    if !old_text.contains(meeting_message) {
        format!("{} {}", old_text, meeting_message).trim().to_string()
    } else {
        old_text.to_string()
    }
}

// --- Business Logic Controllers ---
pub async fn handle_meeting_start(mm: &Mattermost<'_>, backup_file: &Path) {
    if backup_file.exists() {
        return; 
    }

    if mm.config.ignore_if_offline && mm.is_user_offline().await {
        return; 
    }

    let current_state = mm.get_current_state().await.unwrap_or_else(|_| json!({"emoji": "", "text": "", "availability": "online"}));
    
    if let Err(e) = fs::write(backup_file, current_state.to_string()).await {
        error!("BACKUP FATAL: Could not save file ({}). Aborting sync.", e);
        return;
    }
    info!("BACKUP: Saved original status to {:?}", backup_file);

    let old_text = current_state["text"].as_str().unwrap_or("");
    let new_text = build_meeting_text(old_text, &mm.config.meeting_message);

    let new_status = json!({"emoji": "meet", "text": new_text});
    if let Err(e) = mm.update_status_and_avail(new_status, &mm.config.meeting_status).await {
        error!("API ERROR: Failed to set meeting status: {}", e);
    }
}

pub async fn handle_meeting_end(mm: &Mattermost<'_>, backup_file: &Path) {
    if !backup_file.exists() {
        return; 
    }

    if mm.config.ignore_if_offline && mm.is_user_offline().await {
        let _ = fs::remove_file(backup_file).await;
        info!("RESTORE SKIPPED: User is currently offline. Backup deleted without changing status.");
        return;
    }

    match fs::read_to_string(backup_file).await {
        Ok(backup_str) => {
            if let Ok(mut prev_status) = serde_json::from_str::<Value>(&backup_str) {
                // If the saved status had a timer that already expired while we were in the
                // meeting, Mattermost will have already cleared it server-side. Restoring it
                // would bring it back as a permanent status — restore blank instead.
                if has_status_expired(&prev_status) {
                    info!("RESTORE: Saved status had already expired. Restoring to blank.");
                    let saved_avail = prev_status["availability"].as_str().unwrap_or("online").to_string();
                    prev_status = json!({"emoji": "", "text": "", "availability": saved_avail});
                }

                let saved_avail = prev_status["availability"].as_str().unwrap_or("online").to_string();

                if let Err(e) = mm.update_status_and_avail(prev_status, &saved_avail).await {
                    error!("API ERROR: Failed to restore status: {}", e);
                } else {
                    let _ = fs::remove_file(backup_file).await;
                    info!("RESTORE: Status restored (Avail: {}) and backup deleted.", saved_avail);
                }
            }
        }
        Err(e) => error!("RESTORE ERROR: Could not read backup file: {}", e),
    }
}

// ==========================================
//                 TESTS
// ==========================================
#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_build_meeting_text_appends_correctly() {
        let old = "Focusing";
        let msg = "[In a meeting]";
        assert_eq!(build_meeting_text(old, msg), "Focusing [In a meeting]");
    }

    #[test]
    fn test_build_meeting_text_prevents_duplicates() {
        let old = "Focusing [In a meeting]";
        let msg = "[In a meeting]";
        assert_eq!(build_meeting_text(old, msg), "Focusing [In a meeting]");
    }

    #[test]
    fn test_has_status_expired_with_past_timestamp() {
        let expired = json!({"emoji": "fork_and_knife", "text": "Out for lunch", "expires_at": "2000-01-01T12:00:00Z"});
        assert!(has_status_expired(&expired));
    }

    #[test]
    fn test_has_status_expired_with_future_timestamp() {
        let not_expired = json!({"emoji": "fork_and_knife", "text": "Out for lunch", "expires_at": "2099-01-01T12:00:00Z"});
        assert!(!has_status_expired(&not_expired));
    }

    #[test]
    fn test_has_status_expired_with_zero_sentinel() {
        // Mattermost's sentinel value for "no expiry"
        let no_expiry = json!({"emoji": "sword", "text": "Focusing", "expires_at": "0001-01-01T00:00:00Z"});
        assert!(!has_status_expired(&no_expiry));
    }

    #[test]
    fn test_has_status_expired_with_no_field() {
        let no_expiry = json!({"emoji": "sword", "text": "Focusing"});
        assert!(!has_status_expired(&no_expiry));
    }

    #[tokio::test]
    async fn test_backup_file_read_write() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("mock_backup.json");
        let mock_state = json!({"emoji": "sword", "text": "Defending", "availability": "away"});

        let write_res = fs::write(&file_path, mock_state.to_string()).await;
        assert!(write_res.is_ok(), "Failed to write");

        let read_str = fs::read_to_string(&file_path).await.unwrap();
        let parsed: Value = serde_json::from_str(&read_str).unwrap();
        assert_eq!(parsed["emoji"].as_str().unwrap(), "sword");
    }
}