# 🦀 Mattermost Status Sync

Because forgetting to clear your "In a Meeting" status three hours after the call ended is a universal human experience.

This lightweight, blazingly fast background daemon monitors your webcam usage (`/dev/video*`) and automatically updates your Mattermost status and availability. When the camera turns on, it appends a customizable message (like " - In a meeting") to your current status and sets your availability to "Do Not Disturb" (DND). When the camera turns off, your previous status **and availability** (Online, Away, etc.) are restored exactly as they were.

## ✨ Features
* **Fully Configurable:** Change your tokens, sync interval, and custom away messages on the fly using native `snap set` commands.
* **Smart Offline Detection:** If you close your laptop or go "Offline" in Mattermost, the daemon smartly ignores your camera and will not force your status back to "Online" while you sleep.
* **State Preservation:** Backs up your exact state (Emoji, Text, and Availability) safely in the Snap `$SNAP_DATA` directory and restores it flawlessly.
* **Native & Zero-Overhead:** Written in Rust using a fully asynchronous `tokio` runtime. It replaces bulky `fuser`/`jq`/`curl` sub-shells by reading the Linux kernel's `/proc` file descriptors natively, using practically zero CPU or memory.

---

## 📋 Prerequisites
To build this project from source, you will need:
* [Rust & Cargo](https://rustup.rs/)
* Snapcraft (`sudo snap install snapcraft --classic`)

---

## 🚀 Build and Installation

1. **Build the Snap Package:**
   Navigate to the root of the project directory and run the following commands to compile the Rust binary and package it:
   ```bash
   snapcraft clean
   snapcraft pack
   ```

2. **Install the Snap:**
   Install the generated `.snap` file locally. The `--classic` flag is required so the daemon has the system privileges necessary to monitor host video devices via the `/proc` filesystem.
   ```bash
   sudo snap install mm-status-sync_*.snap --dangerous --classic
   ```

---

## 🛠️ Configuration

The daemon runs automatically in the background as a system service. However, **it will remain in a sleep loop until you provide your Mattermost credentials**. 

You configure the daemon using Snap's native configuration hooks. 

### Required Settings
You must set all three of these for the daemon to connect to your workspace:
```bash
sudo snap set mm-status-sync url="https://your-mattermost-url.com"
sudo snap set mm-status-sync token="your-personal-access-token"
sudo snap set mm-status-sync user-id="your-user-id"
```
*(Note: You can generate a Personal Access Token in Mattermost under Account Settings > Security > Personal Access Tokens).*

### Optional Settings
You can fine-tune the daemon's behavior. If you do not set these, it will fall back to the default values.
```bash
# Check the camera every X seconds (Default: 300)
sudo snap set mm-status-sync sleep-seconds="60"

# Change the appended status message (Default: " [In a meeting]")
sudo snap set mm-status-sync meeting-message="🎥 On a call"

# Prevent the daemon from updating your status if you are currently marked as "Offline" (Default: true)
sudo snap set mm-status-sync ignore-if-offline="true"
```

### Reviewing Your Configuration
To see your currently active settings at any time, run:
```bash
sudo snap get mm-status-sync
```

---

## 🔍 Troubleshooting & Logs

Because this runs headlessly, you will need to rely on Snap's built-in tools to monitor its health.

### 1. View the Live Logs
If your status isn't updating, the best place to start is the daemon's output logs. You can follow the live log stream by running:
```bash
sudo snap logs -f mm-status-sync
```

**Common Log Messages:**
* **`INFO: Starting Mattermost Status Sync Daemon...`**: The service has successfully started or restarted.
* **`WARN: Config missing or invalid`**: The daemon is missing the URL, Token, or User ID. It will retry on the next tick.
* **`INFO: BACKUP: Saved original status`**: The camera turned on, and your previous status was securely saved to disk.
* **`INFO: RESTORE: Status restored (Avail: online) and backup deleted.`**: The camera turned off, and your original status and availability were returned to normal.
* **`INFO: RESTORE SKIPPED: User is currently offline.`**: The camera turned off, but because you went offline during the meeting, the daemon safely deleted the backup without forcing your status to "Online".
* **`ERROR: API ERROR: Failed to set meeting status`**: The daemon could not reach your Mattermost instance. Check your URL, Token, and network connection.
