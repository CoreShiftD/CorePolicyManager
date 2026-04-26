use std::path::PathBuf;
use std::time::Duration;

/// Framework-level configuration for a daemon instance.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DaemonConfig {
    pub work_dir: PathBuf,
    pub poll_interval: Duration,
}

impl Default for DaemonConfig {
    fn default() -> Self {
        Self {
            work_dir: PathBuf::from("/data/local/tmp/coreshift"),
            poll_interval: Duration::from_millis(100),
        }
    }
}
