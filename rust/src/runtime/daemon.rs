use crate::features::app_index::AppIndexFeature;
use crate::features::preload::PreloadFeature;
use crate::features::profile::{
    CategoryDatabase, ProfileClass, ProfileFeature, categories_file_path,
};
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

const PRESSURE_REFRESH_INTERVAL_MS: u64 = 5_000;

#[derive(Debug, Clone)]
struct CachedCategoryDb {
    db: CategoryDatabase,
    mtime: Option<SystemTime>,
    by_package: std::collections::HashMap<String, ProfileClass>,
}

impl CachedCategoryDb {
    fn load() -> Self {
        let db = CategoryDatabase::load();
        let mtime = fs::metadata(categories_file_path())
            .ok()
            .and_then(|metadata| metadata.modified().ok());
        let by_package = Self::build_reverse_index(&db);
        Self {
            db,
            mtime,
            by_package,
        }
    }

    fn refresh_if_changed(&mut self) -> bool {
        let current_mtime = fs::metadata(categories_file_path())
            .ok()
            .and_then(|metadata| metadata.modified().ok());
        if current_mtime != self.mtime {
            self.db = CategoryDatabase::load();
            self.mtime = current_mtime;
            self.by_package = Self::build_reverse_index(&self.db);
            return true;
        }
        false
    }

    fn build_reverse_index(
        db: &CategoryDatabase,
    ) -> std::collections::HashMap<String, ProfileClass> {
        let mut by_package = std::collections::HashMap::new();
        for (category, packages) in &db.categories {
            let class = match category.as_str() {
                "game" => ProfileClass::Game,
                "social" => ProfileClass::Social,
                "tool" => ProfileClass::Tool,
                "launcher" => ProfileClass::Launcher,
                "keyboard" => ProfileClass::Keyboard,
                "system" => ProfileClass::System,
                _ => ProfileClass::Unknown,
            };
            for package in packages {
                by_package.insert(package.clone(), class.clone());
            }
        }
        by_package
    }

    fn classify(&self, package: Option<&str>) -> ProfileClass {
        package
            .and_then(|pkg| self.by_package.get(pkg).cloned())
            .unwrap_or(ProfileClass::Unknown)
    }
}

pub struct Daemon {
    reactor: Reactor,
    status: DaemonStatus,
    last_written_status: Option<DaemonStatus>,
    status_dirty: bool,
    profile: ProfileFeature,
    category_db: CachedCategoryDb,
    last_written_profile_status: Option<ProfileStatusFile>,
    profile_dirty: bool,
    foreground: Option<ForegroundResolver>,
    preload: Option<PreloadFeature>,
    preload_status: PreloadStatusFile,
    last_written_preload_status: Option<PreloadStatusFile>,
    preload_dirty: bool,
    last_pressure_refresh_ms: u64,
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
            usage: profile_enabled,
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

                if let Some(inotify_fd_ref) = inotify_fd.as_ref() {
                    match add_watch(inotify_fd_ref, CPUSET_TOP_APP, MODIFY_MASK) {
                        Ok(wd) => cpuset_watch = Some(wd),
                        Err(e) => {
                            logging::error(&format!(
                                "Failed to add watch on {}: {}",
                                CPUSET_TOP_APP, e
                            ));
                        }
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

        let profile = ProfileFeature::default();
        let preload_status = PreloadStatusFile::default();
        let last_pressure_refresh_ms = status.pressure.last_refresh_ms;

        Self {
            reactor,
            status,
            last_written_status: None,
            status_dirty: true,
            profile,
            category_db: CachedCategoryDb::load(),
            last_written_profile_status: None,
            profile_dirty: profile_enabled,
            foreground,
            preload,
            preload_status,
            last_written_preload_status: None,
            preload_dirty: preload_enabled,
            last_pressure_refresh_ms,
            app_index,
            event_buffer: Vec::with_capacity(16),
            inotify_token,
            inotify_fd,
            cpuset_watch,
        }
    }

    fn write_status_files_if_needed(&mut self, force: bool) {
        if self.status.features.usage && self.category_db.refresh_if_changed() {
            self.profile_dirty = true;
        }

        if force || self.status_dirty {
            let now_ms = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64;
            if self.should_refresh_pressure(force, now_ms) {
                let mut pressure_metrics = crate::runtime::pressure::PressureMetrics::default();
                refresh_pressure_metrics(&mut pressure_metrics);
                self.last_pressure_refresh_ms = pressure_metrics.last_refresh_ms;
                self.status.pressure = PressureStatus::from_metrics(&pressure_metrics);
            }
            self.status.daemon.device_uptime_secs = read_device_uptime_secs();
            if let Err(e) = self.status.write_if_changed(&mut self.last_written_status) {
                logging::error(&format!("Status write failed: {}", e));
            } else {
                self.status_dirty = false;
            }
        }

        if self.status.features.usage && (force || self.profile_dirty) {
            let class = self
                .category_db
                .classify(self.status.foreground.package.as_deref());
            let recommendation = ProfileFeature::get_recommendation(&class);
            let profile_status = ProfileStatusFile {
                schema_version: 1,
                foreground_switch_count: self.profile.foreground_switch_count,
                top_apps: self
                    .profile
                    .snapshot_top_apps()
                    .into_iter()
                    .map(
                        |(package, total_secs)| crate::runtime::status::ProfileAppStat {
                            package,
                            total_secs,
                        },
                    )
                    .collect(),
                current_class: class.to_string(),
                recommendation: recommendation.to_string(),
            };
            if let Err(e) = profile_status.write_if_changed(&mut self.last_written_profile_status) {
                logging::error(&format!("Profile status write failed: {}", e));
            } else {
                self.profile_dirty = false;
            }
        }

        if self.status.features.preload && (force || self.preload_dirty) {
            if let Err(e) = self
                .preload_status
                .write_if_changed(&mut self.last_written_preload_status)
            {
                logging::error(&format!("Preload status write failed: {}", e));
            } else {
                self.preload_dirty = false;
            }
        }
    }

    fn should_refresh_pressure(&self, force: bool, now_ms: u64) -> bool {
        force
            || now_ms.saturating_sub(self.last_pressure_refresh_ms) >= PRESSURE_REFRESH_INTERVAL_MS
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
        let previous_foreground = self.status.foreground.clone();

        self.status.apply_foreground_snapshot(&snapshot);
        if prev_package.as_deref() != snapshot.package.as_deref() {
            self.status.foreground.session_started_ms = snapshot.package.as_ref().map(|_| now);
        }

        if self.status.foreground != previous_foreground {
            self.status_dirty = true;
            self.profile_dirty = self.status.features.usage;
        }

        if self.status.features.usage {
            self.profile.on_foreground_changed(
                prev_package.as_deref(),
                snapshot.package.as_deref(),
                prev_session_started_ms,
                now,
            );
            if prev_package.as_deref() != snapshot.package.as_deref() {
                self.profile_dirty = true;
            }
        }

        if let Some(pkg) = snapshot.package.as_deref()
            && let Some(preload) = &mut self.preload
        {
            let previous_preload_status = self.preload_status.clone();
            let candidates = self.app_index.get_candidates(pkg);
            preload.on_foreground_package(
                pkg,
                candidates.as_deref().unwrap_or(&[]),
                &mut self.preload_status,
            );
            if self.preload_status != previous_preload_status {
                self.preload_dirty = true;
            }
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
        self.status_dirty = true;
        self.write_status_files_if_needed(true);
        self.app_index.shutdown();
        logging::info("CoreShift Policy Daemon stopped.");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::foreground::ForegroundSnapshot;
    use std::path::Path;
    use std::time::Duration;

    fn now_ms() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64
    }

    fn with_test_categories_file(path: &Path, f: impl FnOnce()) {
        unsafe {
            std::env::set_var("COREPOLICY_TEST_CATEGORIES_FILE", path);
        }
        f();
        unsafe {
            std::env::remove_var("COREPOLICY_TEST_CATEGORIES_FILE");
        }
    }

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
        assert!(daemon.status.features.usage);
        assert!(daemon.status_dirty);
        assert!(daemon.profile_dirty);
    }

    #[test]
    fn category_reverse_cache_classifies_known_package_correctly() {
        let root =
            std::env::temp_dir().join(format!("coreshift_categories_cache_{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        let categories_path = root.join("profiles_category.json");
        fs::write(
            &categories_path,
            r#"{"version":1,"updated_ms":1,"categories":{"game":["com.example.game"],"social":[],"tool":[],"launcher":[],"keyboard":[],"system":[]}}"#,
        )
        .unwrap();

        with_test_categories_file(&categories_path, || {
            let cache = CachedCategoryDb::load();
            assert_eq!(cache.classify(Some("com.example.game")), ProfileClass::Game);
        });

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn category_cache_reloads_when_mtime_changes() {
        let root = std::env::temp_dir().join(format!(
            "coreshift_categories_reload_{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        let categories_path = root.join("profiles_category.json");

        with_test_categories_file(&categories_path, || {
            let mut db = CategoryDatabase::default();
            assert!(db.add("game", "com.example.game"));
            db.save().unwrap();
            let mut cache = CachedCategoryDb::load();
            assert_eq!(cache.classify(Some("com.example.game")), ProfileClass::Game);

            std::thread::sleep(Duration::from_millis(1_100));
            db.remove("com.example.game");
            assert!(db.add("social", "com.example.game"));
            db.save().unwrap();

            assert!(cache.refresh_if_changed());
            assert_eq!(
                cache.classify(Some("com.example.game")),
                ProfileClass::Social
            );
        });

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn psi_refresh_is_throttled() {
        let mut daemon = Daemon::new(false);
        daemon.last_pressure_refresh_ms = now_ms();
        assert!(!daemon.should_refresh_pressure(false, daemon.last_pressure_refresh_ms + 1_000));
        assert!(daemon.should_refresh_pressure(false, daemon.last_pressure_refresh_ms + 5_000));
        assert!(daemon.should_refresh_pressure(true, daemon.last_pressure_refresh_ms + 1_000));
    }
}
