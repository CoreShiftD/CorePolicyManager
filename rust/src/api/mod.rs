//! Public daemon framework surface.
//!
//! ```compile_fail
//! use coreshift_policy::api::features::AppIndexFeature;
//! ```
//!
//! ```compile_fail
//! use coreshift_policy::api::builtins::PreloadFeature;
//! ```

pub mod config;
pub mod daemon;
pub mod json;
pub mod resolver;
pub mod status;

pub use config::DaemonConfig;
pub use daemon::{
    Daemon, DaemonBuilder, DaemonContext, DaemonError, DaemonFeature, FeatureThreadContext,
    ThreadedFeature,
};
pub use resolver::{ForegroundEvent, ForegroundResolver, ForegroundSnapshot};
