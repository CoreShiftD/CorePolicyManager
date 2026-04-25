// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/

use crate::core::state_view::StateView;
use crate::core::{Event, Intent, LogLevel, SystemService};
use crate::high_level::addon::Addon;
use crate::high_level::api::PreloadSnapshot;
use crate::high_level::identity::{Principal, Request};
use serde::Deserialize;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, serde::Serialize, Deserialize)]
pub struct PreloadConfig {
    pub enabled: bool,
    pub global_max_in_flight: usize,
    pub per_package_warmup_cooldown_ms: u64,
    pub per_package_failure_backoff_ms: u64,
    pub max_paths_per_package: usize,
    pub max_file_size_bytes: u64,
    pub debounce_ms: u64,
}

impl Default for PreloadConfig {
    fn default() -> Self {
        Self {
            enabled: false, // Disabled by default for safe rollout
            global_max_in_flight: 4,
            per_package_warmup_cooldown_ms: 60_000,
            per_package_failure_backoff_ms: 300_000,
            max_paths_per_package: 64,
            max_file_size_bytes: 100 * 1024 * 1024, // 100 MB
            debounce_ms: 500,
        }
    }
}

pub struct PreloadAddon {
    config: PreloadConfig,
    pub dedup_cache: BTreeMap<String, u64>,
    pub negative_cache: BTreeMap<String, u64>,
    pub package_map: BTreeMap<String, std::path::PathBuf>,
    pub package_cache_dirty: bool,
    pub in_flight: BTreeSet<String>,

    last_foreground_pid: i32,
    last_foreground_time: u64,
    pending_foreground_pid: Option<i32>,
    last_foreground_package: Option<String>,

    total_failures: u32,
    auto_disabled: bool,
    events_seen: u64,
    last_cleanup_time: u64,

    /// Last stage where a preload was skipped.
    pub last_skip_stage: Option<String>,
    /// Last skip reason emitted (e.g. `"already_in_flight"`, `"cooldown"`).
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

impl PreloadAddon {
    pub fn new(config: PreloadConfig) -> Self {
        Self {
            config,
            dedup_cache: BTreeMap::new(),
            negative_cache: BTreeMap::new(),
            package_map: BTreeMap::new(),
            package_cache_dirty: false,
            in_flight: BTreeSet::new(),
            last_foreground_pid: -1,
            last_foreground_time: 0,
            pending_foreground_pid: None,
            last_foreground_package: None,
            total_failures: 0,
            auto_disabled: false,
            events_seen: 0,
            last_cleanup_time: 0,
            last_skip_stage: None,
            last_skip_reason: None,
            last_skip_package: None,
            last_discovered_path_count: 0,
            last_warmup_result: None,
            last_warmup_package: None,
        }
    }

    fn submit(&self, intent: Intent) -> Request {
        Request {
            principal: Principal::Addon(102),
            client_id: None,
            cause: crate::core::CauseId(0),
            intent,
        }
    }
}

impl Addon for PreloadAddon {
    fn on_core_event(&mut self, state: &dyn StateView, event: &Event) -> Vec<Request> {
        self.events_seen += 1;

        if self.auto_disabled {
            return Vec::new();
        }

        let mut reqs = Vec::new();
        let now = state.now();

        // Safety: Auto-disable if we hit too many global failures
        if self.total_failures >= 50 {
            self.auto_disabled = true;
            reqs.push(self.submit(Intent::AddonLog {
                addon_id: 102,
                level: LogLevel::Error,
                msg: "CRITICAL: PreloadAddon auto-disabled due to excessive failures".to_string(),
            }));
            return reqs;
        }

        if !self.config.enabled {
            if crate::low_level::sys::path_exists(crate::paths::ENABLE_PRELOAD_PATH) {
                self.config.enabled = true;
                reqs.push(self.submit(Intent::AddonLog {
                    addon_id: 102,
                    level: LogLevel::Info,
                    msg: "Preload enabled via override file".to_string(),
                }));
            } else {
                return Vec::new();
            }
        }

        match event {
            Event::Tick => {
                // 1. Debounce foreground resolution
                if let Some(pid) = self.pending_foreground_pid
                    && now.saturating_sub(self.last_foreground_time) >= self.config.debounce_ms
                {
                    reqs.push(self.submit(Intent::AddonLog {
                        addon_id: 102,
                        level: LogLevel::Debug,
                        msg: format!("debounce_expired pid={} -> ResolveIdentity", pid),
                    }));
                    let payload = serde_json::to_vec(&pid).unwrap_or_default();
                    reqs.push(self.submit(Intent::SystemRequest {
                        request_id: 0,
                        kind: SystemService::ResolveIdentity,
                        payload,
                    }));
                    self.pending_foreground_pid = None;
                }

                // 2. Periodic cache cleanup (every 1 minute)
                if now.saturating_sub(self.last_cleanup_time) >= 60_000 {
                    self.last_cleanup_time = now;
                    // Cleanup dedup cache (entries older than 1 hour)
                    let one_hour_ms = 3_600_000;
                    self.dedup_cache
                        .retain(|_, &mut time| now.saturating_sub(time) < one_hour_ms);

                    // Cleanup negative cache (entries older than 30 mins)
                    let thirty_mins_ms = 1_800_000;
                    self.negative_cache
                        .retain(|_, &mut time| now.saturating_sub(time) < thirty_mins_ms);
                }
            }
            Event::ForegroundChanged { pid } if *pid != self.last_foreground_pid => {
                reqs.push(self.submit(Intent::AddonLog {
                    addon_id: 102,
                    level: LogLevel::Debug,
                    msg: format!("foreground_changed pid={} -> debounce_start", pid),
                }));
                self.pending_foreground_pid = Some(*pid);
                self.last_foreground_time = now;
                self.last_foreground_pid = *pid;
            }
            Event::PackagesChanged => {
                self.package_map.clear();
                self.package_cache_dirty = true;
                self.dedup_cache.clear();
                self.negative_cache.clear();
                reqs.push(self.submit(Intent::AddonLog {
                    addon_id: 102,
                    level: LogLevel::Info,
                    msg: "PKG_EVENT: invalidated caches".to_string(),
                }));
            }
            Event::SystemResponse { kind, payload, .. } => {
                match kind {
                    SystemService::ResolveIdentity => {
                        if let Ok(package_name) = String::from_utf8(payload.clone()) {
                            // Suppress system server and other common system processes
                            if package_name == "android"
                                || package_name == "system"
                                || package_name.contains("com.android.systemui")
                            {
                                self.last_skip_stage = Some("identity_resolution".to_string());
                                self.last_skip_reason = Some("system_package".to_string());
                                self.last_skip_package = Some(package_name.clone());
                                reqs.push(self.submit(Intent::AddonLog {
                                    addon_id: 102,
                                    level: LogLevel::Debug,
                                    msg: format!("preload skip package={} stage=identity reason=system_package", package_name),
                                }));
                                return reqs;
                            }

                            self.last_foreground_package = Some(package_name.clone());
                            reqs.push(self.submit(Intent::AddonLog {
                                addon_id: 102,
                                level: LogLevel::Debug,
                                msg: format!(
                                    "preload_identity_resolved pid={} package={}",
                                    self.last_foreground_pid, package_name
                                ),
                            }));

                            if self.in_flight.contains(&package_name) {
                                self.last_skip_stage = Some("identity_resolution".to_string());
                                self.last_skip_reason = Some("already_in_flight".to_string());
                                self.last_skip_package = Some(package_name.clone());
                                reqs.push(self.submit(Intent::AddonLog {
                                    addon_id: 102,
                                    level: LogLevel::Debug,
                                    msg: format!(
                                        "preload skip package={} stage=identity reason=already_in_flight",
                                        package_name
                                    ),
                                }));
                                return reqs;
                            }

                            if self.in_flight.len() >= self.config.global_max_in_flight {
                                self.last_skip_stage = Some("identity_resolution".to_string());
                                self.last_skip_reason = Some("global_budget_full".to_string());
                                self.last_skip_package = Some(package_name.clone());
                                reqs.push(self.submit(Intent::AddonLog {
                                    addon_id: 102,
                                    level: LogLevel::Warn,
                                    msg: format!(
                                        "preload skip package={} stage=identity reason=global_budget_full count={}",
                                        package_name, self.in_flight.len()
                                    ),
                                }));
                                return reqs;
                            }

                            if let Some(t) = self.negative_cache.get(&package_name) {
                                let elapsed = now.saturating_sub(*t);
                                if elapsed < self.config.per_package_failure_backoff_ms {
                                    self.last_skip_stage = Some("identity_resolution".to_string());
                                    self.last_skip_reason = Some("failure_backoff".to_string());
                                    self.last_skip_package = Some(package_name.clone());
                                    reqs.push(self.submit(Intent::AddonLog {
                                        addon_id: 102,
                                        level: LogLevel::Debug,
                                        msg: format!("preload skip package={} stage=identity reason=failure_backoff remaining_ms={}", package_name, self.config.per_package_failure_backoff_ms - elapsed),
                                    }));
                                    return reqs;
                                }
                            }

                            if let Some(last_warmup) = self.dedup_cache.get(&package_name) {
                                let elapsed = now.saturating_sub(*last_warmup);
                                if elapsed < self.config.per_package_warmup_cooldown_ms {
                                    self.last_skip_stage = Some("identity_resolution".to_string());
                                    self.last_skip_reason = Some("cooldown".to_string());
                                    self.last_skip_package = Some(package_name.clone());
                                    reqs.push(self.submit(Intent::AddonLog {
                                        addon_id: 102,
                                        level: LogLevel::Debug,
                                        msg: format!("preload skip package={} stage=identity reason=cooldown remaining_ms={}", package_name, self.config.per_package_warmup_cooldown_ms - elapsed),
                                    }));
                                    return reqs;
                                }
                            }

                            if let Some(base_dir) = self.package_map.get(&package_name) {
                                reqs.push(self.submit(Intent::AddonLog {
                                    addon_id: 102,
                                    level: LogLevel::Debug,
                                    msg: format!("package_map_hit package={} -> DiscoverPaths", package_name),
                                }));
                                let payload = serde_json::to_vec(&(
                                    package_name.clone(),
                                    base_dir.to_string_lossy().into_owned(),
                                ))
                                .unwrap_or_default();
                                reqs.push(self.submit(Intent::SystemRequest {
                                    request_id: 0,
                                    kind: SystemService::DiscoverPaths,
                                    payload,
                                }));
                            } else {
                                reqs.push(self.submit(Intent::AddonLog {
                                    addon_id: 102,
                                    level: LogLevel::Debug,
                                    msg: format!("package_map_miss package={} -> ResolveDirectory", package_name),
                                }));
                                let payload = package_name.clone().into_bytes();
                                reqs.push(self.submit(Intent::SystemRequest {
                                    request_id: 0,
                                    kind: SystemService::ResolveDirectory,
                                    payload,
                                }));
                            }
                        }
                    }
                    SystemService::ResolveDirectory => {
                        if let Ok((package_name, base_dir)) =
                            serde_json::from_slice::<(String, String)>(payload)
                        {
                            self.package_map
                                .insert(package_name.clone(), std::path::PathBuf::from(&base_dir));
                            self.package_cache_dirty = false;
                            let payload =
                                serde_json::to_vec(&(package_name, base_dir)).unwrap_or_default();
                            reqs.push(self.submit(Intent::SystemRequest {
                                request_id: 0,
                                kind: SystemService::DiscoverPaths,
                                payload,
                            }));
                        }
                    }
                    SystemService::DiscoverPaths => {
                        if let Ok((package_name, mut paths)) =
                            serde_json::from_slice::<(String, Vec<String>)>(payload)
                        {
                            if paths.is_empty() {
                                self.last_skip_stage = Some("path_discovery".to_string());
                                self.last_skip_reason = Some("no_paths_discovered".to_string());
                                self.last_skip_package = Some(package_name.clone());
                                self.negative_cache.insert(package_name.clone(), now);
                                reqs.push(self.submit(Intent::AddonLog {
                                    addon_id: 102,
                                    level: LogLevel::Warn,
                                    msg: format!(
                                        "preload skip package={} stage=discovery reason=no_paths",
                                        package_name
                                    ),
                                }));
                                return reqs;
                            }

                            self.last_discovered_path_count = paths.len();
                            paths.truncate(self.config.max_paths_per_package);
                            self.in_flight.insert(package_name.clone());

                            let mut task_payload = vec![1u8]; // Type 1 = Warmup
                            task_payload.extend(serde_json::to_vec(&paths).unwrap_or_default());

                            reqs.push(self.submit(Intent::AddonTask {
                                addon_id: 102,
                                key: format!("warmup:{}", package_name),
                                payload: task_payload,
                            }));
                            reqs.push(self.submit(Intent::AddonLog {
                                addon_id: 102,
                                level: LogLevel::Info,
                                msg: format!(
                                    "preload warmup task queued: package={} paths={} in_flight={}",
                                    package_name,
                                    paths.len(),
                                    self.in_flight.len()
                                ),
                            }));
                        }
                    }
                }
            }
            Event::SystemFailure { kind, err, .. } if *kind == SystemService::ResolveIdentity => {
                self.last_skip_stage = Some("identity_resolution".to_string());
                self.last_skip_reason = Some(format!("system_failure: {}", err));
                reqs.push(self.submit(Intent::AddonLog {
                    addon_id: 102,
                    level: LogLevel::Warn,
                    msg: format!(
                        "identity resolution failed pid={} err={}",
                        self.last_foreground_pid, err
                    ),
                }));
            }
            Event::AddonCompleted {
                addon_id,
                key,
                payload,
            } if *addon_id == 102
                && let Some(package) = key.strip_prefix("warmup:") =>
            {
                self.in_flight.remove(package);
                self.dedup_cache.insert(package.to_string(), now);
                self.last_warmup_package = Some(package.to_string());

                if let Ok((bytes, duration_ms)) = serde_json::from_slice::<(u64, u64)>(payload) {
                    let result_summary = format!(
                        "package={} bytes={} duration_ms={}",
                        package, bytes, duration_ms
                    );
                    self.last_warmup_result = Some(result_summary.clone());
                    reqs.push(self.submit(Intent::AddonLog {
                        addon_id: 102,
                        level: LogLevel::Info,
                        msg: format!("preload_success {}", result_summary),
                    }));
                }
            }
            Event::AddonFailed { addon_id, key, err } if *addon_id == 102 => {
                let package = key
                    .strip_prefix("warmup:")
                    .or_else(|| key.strip_prefix("resolve_dir:"))
                    .or_else(|| key.strip_prefix("discover_paths:"))
                    .unwrap_or("unknown");

                self.in_flight.remove(package);
                self.negative_cache.insert(package.to_string(), now);
                self.total_failures += 1;
                reqs.push(self.submit(Intent::AddonLog {
                    addon_id: 102,
                    level: LogLevel::Warn,
                    msg: format!(
                        "preload_task_failed stage={} package={} err={} total_fails={}",
                        key, package, err, self.total_failures
                    ),
                }));
            }
            _ => {}
        }
        reqs
    }

    fn preload_snapshot(&self) -> Option<crate::high_level::api::PreloadSnapshot> {
        Some(self.status_snapshot())
    }
}

impl PreloadAddon {
    /// Return a pure policy-state snapshot.
    ///
    /// No filesystem probes, no daemon context, no serialization.
    /// The runtime layer is responsible for assembling the full
    /// [`DaemonStatusReport`] by combining this snapshot with live OS checks.
    pub fn status_snapshot(&self) -> PreloadSnapshot {
        PreloadSnapshot {
            enabled: self.config.enabled,
            last_foreground_pid: self.last_foreground_pid,
            last_foreground_package: self.last_foreground_package.clone(),
            package_cache_count: self.package_map.len(),
            package_cache_dirty: self.package_cache_dirty,
            dedup_cache_count: self.dedup_cache.len(),
            negative_cache_count: self.negative_cache.len(),
            in_flight_count: self.in_flight.len(),
            in_flight_packages: self.in_flight.iter().cloned().collect(),
            total_failures: self.total_failures,
            auto_disabled: self.auto_disabled,
            events_seen: self.events_seen,
            last_skip_stage: self.last_skip_stage.clone(),
            last_skip_reason: self.last_skip_reason.clone(),
            last_skip_package: self.last_skip_package.clone(),
            last_discovered_path_count: self.last_discovered_path_count,
            last_warmup_result: self.last_warmup_result.clone(),
            last_warmup_package: self.last_warmup_package.clone(),
        }
    }
}
