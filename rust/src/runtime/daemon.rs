use crate::features::preload::PreloadFeature;
use crate::paths::{BLACKLIST_FILE, CPUSET_TOP_APP};
use crate::runtime::foreground::ForegroundResolver;
use crate::runtime::indexer::AppIndexer;
use crate::runtime::logging;
use crate::runtime::pressure::refresh_pressure_metrics;
use crate::runtime::signals::SHUTDOWN;
use crate::runtime::status::DaemonStatus;
use coreshift_lowlevel::inotify::{
    InotifyEvent, MODIFY_MASK, PACKAGE_FILE_MASK, add_watch, read_events,
};
use coreshift_lowlevel::reactor::{Event, Fd, Reactor, Token};
use std::fs;
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::Ordering;
use std::time::{SystemTime, UNIX_EPOCH};

pub struct Daemon {
    reactor: Reactor,
    status: DaemonStatus,
    last_written_status: DaemonStatus,
    foreground: Option<ForegroundResolver>,
    preload: Option<PreloadFeature>,
    indexer: Arc<AppIndexer>,
    event_buffer: Vec<Event>,
    inotify_token: Option<Token>,
    inotify_fd: Option<Fd>,
    cpuset_watch: Option<i32>,
    package_list_watch: Option<i32>,
}

impl Daemon {
    pub fn new(preload_only: bool) -> Self {
        let mut reactor = Reactor::new().expect("Failed to create reactor");
        let mut status = DaemonStatus {
            daemon: crate::runtime::status::DaemonInfo {
                alive: true,
                mode: if preload_only {
                    "preload".to_string()
                } else {
                    "default".to_string()
                },
                started_ms: SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_millis() as u64,
                warnings: Vec::new(),
            },
            ..Default::default()
        };
        status.features.profile.enabled = true;

        refresh_pressure_metrics(&mut status.pressure);

        let indexer = Arc::new(AppIndexer::new());
        indexer.rebuild(crate::features::preload::RuntimeAbi::current());
        status.app_index = indexer.snapshot();

        // Startup Diagnostics
        if !Path::new(CPUSET_TOP_APP).exists() {
            logging::warn(&format!("{} unavailable. Degrading.", CPUSET_TOP_APP));
        }

        let mut inotify_token = None;
        let mut inotify_fd = None;
        let mut cpuset_watch = None;
        let mut package_list_watch = None;
        let foreground = match reactor.setup_inotify() {
            Ok(fd) => {
                inotify_token = reactor.inotify_token;
                inotify_fd = Some(fd);
                let blacklist = if let Ok(content) = fs::read_to_string(BLACKLIST_FILE) {
                    serde_json::from_str::<serde_json::Value>(&content)
                        .map(|v| {
                            v["packages"]
                                .as_array()
                                .map(|arr| {
                                    arr.iter()
                                        .filter_map(|v| v.as_str().map(|s| s.to_string()))
                                        .collect()
                                })
                                .unwrap_or_else(Vec::new)
                        })
                        .unwrap_or_else(|_| vec!["com.android.systemui".to_string()])
                } else {
                    vec!["com.android.systemui".to_string()]
                };

                let inotify_fd_ref = inotify_fd.as_ref().unwrap();

                match add_watch(inotify_fd_ref, CPUSET_TOP_APP, MODIFY_MASK) {
                    Ok(wd) => cpuset_watch = Some(wd),
                    Err(e) => {
                        logging::error(&format!(
                            "Failed to add watch on {}: {}",
                            CPUSET_TOP_APP, e
                        ));
                    }
                }
                match add_watch(
                    inotify_fd_ref,
                    "/data/system/packages.list",
                    PACKAGE_FILE_MASK,
                ) {
                    Ok(wd) => package_list_watch = Some(wd),
                    Err(e) => {
                        logging::warn(&format!(
                            "Failed to watch /data/system/packages.list: {}",
                            e
                        ));
                    }
                }

                if cpuset_watch.is_some() {
                    Some(ForegroundResolver::new(blacklist))
                } else {
                    None
                }
            }
            Err(e) => {
                logging::error(&format!("Failed to setup inotify: {}", e));
                None
            }
        };

        let preload = if foreground.is_some() {
            Some(PreloadFeature::new())
        } else {
            None
        };

        let mut d = Self {
            reactor,
            last_written_status: status.clone(),
            status,
            foreground,
            preload,
            indexer,
            event_buffer: Vec::with_capacity(16),
            inotify_token,
            inotify_fd,
            cpuset_watch,
            package_list_watch,
        };

        if !Path::new(CPUSET_TOP_APP).exists() {
            d.status
                .daemon
                .warnings
                .push(format!("{} unavailable", CPUSET_TOP_APP));
        }
        if d.foreground.is_none() {
            d.status
                .daemon
                .warnings
                .push("Foreground tracking disabled (setup failure)".to_string());
        }
        if d.preload.is_none() {
            d.status
                .daemon
                .warnings
                .push("Preload feature disabled (setup failure)".to_string());
        }

        d
    }

    fn write_status_if_needed(&mut self, force: bool) {
        let changed = self.status != self.last_written_status;

        if force || changed {
            refresh_pressure_metrics(&mut self.status.pressure);
            if let Err(e) = self.status.write() {
                logging::error(&format!("Status write failed: {}", e));
            } else {
                self.last_written_status = self.status.clone();
            }
        }
    }

    fn sync_status_app_index(&mut self) {
        self.status.app_index = self.indexer.snapshot();
    }

    fn rebuild_index(&mut self) {
        self.indexer
            .rebuild(crate::features::preload::RuntimeAbi::current());
        self.sync_status_app_index();
    }

    fn handle_inotify_ready(&mut self) {
        if self.foreground.is_none() {
            return;
        }

        let Ok(events) = self
            .inotify_fd
            .as_ref()
            .map(read_events)
            .transpose()
            .map(|events| events.unwrap_or_default())
        else {
            return;
        };

        if events.is_empty() {
            return;
        }

        let package_list_changed = Self::is_package_list_change(&events, self.package_list_watch);
        let foreground_changed = Self::is_foreground_change(&events, self.cpuset_watch);

        if package_list_changed {
            self.rebuild_index();
        }

        if foreground_changed
            && let Some(snapshot) = self
                .foreground
                .as_mut()
                .and_then(|foreground| foreground.resolve_current_foreground())
        {
            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64;

            self.status.features.profile.on_foreground_changed(
                self.status.foreground.package.as_deref(),
                snapshot.package.as_deref(),
                now,
            );
            self.status.apply_foreground_snapshot(&snapshot);

            if let Some(pkg) = snapshot.package.as_deref()
                && let Some(preload) = &mut self.preload
            {
                let candidates = self.indexer.get_candidates(pkg);
                preload.on_foreground_package(pkg, &candidates, &mut self.status);
            }
        }
    }

    fn is_package_list_change(events: &[InotifyEvent], package_list_watch: Option<i32>) -> bool {
        let Some(package_list_watch) = package_list_watch else {
            return false;
        };
        events.iter().any(|event| event.wd == package_list_watch)
    }

    fn is_foreground_change(events: &[InotifyEvent], cpuset_watch: Option<i32>) -> bool {
        let Some(cpuset_watch) = cpuset_watch else {
            return false;
        };
        events.iter().any(|event| event.wd == cpuset_watch)
    }

    pub fn run(&mut self) {
        logging::info("CoreShift Policy Daemon started.");
        self.write_status_if_needed(true);

        while !SHUTDOWN.load(Ordering::SeqCst) {
            self.event_buffer.clear();
            match self.reactor.wait(&mut self.event_buffer, 16, -1) {
                Ok(_) => {
                    let mut inotify_ready = false;
                    for event in &self.event_buffer {
                        if Some(event.token) == self.inotify_token {
                            inotify_ready = true;
                        }
                    }

                    if inotify_ready {
                        self.handle_inotify_ready();
                    }

                    self.write_status_if_needed(false);
                }
                Err(e) => {
                    if std::io::Error::last_os_error().kind() != std::io::ErrorKind::Interrupted {
                        logging::error(&format!("Reactor wait error: {}\n", e));
                    }
                }
            }
        }

        logging::info("Shutdown requested. Cleaning up...");
        self.status.daemon.alive = false;
        self.write_status_if_needed(true);
        logging::info("CoreShift Policy Daemon stopped.");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn package_list_event_triggers_rebuild() {
        let events = vec![
            InotifyEvent {
                wd: 3,
                mask: MODIFY_MASK,
                name: Some("ignored".to_string()),
            },
            InotifyEvent {
                wd: 7,
                mask: PACKAGE_FILE_MASK,
                name: None,
            },
        ];

        assert!(Daemon::is_package_list_change(&events, Some(7)));
        assert!(!Daemon::is_package_list_change(&events, Some(8)));
    }
}
