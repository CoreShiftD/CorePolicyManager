use crate::features::app_index::AppIndexFeature;
use crate::features::preload::PreloadFeature;
use crate::features::profile::ProfileFeature;
use crate::paths::{BLACKLIST_FILE, CPUSET_TOP_APP};
use crate::runtime::foreground::ForegroundResolver;
use crate::runtime::logging;
use crate::runtime::pressure::refresh_pressure_metrics;
use crate::runtime::signals::SHUTDOWN;
use crate::runtime::status::{
    DaemonStatus, FeatureFlags, PreloadStatusFile, PressureStatus, ProfileStatusFile,
    read_device_uptime_secs,
};
use coreshift_lowlevel::inotify::{InotifyEvent, MODIFY_MASK, add_watch, read_events};
use coreshift_lowlevel::reactor::{Event, Fd, Reactor, Token};
use std::fs;
use std::path::Path;
use std::sync::atomic::Ordering;
use std::time::{SystemTime, UNIX_EPOCH};

pub struct Daemon {
    reactor: Reactor,
    status: DaemonStatus,
    last_written_status: Option<DaemonStatus>,
    profile: ProfileFeature,
    last_written_profile_status: Option<ProfileStatusFile>,
    foreground: Option<ForegroundResolver>,
    preload: Option<PreloadFeature>,
    preload_status: PreloadStatusFile,
    last_written_preload_status: Option<PreloadStatusFile>,
    app_index: AppIndexFeature,
    event_buffer: Vec<Event>,
    inotify_token: Option<Token>,
    inotify_fd: Option<Fd>,
    cpuset_watch: Option<i32>,
}

impl Daemon {
    pub fn new(preload_only: bool) -> Self {
        let mut reactor = Reactor::new().expect("Failed to create reactor");
        let mut status = DaemonStatus {
            daemon: crate::runtime::status::DaemonInfo {
                alive: true,
                started_ms: SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_millis() as u64,
                device_uptime_secs: read_device_uptime_secs(),
            },
            ..Default::default()
        };

        let mut pressure_metrics = crate::runtime::pressure::PressureMetrics::default();
        refresh_pressure_metrics(&mut pressure_metrics);
        status.pressure = PressureStatus::from_metrics(&pressure_metrics);

        let profile_enabled = !preload_only;
        let preload_enabled = true;
        let app_index = AppIndexFeature::new(true, crate::features::preload::RuntimeAbi::current());
        status.features = FeatureFlags {
            preload: preload_enabled,
            profile: profile_enabled,
            pressure: true,
            app_index: app_index.enabled(),
        };

        // Startup Diagnostics
        if !Path::new(CPUSET_TOP_APP).exists() {
            logging::warn(&format!("{} unavailable. Degrading.", CPUSET_TOP_APP));
        }

        let mut inotify_token = None;
        let mut inotify_fd = None;
        let mut cpuset_watch = None;
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

        let profile = ProfileFeature {
            enabled: profile_enabled,
            ..Default::default()
        };
        let preload_status = PreloadStatusFile::default();

        Self {
            reactor,
            status,
            last_written_status: None,
            profile,
            last_written_profile_status: None,
            foreground,
            preload,
            preload_status,
            last_written_preload_status: None,
            app_index,
            event_buffer: Vec::with_capacity(16),
            inotify_token,
            inotify_fd,
            cpuset_watch,
        }
    }

    fn write_status_files_if_needed(&mut self, force: bool) {
        if force || self.last_written_status.as_ref() != Some(&self.status) {
            let mut pressure_metrics = crate::runtime::pressure::PressureMetrics::default();
            refresh_pressure_metrics(&mut pressure_metrics);
            self.status.pressure = PressureStatus::from_metrics(&pressure_metrics);
            self.status.daemon.device_uptime_secs = read_device_uptime_secs();
            if let Err(e) = self.status.write_if_changed(&mut self.last_written_status) {
                logging::error(&format!("Status write failed: {}", e));
            }
        }

        let db = crate::features::profile::CategoryDatabase::load();
        let profile_status = ProfileStatusFile::from_feature(
            &self.profile,
            self.status.foreground.package.as_deref(),
            &db,
        );
        if let Err(e) = profile_status.write_if_changed(&mut self.last_written_profile_status) {
            logging::error(&format!("Profile status write failed: {}", e));
        }

        if let Err(e) = self
            .preload_status
            .write_if_changed(&mut self.last_written_preload_status)
        {
            logging::error(&format!("Preload status write failed: {}", e));
        }
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

        let foreground_changed = Self::is_foreground_change(&events, self.cpuset_watch);

        if foreground_changed
            && let Some(snapshot) = self
                .foreground
                .as_mut()
                .and_then(|foreground| foreground.resolve_current_foreground())
        {
            self.process_foreground_snapshot(snapshot);
        }
    }

    fn is_foreground_change(events: &[InotifyEvent], cpuset_watch: Option<i32>) -> bool {
        let Some(cpuset_watch) = cpuset_watch else {
            return false;
        };
        events.iter().any(|event| event.wd == cpuset_watch)
    }

    fn process_foreground_snapshot(
        &mut self,
        snapshot: crate::runtime::foreground::ForegroundSnapshot,
    ) {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;
        let prev_package = self.status.foreground.package.clone();
        let prev_session_started_ms = self.status.foreground.session_started_ms;

        self.status.apply_foreground_snapshot(&snapshot);
        if prev_package.as_deref() != snapshot.package.as_deref() {
            self.status.foreground.session_started_ms = snapshot.package.as_ref().map(|_| now);
        }

        if self.profile.enabled {
            self.profile.on_foreground_changed(
                prev_package.as_deref(),
                snapshot.package.as_deref(),
                prev_session_started_ms,
                now,
            );
        }

        if let Some(pkg) = snapshot.package.as_deref()
            && let Some(preload) = &mut self.preload
        {
            let candidates = self.app_index.get_candidates(pkg);
            preload.on_foreground_package(
                pkg,
                candidates.as_deref().unwrap_or(&[]),
                &mut self.preload_status,
            );
        }
    }

    pub fn run(&mut self) {
        logging::info("CoreShift Policy Daemon started.");
        self.write_status_files_if_needed(true);

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

                    self.write_status_files_if_needed(false);
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
        self.write_status_files_if_needed(true);
        self.app_index.shutdown();
        logging::info("CoreShift Policy Daemon stopped.");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::foreground::ForegroundSnapshot;

    #[test]
    fn foreground_event_updates_inline_features_without_extra_worker_logic() {
        let mut daemon = Daemon::new(false);
        let before = daemon.app_index.name();
        daemon.process_foreground_snapshot(ForegroundSnapshot {
            pid: Some(1234),
            package: Some("com.example.app".to_string()),
            last_skip_reason: None,
        });

        assert_eq!(before, "app_index");
        assert_eq!(
            daemon.status.foreground.package.as_deref(),
            Some("com.example.app")
        );
        assert_eq!(daemon.status.foreground.pid, Some(1234));
        assert!(daemon.status.foreground.session_started_ms.is_some());
        assert!(daemon.profile.enabled);
    }
}
