use crate::core::state_view::StateView;
use crate::core::{Event, Intent, SystemService};
use crate::high_level::addon::Addon;
use crate::high_level::identity::{Principal, Request};
use std::collections::{BTreeMap, BTreeSet};

pub struct PreloadAddon {
    pub dedup_cache: BTreeMap<String, u64>,
    pub negative_cache: BTreeMap<String, u64>,
    pub package_map: BTreeMap<String, std::path::PathBuf>,
    pub in_flight: BTreeSet<String>,
}

impl PreloadAddon {
    pub fn new() -> Self {
        Self {
            dedup_cache: BTreeMap::new(),
            negative_cache: BTreeMap::new(),
            package_map: BTreeMap::new(),
            in_flight: BTreeSet::new(),
        }
    }

    fn submit(&self, intent: Intent) -> Request {
        Request {
            principal: Principal::Addon(102), // Preload ID
            client_id: None,
            cause: crate::core::CauseId(0), // Will be assigned by core
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
        let mut reqs = Vec::new();
        match event {
            Event::ForegroundChanged { pid } => {
                let payload = serde_json::to_vec(pid).unwrap_or_default();
                reqs.push(self.submit(Intent::SystemRequest {
                    request_id: 0, // Using CauseId in core is better but for now 0
                    kind: SystemService::ResolveIdentity,
                    payload,
                }));
            }
            Event::PackagesChanged => {
                reqs.push(self.submit(Intent::PackagesChanged));
            }
            Event::SystemResponse { request_id: _, kind, payload } => {
                match kind {
                    SystemService::ResolveIdentity => {
                        if let Ok(package_name) = String::from_utf8(payload.clone()) {
                             if self.in_flight.contains(&package_name) {
                                return reqs;
                            }

                            let now = state.now();
                            if let Some(t) = self.negative_cache.get(&package_name)
                                && now.saturating_sub(*t) < 300_000
                            {
                                return reqs;
                            }

                            if let Some(last_warmup) = self.dedup_cache.get(&package_name)
                                && now.saturating_sub(*last_warmup) < 60_000
                            {
                                return reqs;
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
                         if let Ok((package_name, base_dir)) = serde_json::from_slice::<(String, String)>(&payload) {
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
                        if let Ok((package_name, paths)) = serde_json::from_slice::<(String, Vec<String>)>(&payload) {
                            self.in_flight.insert(package_name.clone());
                            // AddonTask is still the correct way for warmup work
                            let mut task_payload = vec![1u8]; // Type 1 = Warmup
                            task_payload.extend(serde_json::to_vec(&paths).unwrap_or_default());

                            reqs.push(self.submit(Intent::AddonTask {
                                addon_id: 102,
                                key: format!("warmup:{}", package_name),
                                payload: task_payload,
                            }));
                        }
                    }
                }
            }
            Event::AddonCompleted { addon_id, key, .. } if *addon_id == 102 => {
                if key.starts_with("warmup:") {
                    let package = &key[7..];
                    self.in_flight.remove(package);
                    self.dedup_cache.insert(package.to_string(), state.now());
                }
            }
            Event::AddonFailed { addon_id, key, .. } if *addon_id == 102 => {
                if key.starts_with("warmup:") {
                    let package = &key[7..];
                    self.in_flight.remove(package);
                    self.negative_cache.insert(package.to_string(), state.now());
                } else if key.starts_with("resolve_dir:") {
                    let package = &key[12..];
                    self.in_flight.remove(package);
                    self.negative_cache.insert(package.to_string(), state.now());
                } else if key.starts_with("discover_paths:") {
                    let package = &key[15..];
                    self.in_flight.remove(package);
                    self.negative_cache.insert(package.to_string(), state.now());
                }
            }
            _ => {}
        }
        reqs
    }
}
