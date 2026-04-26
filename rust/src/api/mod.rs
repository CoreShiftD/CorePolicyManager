pub mod config;
pub mod daemon;
pub mod json;
pub mod resolver;
pub mod status;

// Public re-exports for the daemon framework
pub use config::{RuntimeConfig, RuntimeFeature, all_features, daemon_config_from_features};
pub use daemon::{
    Daemon, DaemonBuilder, DaemonConfig, DaemonContext, DaemonError, DaemonFeature,
    FeatureThreadContext, ThreadedFeature,
};
pub use resolver::{ForegroundEvent, ForegroundResolver, ForegroundSnapshot};
pub use status::{PublicStatus, read_public_status, read_public_status_from_paths};
