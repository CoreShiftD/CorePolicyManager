use crate::paths::{LOG_FILE, WORK_DIR};
use std::collections::HashMap;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::sync::Mutex;
use std::time::{Duration, Instant, SystemTime};

static STDOUT_ENABLED: Mutex<Option<bool>> = Mutex::new(None);
static LAST_LOGS: Mutex<Option<HashMap<String, Instant>>> = Mutex::new(None);

pub enum Level {
    Info,
    Warn,
    Error,
}

impl Level {
    fn as_str(&self) -> &'static str {
        match self {
            Level::Info => "INFO",
            Level::Warn => "WARN",
            Level::Error => "ERROR",
        }
    }
}

pub fn init() {
    let _ = fs::create_dir_all(WORK_DIR);
    let mut stdout = STDOUT_ENABLED.lock().unwrap();
    *stdout = Some(
        std::env::var("COREPOLICY_STDOUT_LOG")
            .map(|v| v == "1")
            .unwrap_or(false),
    );

    let mut last_logs = LAST_LOGS.lock().unwrap();
    if last_logs.is_none() {
        *last_logs = Some(HashMap::new());
    }
}

pub fn info(msg: &str) {
    log_internal(Level::Info, msg);
}

pub fn warn(msg: &str) {
    log_internal(Level::Warn, msg);
}

pub fn error(msg: &str) {
    log_internal(Level::Error, msg);
}

/// Log only if message with this key hasn't been logged in the last duration
pub fn dedup_info(key: &str, msg: &str, min_interval: Duration) {
    let mut last_logs_guard = LAST_LOGS.lock().unwrap();
    if last_logs_guard.is_none() {
        *last_logs_guard = Some(HashMap::new());
    }
    let last_logs = last_logs_guard.as_mut().unwrap();

    let now = Instant::now();
    if let Some(&last_time) = last_logs.get(key)
        && now.duration_since(last_time) < min_interval
    {
        return;
    }

    last_logs.insert(key.to_string(), now);
    info(msg);
}

fn log_internal(level: Level, msg: &str) {
    let timestamp = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);

    let line = format!("[{}] [{}] {}\n", timestamp, level.as_str(), msg);

    if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(LOG_FILE) {
        let _ = file.write_all(line.as_bytes());
    }

    let stdout_enabled = *STDOUT_ENABLED.lock().unwrap();
    if stdout_enabled.unwrap_or(false) {
        print!("{}", line);
    }
}
