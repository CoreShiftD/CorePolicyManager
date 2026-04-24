use crate::core::state_view::StateView;
use crate::core::{Event, Intent, SystemService, LogLevel};
use crate::high_level::addon::Addon;
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
    pub in_flight: BTreeSet<String>,
    
    last_foreground_pid: i32,
    last_foreground_time: u64,
    pending_foreground_pid: Option<i32>,
    
    total_failures: u32,
    auto_disabled: bool,
}

impl PreloadAddon {
    pub fn new(config: PreloadConfig) -> Self {
        Self {
            config,
            dedup_cache: BTreeMap::new(),
            negative_cache: BTreeMap::new(),
            package_map: BTreeMap::new(),
            in_flight: BTreeSet::new(),
            last_foreground_pid: -1,
            last_foreground_time: 0,
            pending_foreground_pid: None,
            total_failures: 0,
            auto_disabled: false,
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
    fn on_core_event(
        &mut self,
        state: &dyn StateView,
        event: &Event,
    ) -> Vec<Request> {
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
            if std::path::Path::new(crate::paths::ENABLE_PRELOAD_PATH).exists() {
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
                if let Some(pid) = self.pending_foreground_pid {
                    if now.saturating_sub(self.last_foreground_time) >= self.config.debounce_ms {
                        let payload = serde_json::to_vec(&pid).unwrap_or_default();
                        reqs.push(self.submit(Intent::SystemRequest {
                            request_id: 0,
                            kind: SystemService::ResolveIdentity,
                            payload,
                        }));
                        self.pending_foreground_pid = None;
                    }
                }
            }
            Event::ForegroundChanged { pid } => {
                if *pid != self.last_foreground_pid {
                    self.pending_foreground_pid = Some(*pid);
                    self.last_foreground_time = now;
                    self.last_foreground_pid = *pid;
                }
            }
            Event::PackagesChanged => {
                self.package_map.clear();
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
                            reqs.push(self.submit(Intent::AddonLog {
                                addon_id: 102,
                                level: LogLevel::Debug,
                                msg: format!("preload foreground pid={} package={}", self.last_foreground_pid, package_name),
                            }));

                             if self.in_flight.contains(&package_name) {
                                reqs.push(self.submit(Intent::AddonLog {
                                    addon_id: 102,
                                    level: LogLevel::Debug,
                                    msg: format!("preload skip package={} reason=already_in_flight", package_name),
                                }));
                                return reqs;
                            }
                            
                            if self.in_flight.len() >= self.config.global_max_in_flight {
                                reqs.push(self.submit(Intent::AddonLog {
                                    addon_id: 102,
                                    level: LogLevel::Warn,
                                    msg: format!("preload skip package={} reason=global_budget_full", package_name),
                                }));
                                return reqs;
                            }

                            if let Some(t) = self.negative_cache.get(&package_name) {
                                let elapsed = now.saturating_sub(*t);
                                if elapsed < self.config.per_package_failure_backoff_ms {
                                    reqs.push(self.submit(Intent::AddonLog {
                                        addon_id: 102,
                                        level: LogLevel::Debug,
                                        msg: format!("preload skip package={} reason=failure_backoff remaining_ms={}", package_name, self.config.per_package_failure_backoff_ms - elapsed),
                                    }));
                                    return reqs;
                                }
                            }

                            if let Some(last_warmup) = self.dedup_cache.get(&package_name) {
                                let elapsed = now.saturating_sub(*last_warmup);
                                if elapsed < self.config.per_package_warmup_cooldown_ms {
                                    reqs.push(self.submit(Intent::AddonLog {
                                        addon_id: 102,
                                        level: LogLevel::Debug,
                                        msg: format!("preload skip package={} reason=cooldown remaining_ms={}", package_name, self.config.per_package_warmup_cooldown_ms - elapsed),
                                    }));
                                    return reqs;
                                }
                            }

                            if let Some(base_dir) = self.package_map.get(&package_name) {
                                let payload = serde_json::to_vec(&(package_name.clone(), base_dir.to_string_lossy().into_owned())).unwrap_or_default();
                                reqs.push(self.submit(Intent::SystemRequest {
                                    request_id: 0,
                                    kind: SystemService::DiscoverPaths,
                                    payload,
                                }));
                            } else {
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
                         if let Ok((package_name, base_dir)) = serde_json::from_slice::<(String, String)>(payload) {
                             self.package_map.insert(package_name.clone(), std::path::PathBuf::from(&base_dir));
                             let payload = serde_json::to_vec(&(package_name, base_dir)).unwrap_or_default();
                             reqs.push(self.submit(Intent::SystemRequest {
                                request_id: 0,
                                kind: SystemService::DiscoverPaths,
                                payload,
                            }));
                         }
                    }
                    SystemService::DiscoverPaths => {
                        if let Ok((package_name, mut paths)) = serde_json::from_slice::<(String, Vec<String>)>(payload) {
                            if paths.is_empty() {
                                self.negative_cache.insert(package_name.clone(), now);
                                reqs.push(self.submit(Intent::AddonLog {
                                    addon_id: 102,
                                    level: LogLevel::Warn,
                                    msg: format!("preload fail package={} reason=no_paths_discovered", package_name),
                                }));
                                return reqs;
                            }

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
                                msg: format!("preload start package={} paths={}", package_name, paths.len()),
                            }));
                        }
                    }
                }
            }
            Event::AddonCompleted { addon_id, key, payload } if *addon_id == 102 => {
                if key.starts_with("warmup:") {
                    let package = &key[7..];
                    self.in_flight.remove(package);
                    self.dedup_cache.insert(package.to_string(), now);
                    
                    if let Ok((bytes, duration_ms)) = serde_json::from_slice::<(u64, u64)>(payload) {
                        reqs.push(self.submit(Intent::AddonLog {
                            addon_id: 102,
                            level: LogLevel::Info,
                            msg: format!("preload done package={} bytes={} duration_ms={}", package, bytes, duration_ms),
                        }));
                    }
                }
            }
            Event::AddonFailed { addon_id, key, err } if *addon_id == 102 => {
                let package = if key.starts_with("warmup:") {
                    &key[7..]
                } else if key.starts_with("resolve_dir:") {
                    &key[12..]
                } else if key.starts_with("discover_paths:") {
                    &key[15..]
                } else {
                    "unknown"
                };
                
                self.in_flight.remove(package);
                self.negative_cache.insert(package.to_string(), now);
                self.total_failures += 1;
                reqs.push(self.submit(Intent::AddonLog {
                    addon_id: 102,
                    level: LogLevel::Warn,
                    msg: format!("preload fail package={} reason={} err={} total_fails={}", package, key, err, self.total_failures),
                }));
            }
            _ => {}
        }
        reqs
    }
}
