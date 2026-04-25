use crate::paths::STATUS_FILE;
use crate::runtime::foreground::ForegroundSnapshot;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

#[derive(Serialize, Deserialize, Debug, Clone, Default, PartialEq)]
pub struct DaemonStatus {
    pub daemon_alive: bool,
    pub mode: String,
    pub uptime_secs: u64,
    pub ticks: u64,
    pub enabled_features: Vec<String>,
    pub foreground_pid: Option<i32>,
    pub foreground_package: Option<String>,
    pub last_event: Option<String>,
    pub last_skip_reason: Option<String>,
    pub warnings: Vec<String>,

    // Preload Measurement Fields
    pub last_preload_package: Option<String>,
    pub last_preload_file_count: usize,
    pub last_preload_files_failed: usize,
    pub last_preload_bytes: u64,
    pub last_preload_discovery_ms: u64,
    pub last_preload_readahead_ms: u64,
    pub last_preload_total_ms: u64,
    pub last_preload_result: Option<String>,
}

impl DaemonStatus {
    pub fn apply_foreground_snapshot(&mut self, snapshot: &ForegroundSnapshot) {
        self.foreground_pid = snapshot.pid;
        self.foreground_package = snapshot.package.clone();
        self.last_skip_reason = snapshot.last_skip_reason.clone();

        if let Some(pkg) = snapshot.package.as_deref() {
            self.last_event = Some(format!("Accepted {}", pkg));
        } else if let Some(reason) = snapshot.last_skip_reason.as_deref() {
            self.last_event = Some(format!("Skipped {}", reason));
        } else {
            self.last_event = None;
        }
    }

    pub fn write(&self) -> Result<(), std::io::Error> {
        let path = Path::new(STATUS_FILE);

        // Ensure directory exists
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        let temp_path = format!("{}.tmp", STATUS_FILE);

        let json = serde_json::to_string_pretty(self).map_err(std::io::Error::other)?;

        fs::write(&temp_path, json)?;
        fs::rename(&temp_path, STATUS_FILE)?;

        Ok(())
    }

    pub fn read() -> Option<Self> {
        if let Ok(content) = fs::read_to_string(STATUS_FILE) {
            serde_json::from_str(&content).ok()
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::paths::STATUS_FILE;

    #[test]
    fn test_apply_foreground_snapshot() {
        let mut status = DaemonStatus::default();
        let snapshot = ForegroundSnapshot {
            pid: Some(123),
            package: Some("com.example.app".to_string()),
            last_skip_reason: None,
        };

        status.apply_foreground_snapshot(&snapshot);

        assert_eq!(status.foreground_pid, Some(123));
        assert_eq!(
            status.foreground_package.as_deref(),
            Some("com.example.app")
        );
        assert_eq!(
            status.last_event.as_deref(),
            Some("Accepted com.example.app")
        );
        assert_eq!(status.last_skip_reason, None);
    }

    #[test]
    fn test_apply_foreground_skip_snapshot() {
        let mut status = DaemonStatus::default();
        let snapshot = ForegroundSnapshot {
            pid: None,
            package: None,
            last_skip_reason: Some("no_app_candidate".to_string()),
        };

        status.apply_foreground_snapshot(&snapshot);

        assert_eq!(status.foreground_pid, None);
        assert_eq!(status.foreground_package, None);
        assert_eq!(status.last_skip_reason.as_deref(), Some("no_app_candidate"));
        assert_eq!(
            status.last_event.as_deref(),
            Some("Skipped no_app_candidate")
        );
    }

    #[test]
    fn test_atomic_status_write() {
        let status = DaemonStatus {
            mode: "test".to_string(),
            ..Default::default()
        };
        let _ = status.write();

        assert!(fs::metadata(STATUS_FILE).is_ok());
        let read_status = DaemonStatus::read().unwrap();
        assert_eq!(read_status.mode, "test");

        let _ = fs::remove_file(STATUS_FILE);
    }
}
