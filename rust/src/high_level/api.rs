// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/

use crate::core::{CancelPolicy, ExecPolicy, ExecSpec};
use serde::{Deserialize, Serialize};

/// Strict command schema defining finite execution space.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Command {
    Cmd { service: String, args: Vec<String> },
    Dumpsys { service: String, args: Vec<String> },
    GetResult { id: u64 },
    Cancel { id: u64 },
    PreloadStatus,
}

// ---------------------------------------------------------------------------
// Typed IPC response types
// ---------------------------------------------------------------------------

/// Registration status for a single inotify-watched path.
///
/// Populated by the runtime after inotify setup; the addon only stores the
/// result, it does not perform the registration itself.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WatchedPathStatus {
    pub path: String,
    pub registered: bool,
}

/// Full daemon status report returned by `preload-status`.
///
/// Assembled by the runtime layer, which combines:
/// - daemon operational context (mode, socket path)
/// - live filesystem probes (control file, foreground path)
/// - a snapshot of the PreloadAddon's policy state
///
/// This is the canonical wire type for the `PreloadStatus` IPC response.
/// It is serialized to JSON and framed by `mid_level::ipc`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DaemonStatusReport {
    /// Seconds since daemon startup.
    pub uptime_secs: u64,
    /// Daemon mode: `"normal"`, `"preload"`, or `"record"`.
    pub mode: String,
    /// Path of the Unix-domain socket the daemon is listening on.
    pub socket_path: String,
    /// Whether the preload addon is loaded in this daemon instance.
    pub preload_addon_loaded: bool,
    /// Whether the `enable_preload` control file currently exists on disk.
    pub enable_preload_file_exists: bool,
    /// Path of the `enable_preload` control file.
    pub enable_preload_path: String,
    /// Whether `/dev/cpuset/top-app/cgroup.procs` exists on this device.
    pub foreground_path_exists: bool,
    /// Watched inotify paths and their registration status.
    pub watched_paths: Vec<WatchedPathStatus>,
    /// Live inotify runtime state, if inotify setup reached the runtime path.
    pub inotify: Option<InotifyStatus>,
    /// Preload addon policy state snapshot, if the addon is loaded.
    pub preload: Option<PreloadSnapshot>,
}

/// Runtime-owned inotify diagnostics returned by `preload-status`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct InotifyStatus {
    /// Whether the inotify file descriptor is active in the runtime.
    pub fd_active: bool,
    /// Current cgroup watch descriptor, if registered.
    pub wd_cgroup: Option<i32>,
    /// Current packages.xml watch descriptor, if registered.
    pub wd_pkg_xml: Option<i32>,
    /// Current packages.list watch descriptor, if registered.
    pub wd_pkg_list: Option<i32>,
    /// Number of raw inotify records decoded since startup.
    pub events_seen: u64,
    /// Last raw kernel mask observed.
    pub last_raw_mask: Option<u32>,
    /// Last semantic source mapped from a watch descriptor.
    pub last_source: Option<String>,
    /// Last foreground PID emitted to core, if any.
    pub last_foreground_pid: Option<i32>,
    /// Whether package resolution state must be refreshed lazily.
    pub package_cache_dirty: bool,
    /// Last exceptional event such as queue overflow, ignored watch, or unmount.
    pub last_exception: Option<String>,
}

/// Pure policy-state snapshot from the PreloadAddon.
///
/// Contains only what the addon itself tracks: no filesystem probes,
/// no daemon context, no serialization logic.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PreloadSnapshot {
    /// Whether the addon's `config.enabled` flag is set.
    pub enabled: bool,
    /// Last foreground PID seen (`-1` if none yet).
    pub last_foreground_pid: i32,
    /// Last foreground package name resolved, if any.
    pub last_foreground_package: Option<String>,
    /// Number of packages in the resolved package map.
    pub package_cache_count: usize,
    /// Whether the addon must refresh package mappings before resolution.
    pub package_cache_dirty: bool,
    /// Number of entries in the warmup dedup cache.
    pub dedup_cache_count: usize,
    /// Number of entries in the failure negative cache.
    pub negative_cache_count: usize,
    /// Number of warmups currently in flight.
    pub in_flight_count: usize,
    /// Packages currently in flight.
    pub in_flight_packages: Vec<String>,
    /// Total warmup failures since daemon start.
    pub total_failures: u32,
    /// Whether the addon has been auto-disabled due to excessive failures.
    pub auto_disabled: bool,
    /// Number of core events seen by the addon.
    pub events_seen: u64,
    /// Last stage where a preload was skipped.
    pub last_skip_stage: Option<String>,
    /// Last skip reason (e.g. `"already_in_flight"`, `"cooldown"`).
    pub last_skip_reason: Option<String>,
    /// Last package skipped.
    pub last_skip_package: Option<String>,
    /// Last discovered path count for a package.
    pub last_discovered_path_count: usize,
    /// Last warmup result summary (e.g. `"package=com.foo bytes=1234 duration_ms=50"`).
    pub last_warmup_result: Option<String>,
    /// Last package warmed up.
    pub last_warmup_package: Option<String>,
}

use crate::high_level::android::ExecConfig;

impl Command {
    pub fn map_to_exec(self) -> (ExecSpec, ExecPolicy) {
        match self {
            Command::Cmd { service, args } => {
                let args_refs: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
                let cfg = ExecConfig {
                    timeout_ms: None,
                    kill_grace_ms: 1000,
                    cancel: CancelPolicy::Graceful,
                    max_output: 1024 * 1024,
                };
                let req = crate::high_level::android::cmd(&service, &args_refs, cfg);
                (
                    ExecSpec {
                        argv: req.argv,
                        stdin: req.stdin,
                        capture_stdout: req.capture_stdout,
                        capture_stderr: req.capture_stderr,
                        max_output: req.max_output,
                    },
                    ExecPolicy {
                        timeout_ms: req.timeout_ms,
                        kill_grace_ms: req.kill_grace_ms,
                        cancel: req.cancel,
                    },
                )
            }
            Command::Dumpsys { service, args } => {
                let args_refs: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
                let cfg = ExecConfig {
                    timeout_ms: None,
                    kill_grace_ms: 1000,
                    cancel: CancelPolicy::Graceful,
                    max_output: 4 * 1024 * 1024,
                };
                let req = crate::high_level::android::dumpsys(&service, &args_refs, cfg);
                (
                    ExecSpec {
                        argv: req.argv,
                        stdin: req.stdin,
                        capture_stdout: req.capture_stdout,
                        capture_stderr: req.capture_stderr,
                        max_output: req.max_output,
                    },
                    ExecPolicy {
                        timeout_ms: req.timeout_ms,
                        kill_grace_ms: req.kill_grace_ms,
                        cancel: req.cancel,
                    },
                )
            }
            Command::GetResult { .. } | Command::Cancel { .. } | Command::PreloadStatus => {
                unreachable!()
            }
        }
    }
}
