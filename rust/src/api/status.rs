use crate::api::json;
pub use crate::runtime::status::{PublicStatus, read_public_status, read_public_status_from_paths};
use serde::Serialize;
use std::path::Path;

/// Writes a status object to a file atomically.
pub fn write_status<T: Serialize>(path: impl AsRef<Path>, status: &T) -> Result<(), String> {
    json::write_atomic(path, status)
}
