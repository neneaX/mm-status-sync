mod config;
mod mattermost;
mod camera;
mod state;

use log::{info, warn};
use reqwest::Client;
use std::env;
use std::path::PathBuf;
use tokio::signal::unix::{signal, SignalKind};
use tokio::time::{interval, Duration};

// Import our isolated modules
use config::Config;
use mattermost::Mattermost;
use camera::is_camera_active;
use state::{handle_meeting_start, handle_meeting_end};

#[tokio::main]
async fn main() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    let args: Vec<String> = env::args().collect();
    if args.contains(&String::from("--help")) || args.contains(&String::from("-h")) {
        println!("Mattermost Status Sync Daemon\nTo configure:\n  sudo snap set mm-status-sync url=\"...\" token=\"...\" user-id=\"...\"");
        return; 
    }

    info!("Starting Mattermost Status Sync Daemon...");
    
    let http_client = Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .expect("Failed to build HTTP client");
    
    let data_dir = env::var("SNAP_DATA").expect("FATAL: SNAP_DATA environment variable is missing!");
    let backup_file = PathBuf::from(&data_dir).join("mm_status_backup.json");

    let mut sigterm = signal(SignalKind::terminate()).expect("Failed to bind SIGTERM handler");
    let mut sigint = signal(SignalKind::interrupt()).expect("Failed to bind SIGINT handler");

    let initial_sleep = Config::load().await.map(|c| c.sleep_seconds).unwrap_or(60);
    let mut timer = interval(Duration::from_secs(initial_sleep));

    // The Ultimate Orchestrator Loop
    loop {
        tokio::select! {
            _ = sigterm.recv() => {
                info!("Received SIGTERM from systemd. Shutting down safely.");
                break;
            }
            _ = sigint.recv() => {
                info!("Received SIGINT. Shutting down safely.");
                break;
            }
            _ = timer.tick() => {
                let config = match Config::load().await {
                    Ok(c) => c,
                    Err(e) => {
                        warn!("Config missing or invalid: {}. Retrying on next tick...", e);
                        continue;
                    }
                };

                let mm = Mattermost::new(&http_client, &config);

                if is_camera_active().await {
                    handle_meeting_start(&mm, &backup_file).await;
                } else {
                    handle_meeting_end(&mm, &backup_file).await;
                }
            }
        }
    }
}