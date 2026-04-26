pub use crate::daemon::foreground::{ForegroundResolver, ForegroundSnapshot};

/// Represents a change in the foreground application.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ForegroundEvent {
    pub previous: ForegroundSnapshot,
    pub current: ForegroundSnapshot,
}
