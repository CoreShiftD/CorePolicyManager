use crate::core::{Action, Module};

use std::collections::{BTreeMap, BTreeSet};

pub struct WarmupState {
    pub dedup_cache: BTreeMap<String, u64>,
    pub negative_cache: BTreeMap<String, u64>,
    pub package_map: BTreeMap<String, std::path::PathBuf>,
    pub in_flight: BTreeSet<String>,
    pub hash: u64,
}

impl Default for WarmupState {
    fn default() -> Self {
        Self::new()
    }
}

impl WarmupState {
    pub fn new() -> Self {
        Self {
            dedup_cache: BTreeMap::new(),
            negative_cache: BTreeMap::new(),
            package_map: BTreeMap::new(),
            in_flight: BTreeSet::new(),
            hash: 0,
        }
    }
}

pub struct WarmupModule;

impl Module for WarmupModule {
    fn handle(
        &self,
        state: &dyn crate::core::state_view::StateView,
        action: &Action,
    ) -> Vec<Action> {
        let mut actions = Vec::new();
        match action {
            Action::AppForegrounded { pid } => {
                actions.push(Action::RequestPackageName { pid: *pid });
            }
            Action::PackageNameResolved {
                pid: _,
                package_name,
            } => {
                actions.push(Action::ResolvePackage {
                    package_name: package_name.clone(),
                });
            }
            Action::ResolvePackage { package_name } => {
                if state.is_warmup_in_flight(package_name) {
                    return actions; // Already in-flight
                }

                let now = state.now();
                if let Some(t) = state.warmup_negative_cached(package_name)
                    && now.saturating_sub(t) < 300_000
                {
                    return actions;
                }

                if let Some(last_warmup) = state.warmup_last_run(package_name)
                    && now.saturating_sub(last_warmup) < 60_000
                {
                    return actions;
                }

                if let Some(base_dir) = state.warmup_base_dir(package_name) {
                    actions.push(Action::DiscoverPaths {
                        package_name: package_name.clone(),
                        base_dir,
                    });
                } else {
                    actions.push(Action::RequestPackageDir {
                        package_name: package_name.clone(),
                    });
                }
            }
            Action::PackageDirResolved {
                package_name,
                base_dir,
            } => {
                actions.push(Action::DiscoverPaths {
                    package_name: package_name.clone(),
                    base_dir: base_dir.clone(),
                });
            }
            Action::DiscoverPaths {
                package_name,
                base_dir,
            } => {
                actions.push(Action::RequestPackagePaths {
                    package_name: package_name.clone(),
                    base_dir: base_dir.clone(),
                });
            }
            Action::PackagePathsDiscovered {
                package_name,
                paths,
            } => {
                actions.push(Action::ScheduleWarmup {
                    package_name: package_name.clone(),
                    paths: paths.clone(),
                });
            }
            Action::ScheduleWarmup {
                package_name,
                paths,
            } => {
                for path in paths {
                    actions.push(Action::DispatchWarmupChunk {
                        package_name: package_name.clone(),
                        path: path.clone(),
                    });
                }
            }
            _ => {}
        }
        actions
    }

    fn handle_event(
        &self,
        _state: &dyn crate::core::state_view::StateView,
        event: &crate::core::Event,
    ) -> Vec<Action> {
        let mut actions = Vec::new();
        match event {
            crate::core::Event::AppForegrounded { pid } => {
                actions.push(Action::AppForegrounded { pid: *pid });
            }
            crate::core::Event::PackagesChanged => {
                actions.push(Action::PackagesChanged);
            }
            crate::core::Event::PackageNameResolved { pid, package_name } => {
                actions.push(Action::PackageNameResolved {
                    pid: *pid,
                    package_name: package_name.clone(),
                });
            }
            crate::core::Event::PackageDirResolved {
                package_name,
                base_dir,
            } => {
                actions.push(Action::PackageDirResolved {
                    package_name: package_name.clone(),
                    base_dir: base_dir.clone(),
                });
            }
            crate::core::Event::PackagePathsDiscovered {
                package_name,
                paths,
            } => {
                actions.push(Action::PackagePathsDiscovered {
                    package_name: package_name.clone(),
                    paths: paths.clone(),
                });
            }
            crate::core::Event::WarmupCompleted {
                package,
                bytes: _,
                duration_ms: _,
            } => {
                actions.push(Action::CompleteWarmup {
                    package_name: package.clone(),
                });
            }
            crate::core::Event::PackageResolutionFailed { pid: _, err } => {
                // If package resolution fails, we can't tie it to a package name in warmup state.
                actions.push(Action::EmitLog {
                    owner: crate::core::WARMUP_OWNER,
                    level: crate::core::LogLevel::Warn,
                    event: crate::core::LogEvent::Error {
                        id: 0,
                        err: err.clone(),
                    },
                });
            }
            crate::core::Event::PackageDirResolutionFailed { package_name, err }
            | crate::core::Event::PackagePathsDiscoveryFailed { package_name, err }
            | crate::core::Event::WarmupFailed { package_name, err } => {
                actions.push(Action::HandleWarmupFailure {
                    package_name: package_name.clone(),
                    err: err.clone(),
                });
            }
            _ => {}
        }
        actions
    }
}
