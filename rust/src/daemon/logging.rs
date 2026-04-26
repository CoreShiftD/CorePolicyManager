use crate::paths::{LOG_FILE, WORK_DIR};
use std::collections::HashMap;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::sync::{Mutex, MutexGuard};
use std::time::{Duration, Instant, SystemTime};

static STDOUT_ENABLED: Mutex<Option<bool>> = Mutex::new(None);
static DEBUG_ENABLED: Mutex<Option<bool>> = Mutex::new(None);
static LAST_LOGS: Mutex<Option<HashMap<String, Instant>>> = Mutex::new(None);

pub enum Level {
    Debug,
    Info,
    Warn,
    Error,
}

fn lock_or_recover<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    match mutex.lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    }
}

impl Level {
    fn as_str(&self) -> &'static str {
        match self {
            Level::Debug => "DEBUG",
            Level::Info => "INFO",
            Level::Warn => "WARN",
            Level::Error => "ERROR",
        }
    }
}

pub fn init() {
    let _ = fs::create_dir_all(WORK_DIR);
    let stdout_enabled = std::env::var("COREPOLICY_STDOUT_LOG")
        .map(|v| v == "1")
        .unwrap_or(false);
    {
        let mut stdout = lock_or_recover(&STDOUT_ENABLED);
        *stdout = Some(stdout_enabled);
    }

    {
        let mut debug = lock_or_recover(&DEBUG_ENABLED);
        *debug = Some(
            std::env::var("COREPOLICY_DEBUG_LOG")
                .map(|v| v == "1")
                .unwrap_or(false)
                || stdout_enabled,
        );
    }

    let mut last_logs = lock_or_recover(&LAST_LOGS);
    if last_logs.is_none() {
        *last_logs = Some(HashMap::new());
    }
}

pub fn debug(msg: &str) {
    if debug_enabled() {
        log_internal(Level::Debug, msg);
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

/// Log a debug message only if message with this key hasn't been logged in the last duration.
pub fn dedup_debug(key: &str, msg: &str, min_interval: Duration) {
    dedup_log(Level::Debug, key, msg, min_interval);
}

/// Log only if message with this key hasn't been logged in the last duration
pub fn dedup_info(key: &str, msg: &str, min_interval: Duration) {
    dedup_log(Level::Info, key, msg, min_interval);
}

fn dedup_log(level: Level, key: &str, msg: &str, min_interval: Duration) {
    if matches!(level, Level::Debug) && !debug_enabled() {
        return;
    }

    let mut last_logs_guard = lock_or_recover(&LAST_LOGS);
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
    log_internal(level, msg);
}

fn log_internal(level: Level, msg: &str) {
    let timestamp = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);

    let line = format!("[{}] [{}] {}\n", timestamp, level.as_str(), msg);

    if !matches!(level, Level::Debug)
        && let Ok(mut file) = OpenOptions::new().create(true).append(true).open(LOG_FILE)
    {
        let _ = file.write_all(line.as_bytes());
    }

    let stdout_enabled = *lock_or_recover(&STDOUT_ENABLED);
    if stdout_enabled.unwrap_or(false) {
        print!("{}", line);
    }
}

fn debug_enabled() -> bool {
    let mut debug = lock_or_recover(&DEBUG_ENABLED);
    if debug.is_none() {
        let stdout_enabled = *lock_or_recover(&STDOUT_ENABLED);
        *debug = Some(
            std::env::var("COREPOLICY_DEBUG_LOG")
                .map(|v| v == "1")
                .unwrap_or(false)
                || stdout_enabled.unwrap_or(false),
        );
    }
    debug.unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    #[test]
    fn lock_poison_recovery_does_not_panic() {
        let mutex = Arc::new(Mutex::new(1u32));
        let worker_mutex = Arc::clone(&mutex);
        let _ = std::thread::spawn(move || {
            let _guard = worker_mutex.lock().unwrap();
            panic!("poison");
        })
        .join();

        let guard = lock_or_recover(&mutex);
        assert_eq!(*guard, 1);
    }
}
