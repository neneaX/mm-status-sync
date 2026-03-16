use tokio::fs;

pub async fn is_camera_active() -> bool {
    if let Ok(mut pids) = fs::read_dir("/proc").await {
        while let Ok(Some(pid_entry)) = pids.next_entry().await {
            if pid_entry.file_name().to_string_lossy().parse::<u32>().is_err() {
                continue;
            }

            let fd_path = pid_entry.path().join("fd");
            if let Ok(mut fds) = fs::read_dir(&fd_path).await {
                while let Ok(Some(fd_entry)) = fds.next_entry().await {
                    if let Ok(target) = fs::read_link(fd_entry.path()).await {
                        if target.to_string_lossy().starts_with("/dev/video") {
                            return true; 
                        }
                    }
                }
            }
        }
    }
    false
}