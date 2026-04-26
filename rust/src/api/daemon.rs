pub use crate::api::config::DaemonConfig;
pub use crate::daemon::runtime::Daemon;

use crate::api::resolver::ForegroundEvent;
use crate::runtime::status::DaemonStatus;
use std::path::PathBuf;
use std::time::Duration;

/// Error type for the daemon framework.
#[derive(Debug)]
pub enum DaemonError {
    Io(std::io::Error),
    Runtime(String),
}

impl From<std::io::Error> for DaemonError {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e)
    }
}

impl std::fmt::Display for DaemonError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(e) => write!(f, "IO error: {}", e),
            Self::Runtime(s) => write!(f, "Runtime error: {}", s),
        }
    }
}

/// Context passed to synchronous daemon features.
pub struct DaemonContext {
    pub work_dir: PathBuf,
    pub status: DaemonStatus,
}

/// Context passed to threaded (worker) features.
pub struct FeatureThreadContext {
    pub work_dir: PathBuf,
    pub shutdown_receiver: std::sync::mpsc::Receiver<()>,
    pub foreground_receiver: std::sync::mpsc::Receiver<ForegroundEvent>,
}

impl FeatureThreadContext {
    pub fn shutdown_requested(&self) -> bool {
        match self.shutdown_receiver.try_recv() {
            Ok(_) => true,
            Err(std::sync::mpsc::TryRecvError::Disconnected) => true,
            Err(std::sync::mpsc::TryRecvError::Empty) => false,
        }
    }

    pub fn wait_for_shutdown(&self, timeout: Duration) {
        let _ = self.shutdown_receiver.recv_timeout(timeout);
    }
}

/// A synchronous feature that responds to daemon events.
pub trait DaemonFeature: Send {
    fn name(&self) -> &'static str;

    fn on_start(&mut self, _ctx: &DaemonContext) -> Result<(), DaemonError> {
        Ok(())
    }

    fn on_foreground_changed(
        &mut self,
        _ctx: &DaemonContext,
        _event: &ForegroundEvent,
    ) -> Result<(), DaemonError> {
        Ok(())
    }

    fn on_tick(&mut self, _ctx: &DaemonContext) -> Result<(), DaemonError> {
        Ok(())
    }

    fn on_shutdown(&mut self, _ctx: &DaemonContext) -> Result<(), DaemonError> {
        Ok(())
    }
}

/// A background worker feature that runs in its own thread.
pub trait ThreadedFeature: Send + 'static {
    fn name(&self) -> &'static str;

    fn run(self: Box<Self>, ctx: FeatureThreadContext) -> Result<(), DaemonError>;
}

/// Builder for constructing a Daemon instance.
pub struct DaemonBuilder {
    config: DaemonConfig,
    features: Vec<Box<dyn DaemonFeature>>,
    threaded_features: Vec<Box<dyn ThreadedFeature>>,
}

impl DaemonBuilder {
    pub fn new() -> Self {
        Self {
            config: DaemonConfig::default(),
            features: Vec::new(),
            threaded_features: Vec::new(),
        }
    }

    pub fn with_config(mut self, config: DaemonConfig) -> Self {
        self.config = config;
        self
    }

    pub fn with_feature(mut self, feature: Box<dyn DaemonFeature>) -> Self {
        self.features.push(feature);
        self
    }

    pub fn with_threaded_feature(mut self, feature: Box<dyn ThreadedFeature>) -> Self {
        self.threaded_features.push(feature);
        self
    }

    pub fn with_work_dir(mut self, work_dir: PathBuf) -> Self {
        self.config.work_dir = work_dir;
        self
    }

    pub fn with_poll_interval(mut self, interval: Duration) -> Self {
        self.config.poll_interval = interval;
        self
    }

    pub fn build(self) -> Result<Daemon, DaemonError> {
        Ok(Daemon::new_from_builder(
            self.config,
            self.features,
            self.threaded_features,
        ))
    }
}

impl Default for DaemonBuilder {
    fn default() -> Self {
        Self::new()
    }
}
