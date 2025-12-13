//! Simple file-based logging for debugging.
//! Writes to a file since stdout/stderr are used by the TUI.

use std::fs::{File, OpenOptions};
use std::io::Write;
use std::sync::Mutex;

static LOGGER: Mutex<Option<File>> = Mutex::new(None);

/// Initialize the logger with an optional file path.
/// If no path is provided, logging is disabled.
/// Returns Ok(true) if logging enabled, Ok(false) if no logfile specified,
/// or Err with message if logfile couldn't be opened.
pub fn init(logfile: Option<String>) -> Result<bool, String> {
    if let Some(path) = logfile {
        match OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
        {
            Ok(file) => {
                let mut logger = LOGGER.lock().unwrap();
                *logger = Some(file);
                // Write initial message directly since logger is now set
                drop(logger);
                info(&format!("Log file opened: {}", path));
                Ok(true)
            }
            Err(e) => {
                Err(format!("Failed to open log file '{}': {}", path, e))
            }
        }
    } else {
        Ok(false)
    }
}

/// Log a message with timestamp.
pub fn log(level: &str, msg: &str) {
    let mut logger = LOGGER.lock().unwrap();
    if let Some(ref mut file) = *logger {
        let timestamp = chrono::Local::now().format("%Y-%m-%d %H:%M:%S%.3f");
        let _ = writeln!(file, "[{}] [{}] {}", timestamp, level, msg);
        let _ = file.flush();
    }
}

/// Log an info message.
pub fn info(msg: &str) {
    log("INFO", msg);
}

/// Log a warning message.
pub fn warn(msg: &str) {
    log("WARN", msg);
}

/// Log an error message.
pub fn error(msg: &str) {
    log("ERROR", msg);
}

/// Log a debug message.
pub fn debug(msg: &str) {
    log("DEBUG", msg);
}
