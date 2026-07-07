// ./src-tauri/src/app_log.rs
/*
Purpose: Provide robust file logging for the Genomics Caddy application.
Logs are written to both standard error (for developer console) and a persistent log file `genomics_caddy.log` in the application data directory.
*/

use std::fs::OpenOptions;
use std::io::Write;
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

static LOG_PATH: Mutex<Option<PathBuf>> = Mutex::new(None);

/// Initialize the logger with the resolved data directory.
pub fn init(data_dir: PathBuf) {
    let log_file = data_dir.join("genomics_caddy.log");
    
    // Rotate/truncate log if it gets too large (> 10 MB)
    if let Ok(metadata) = std::fs::metadata(&log_file) {
        if metadata.len() > 10 * 1024 * 1024 {
            let _ = std::fs::remove_file(&log_file);
        }
    }

    if let Ok(mut guard) = LOG_PATH.lock() {
        *guard = Some(log_file);
    }
    
    log("SYSTEM", "Genomics Caddy application logger initialized.");
}

/// Log a message with a specific tag.
pub fn log(tag: &str, message: &str) {
    let path = if let Ok(guard) = LOG_PATH.lock() {
        guard.clone()
    } else {
        None
    };

    let time_str = match SystemTime::now().duration_since(UNIX_EPOCH) {
        Ok(d) => {
            let total_secs = d.as_secs();
            let hours = (total_secs % 86400) / 3600;
            let minutes = (total_secs % 3600) / 60;
            let seconds = total_secs % 60;
            format!("{:02}:{:02}:{:02}", hours, minutes, seconds)
        }
        Err(_) => "00:00:00".to_string(),
    };

    let line = format!("[{}] [{}] {}\n", time_str, tag, message);
    
    // Print to standard error (visible in terminal / console)
    eprint!("{}", line);

    // Append to file if initialized
    if let Some(p) = path {
        if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(p) {
            let _ = file.write_all(line.as_bytes());
        }
    }
}
