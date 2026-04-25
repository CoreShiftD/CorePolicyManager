// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/

//! Runtime status assembly.
//!
//! This module is the single place responsible for building a
//! [`DaemonStatusReport`].  It combines:
//!
//! - daemon operational context (mode, socket path) passed in by the caller
//! - live filesystem probes via `low_level::sys::path_exists`
//! - a pure policy-state snapshot from the PreloadAddon
//! - inotify watch registration results stored on the addon
//!
//! No addon code performs filesystem probes.  No IPC code knows preload
//! internals.  The CLI only pretty-prints the typed report.

use crate::high_level::addon::Addon;
use crate::high_level::api::{DaemonStatusReport, WatchedPathStatus};
use crate::low_level::sys::path_exists;

/// Assemble a [`DaemonStatusReport`] from live daemon state.
///
/// # Parameters
/// - `mode`: daemon mode string (`"normal"`, `"preload"`, `"record"`)
/// - `socket_path`: path of the bound Unix-domain socket
/// - `preload_addon`: trait-object reference to the PreloadAddon if loaded,
///   or `None`.  The addon's `preload_snapshot()` method is called to obtain
///   a pure policy-state snapshot; no filesystem probes occur inside the addon.
/// - `watch_registrations`: inotify registration results collected at startup
///
/// Filesystem probes (`path_exists`) are performed here in the runtime layer,
/// not inside the addon.
pub fn assemble_daemon_status(
    mode: &str,
    socket_path: &str,
    preload_addon: Option<&dyn Addon>,
    watch_registrations: &[WatchedPathStatus],
) -> DaemonStatusReport {
    let enable_preload_file_exists = path_exists(crate::paths::ENABLE_PRELOAD_PATH);
    let foreground_path_exists = path_exists("/dev/cpuset/top-app/cgroup.procs");

    DaemonStatusReport {
        mode: mode.to_string(),
        socket_path: socket_path.to_string(),
        preload_addon_loaded: preload_addon.is_some(),
        enable_preload_file_exists,
        enable_preload_path: crate::paths::ENABLE_PRELOAD_PATH.to_string(),
        foreground_path_exists,
        watched_paths: watch_registrations.to_vec(),
        preload: preload_addon.and_then(|a| a.preload_snapshot()),
    }
}
