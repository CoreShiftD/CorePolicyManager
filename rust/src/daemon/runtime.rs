use crate::daemon::foreground::{ForegroundResolver, ForegroundSnapshot};
use crate::features::app_index::AppIndexFeature;
use crate::features::preload::{PreloadFeature, RuntimeAbi};
use crate::features::profile::{
    CategoryDatabase, PrivilegeMode, ProfileClass, ProfileFeature, ProfileRuleAction,
    ProfileRulesFile, SelectedProfile, categories_file_path, profile_rules_file_path,
};
use crate::features::tweaks;
use crate::paths::CPUSET_TOP_APP;
use crate::runtime::logging;
use crate::runtime::pressure::refresh_pressure_metrics;
use crate::runtime::signals::SHUTDOWN;
use crate::runtime::status::{
    DaemonInfo, DaemonStatus, FeatureFlags, PreloadStatusFile, PressureStatus, ProfileAppStat,
    ProfileStatusFile, read_device_uptime_secs,
};
use coreshift_lowlevel::inotify::{InotifyEvent, MODIFY_MASK, add_watch, read_events};
use coreshift_lowlevel::reactor::{Event, Fd, Reactor, Token};
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::sync::atomic::Ordering;
use std::time::{SystemTime, UNIX_EPOCH};

const PRESSURE_REFRESH_INTERVAL_MS: u64 = 5_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DaemonConfig {
    pub preload: bool,
    pub usage: bool,
    pub pressure: bool,
    pub app_index: bool,
    pub profile: bool,
}

impl Default for DaemonConfig {
    fn default() -> Self {
        Self {
            preload: true,
            usage: true,
            pressure: true,
            app_index: true,
            profile: true,
        }
    }
}

#[derive(Debug, Clone)]
struct CachedCategoryDb {
    db: CategoryDatabase,
    mtime: Option<SystemTime>,
    by_package: HashMap<String, ProfileClass>,
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

    fn build_reverse_index(db: &CategoryDatabase) -> HashMap<String, ProfileClass> {
        let mut by_package = HashMap::new();
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

#[derive(Debug, Clone)]
struct CachedProfileRules {
    rules: ProfileRulesFile,
    mtime: Option<SystemTime>,
}

impl CachedProfileRules {
    fn load() -> Self {
        let rules = ProfileRulesFile::load();
        let mtime = fs::metadata(profile_rules_file_path())
            .ok()
            .and_then(|metadata| metadata.modified().ok());
        Self { rules, mtime }
    }

    fn refresh_if_changed(&mut self) -> bool {
        let current_mtime = fs::metadata(profile_rules_file_path())
            .ok()
            .and_then(|metadata| metadata.modified().ok());
        if current_mtime != self.mtime {
            self.rules = ProfileRulesFile::load();
            self.mtime = current_mtime;
            return true;
        }
        false
    }

    fn select(&self, class: &ProfileClass, privilege: &PrivilegeMode) -> SelectedProfile {
        self.rules.resolve(class, privilege)
    }

    fn action(&self, class: &ProfileClass, privilege: &PrivilegeMode) -> ProfileRuleAction {
        self.rules.resolve_action(class, privilege)
    }
}

use crate::api::daemon::{
    DaemonContext, DaemonFeature, FeatureThreadContext, ThreadedFeature,
};
use crate::api::resolver::ForegroundEvent;
use std::sync::mpsc::{Sender, channel};
use std::thread;

pub struct Daemon {
    reactor: Reactor,
    status: DaemonStatus,
    last_written_status: Option<DaemonStatus>,
    status_dirty: bool,

    // Core built-in state
    profile: ProfileFeature,
    category_db: CachedCategoryDb,
    profile_rules: CachedProfileRules,
    last_written_profile_status: Option<ProfileStatusFile>,
    profile_dirty: bool,
    foreground_resolver: Option<ForegroundResolver>,
    preload: Option<PreloadFeature>,
    preload_status: PreloadStatusFile,
    last_written_preload_status: Option<PreloadStatusFile>,
    preload_dirty: bool,
    last_pressure_refresh_ms: u64,
    app_index: AppIndexFeature,
    last_tweak_signature: Option<String>,

    // Plugin system
    features: Vec<Box<dyn DaemonFeature>>,
    worker_threads: Vec<thread::JoinHandle<()>>,
    worker_shutdown_tx: Vec<Sender<()>>,
    worker_foreground_tx: Vec<Sender<ForegroundEvent>>,

    // Internal machinery
    event_buffer: Vec<Event>,
    inotify_token: Option<Token>,
    inotify_fd: Option<Fd>,
    cpuset_watch: Option<i32>,
    work_dir: std::path::PathBuf,
    poll_interval: std::time::Duration,
}

impl Daemon {
    pub fn new(config: DaemonConfig) -> Self {
        Self::new_from_builder(
            config,
            Vec::new(),
            Vec::new(),
            std::path::PathBuf::from("/data/local/tmp/coreshift"),
            std::time::Duration::from_millis(100),
        )
    }

    pub fn new_from_builder(
        config: DaemonConfig,
        features: Vec<Box<dyn DaemonFeature>>,
        threaded_features: Vec<Box<dyn ThreadedFeature>>,
        work_dir: std::path::PathBuf,
        poll_interval: std::time::Duration,
    ) -> Self {
        let mut reactor = Reactor::new().expect("Failed to create reactor");
        let privilege = detect_privilege_mode();
        let mut status = DaemonStatus {
            daemon: DaemonInfo {
                alive: true,
                started_ms: SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_millis() as u64,
                privilege: privilege.clone(),
                device_uptime_secs: read_device_uptime_secs(),
            },
            ..Default::default()
        };

        let mut pressure_metrics = crate::runtime::pressure::PressureMetrics::default();
        if config.pressure {
            refresh_pressure_metrics(&mut pressure_metrics);
            status.pressure = PressureStatus::from_metrics(&pressure_metrics);
        }

        let app_index = AppIndexFeature::new(config.app_index, RuntimeAbi::current());
        status.features = FeatureFlags {
            preload: config.preload,
            usage: config.usage,
            profile: config.profile,
            pressure: config.pressure,
            app_index: app_index.enabled(),
        };

        if !Path::new(CPUSET_TOP_APP).exists() {
            logging::warn(&format!("{} unavailable. Degrading.", CPUSET_TOP_APP));
        }

        let mut inotify_token = None;
        let mut inotify_fd = None;
        let mut cpuset_watch = None;
        let foreground_resolver = match reactor.setup_inotify() {
            Ok(fd) => {
                inotify_token = reactor.inotify_token;
                inotify_fd = Some(fd);
                let blacklist_path = work_dir.join("foreground_blacklist.json");
                let blacklist = if let Ok(content) = fs::read_to_string(&blacklist_path) {
                    serde_json::from_str::<serde_json::Value>(&content)
                        .map(|value| {
                            value["packages"]
                                .as_array()
                                .map(|packages| {
                                    packages
                                        .iter()
                                        .filter_map(|entry| entry.as_str().map(str::to_string))
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
                        Err(error) => {
                            logging::error(&format!(
                                "Failed to add watch on {}: {}",
                                CPUSET_TOP_APP, error
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
            Err(error) => {
                logging::error(&format!("Failed to setup inotify: {}", error));
                None
            }
        };

        let preload = if config.preload {
            Some(PreloadFeature::new())
        } else {
            None
        };

        let last_pressure_refresh_ms = status.pressure.last_refresh_ms;

        let mut worker_threads = Vec::new();
        let mut worker_shutdown_tx = Vec::new();
        let mut worker_foreground_tx = Vec::new();

        for feature in threaded_features {
            let (shutdown_tx, shutdown_rx) = channel();
            let (foreground_tx, foreground_rx) = channel();

            let name = feature.name();
            let feature_work_dir = work_dir.clone();

            let handle = thread::spawn(move || {
                let ctx = FeatureThreadContext {
                    work_dir: feature_work_dir,
                    shutdown_receiver: shutdown_rx,
                    foreground_receiver: foreground_rx,
                };
                if let Err(e) = feature.run(ctx) {
                    logging::error(&format!("Worker feature '{}' failed: {}", name, e));
                }
            });

            worker_threads.push(handle);
            worker_shutdown_tx.push(shutdown_tx);
            worker_foreground_tx.push(foreground_tx);
        }

        Self {
            reactor,
            status,
            last_written_status: None,
            status_dirty: true,
            profile: ProfileFeature::default(),
            category_db: CachedCategoryDb::load(),
            profile_rules: CachedProfileRules::load(),
            last_written_profile_status: None,
            profile_dirty: true,
            foreground_resolver,
            preload,
            preload_status: PreloadStatusFile::default(),
            last_written_preload_status: None,
            preload_dirty: config.preload,
            last_pressure_refresh_ms,
            app_index,
            last_tweak_signature: None,
            features,
            worker_threads,
            worker_shutdown_tx,
            worker_foreground_tx,
            event_buffer: Vec::with_capacity(16),
            inotify_token,
            inotify_fd,
            cpuset_watch,
            work_dir,
            poll_interval,
        }
    }

    fn profile_subsystem_enabled(&self) -> bool {
        self.status.features.usage || self.status.features.profile
    }

    fn refresh_profile_caches(&mut self) {
        if self.profile_subsystem_enabled()
            && (self.category_db.refresh_if_changed() || self.profile_rules.refresh_if_changed())
        {
            self.profile_dirty = true;
        }
    }

    fn current_profile_class(&self) -> ProfileClass {
        self.category_db
            .classify(self.status.foreground.package.as_deref())
    }

    fn build_profile_status(&self) -> ProfileStatusFile {
        let class = self.current_profile_class();
        let selected_profile = self
            .profile_rules
            .select(&class, &self.status.daemon.privilege);
        let (foreground_switch_count, top_apps) = if self.status.features.usage {
            (
                self.profile.foreground_switch_count,
                self.profile
                    .snapshot_top_apps()
                    .into_iter()
                    .map(|(package, total_secs)| ProfileAppStat {
                        package,
                        total_secs,
                    })
                    .collect(),
            )
        } else {
            (0, Vec::new())
        };

        ProfileStatusFile {
            schema_version: 1,
            current_class: class.to_string(),
            privilege: self.status.daemon.privilege.to_string(),
            selected_profile,
            foreground_switch_count,
            top_apps,
        }
    }

    fn write_status_files_if_needed(&mut self, force: bool) {
        self.refresh_profile_caches();

        if force || self.status_dirty {
            let now_ms = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64;
            if self.status.features.pressure && self.should_refresh_pressure(force, now_ms) {
                let mut pressure_metrics = crate::runtime::pressure::PressureMetrics::default();
                refresh_pressure_metrics(&mut pressure_metrics);
                self.last_pressure_refresh_ms = pressure_metrics.last_refresh_ms;
                self.status.pressure = PressureStatus::from_metrics(&pressure_metrics);
            }
            self.status.daemon.device_uptime_secs = read_device_uptime_secs();
            if let Err(error) = self.status.write_if_changed(&mut self.last_written_status) {
                logging::error(&format!("Status write failed: {}", error));
            } else {
                self.status_dirty = false;
            }
        }

        if self.profile_subsystem_enabled() && (force || self.profile_dirty) {
            let profile_status = self.build_profile_status();
            if let Err(error) =
                profile_status.write_if_changed(&mut self.last_written_profile_status)
            {
                logging::error(&format!("Profile status write failed: {}", error));
            } else {
                self.profile_dirty = false;
            }
        }

        if self.status.features.preload && (force || self.preload_dirty) {
            if let Err(error) = self
                .preload_status
                .write_if_changed(&mut self.last_written_preload_status)
            {
                logging::error(&format!("Preload status write failed: {}", error));
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
        if self.foreground_resolver.is_none() {
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
                .foreground_resolver
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
            self.profile_dirty = self.profile_subsystem_enabled();
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

        let previous_foreground_snapshot = ForegroundSnapshot {
            pid: previous_foreground.pid,
            package: previous_foreground.package,
            last_skip_reason: None,
        };

        let foreground_event = ForegroundEvent {
            previous: previous_foreground_snapshot,
            current: snapshot.clone(),
        };

        // Notify plugins
        let ctx = DaemonContext {
            work_dir: self.work_dir.clone(),
            status: self.status.clone(),
        };
        for feature in &mut self.features {
            if let Err(e) = feature.on_foreground_changed(&ctx, &foreground_event) {
                logging::error(&format!(
                    "Feature '{}' error on foreground change: {}",
                    feature.name(),
                    e
                ));
            }
        }
        for tx in &self.worker_foreground_tx {
            let _ = tx.send(foreground_event.clone());
        }

        if let Some(pkg) = snapshot.package.as_deref() {
            self.refresh_profile_caches();
            let class = self.current_profile_class();
            let profile_action = if self.status.features.profile {
                Some(
                    self.profile_rules
                        .action(&class, &self.status.daemon.privilege),
                )
            } else {
                None
            };
            let allow_preload = profile_action
                .as_ref()
                .map(|action| action.preload)
                .unwrap_or(true);
            if let Some(action) = profile_action.as_ref() {
                self.apply_profile_commands_if_needed(&class, action);
            } else {
                self.last_tweak_signature = None;
            }
            if allow_preload {
                let candidates = if self.status.features.app_index {
                    self.app_index.get_candidates(pkg)
                } else {
                    None
                };
                let Some(preload) = &mut self.preload else {
                    return;
                };
                let previous_preload_status = self.preload_status.clone();
                preload.on_foreground_package(
                    pkg,
                    candidates.as_deref().unwrap_or(&[]),
                    &mut self.preload_status,
                );
                if self.preload_status != previous_preload_status {
                    self.preload_dirty = true;
                }
            }
        } else {
            self.last_tweak_signature = None;
        }
    }

    fn apply_profile_commands_if_needed(
        &mut self,
        class: &ProfileClass,
        action: &ProfileRuleAction,
    ) {
        if action.commands.is_empty() {
            self.last_tweak_signature = None;
            return;
        }

        let Ok(signature) = tweaks::command_fingerprint(&action.commands) else {
            logging::warn("Skipping invalid profile tweak command set.");
            return;
        };
        if self.last_tweak_signature.as_deref() == Some(signature.as_str()) {
            return;
        }

        let source = format!("profile:{}/{}", class, self.status.daemon.privilege);
        let summary = tweaks::apply_tweak_commands(&source, &action.commands);
        if summary.failed_writes > 0 {
            logging::warn(&format!(
                "Tweak command application reported failures for {}.",
                source
            ));
        }
        self.last_tweak_signature = Some(signature);
    }

    pub fn status(&self) -> &DaemonStatus {
        &self.status
    }

    pub fn run(&mut self) {
        logging::info("CoreShift Policy Daemon started.");

        // on_start
        let ctx = DaemonContext {
            work_dir: self.work_dir.clone(),
            status: self.status.clone(),
        };
        for feature in &mut self.features {
            if let Err(e) = feature.on_start(&ctx) {
                logging::error(&format!(
                    "Feature '{}' error on start: {}",
                    feature.name(),
                    e
                ));
            }
        }

        self.write_status_files_if_needed(true);

        while !SHUTDOWN.load(Ordering::SeqCst) {
            self.event_buffer.clear();

            // on_tick
            let ctx = DaemonContext {
                work_dir: self.work_dir.clone(),
                status: self.status.clone(),
            };
            for feature in &mut self.features {
                if let Err(e) = feature.on_tick(&ctx) {
                    logging::error(&format!(
                        "Feature '{}' error on tick: {}",
                        feature.name(),
                        e
                    ));
                }
            }

            match self.reactor.wait(
                &mut self.event_buffer,
                16,
                self.poll_interval.as_millis() as i32,
            ) {
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
                Err(error) => {
                    if std::io::Error::last_os_error().kind() != std::io::ErrorKind::Interrupted {
                        logging::error(&format!("Reactor wait error: {}\n", error));
                    }
                }
            }
        }

        logging::info("Shutdown requested. Cleaning up...");

        // on_shutdown
        let ctx = DaemonContext {
            work_dir: self.work_dir.clone(),
            status: self.status.clone(),
        };
        for feature in &mut self.features {
            if let Err(e) = feature.on_shutdown(&ctx) {
                logging::error(&format!(
                    "Feature '{}' error on shutdown: {}",
                    feature.name(),
                    e
                ));
            }
        }
        for tx in &self.worker_shutdown_tx {
            let _ = tx.send(());
        }

        // Wait for workers
        for handle in self.worker_threads.drain(..) {
            let _ = handle.join();
        }

        self.status.daemon.alive = false;
        self.status_dirty = true;
        self.write_status_files_if_needed(true);
        self.app_index.shutdown();
        logging::info("CoreShift Policy Daemon stopped.");
    }
}

fn detect_privilege_mode() -> PrivilegeMode {
    if let Some(uid) = std::env::var_os("COREPOLICY_TEST_UID")
        .and_then(|value| value.into_string().ok())
        .and_then(|value| value.parse::<u32>().ok())
    {
        return privilege_mode_from_uid(uid);
    }

    let Some(content) = fs::read_to_string("/proc/self/status").ok() else {
        return PrivilegeMode::Unknown;
    };
    let Some(uid_line) = content.lines().find(|line| line.starts_with("Uid:")) else {
        return PrivilegeMode::Unknown;
    };
    let uid = uid_line
        .split_whitespace()
        .nth(1)
        .and_then(|value| value.parse::<u32>().ok());
    uid.map(privilege_mode_from_uid)
        .unwrap_or(PrivilegeMode::Unknown)
}

fn privilege_mode_from_uid(uid: u32) -> PrivilegeMode {
    match uid {
        0 => PrivilegeMode::Root,
        2000 => PrivilegeMode::Shell,
        _ => PrivilegeMode::Unknown,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::foreground::ForegroundSnapshot;
    use std::fs;
    use std::path::Path;
    use std::sync::{Mutex, OnceLock};
    use std::time::Duration;

    fn now_ms() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64
    }

    fn test_env_lock() -> &'static Mutex<()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
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

    fn with_test_profile_rules_file(path: &Path, f: impl FnOnce()) {
        unsafe {
            std::env::set_var("COREPOLICY_TEST_PROFILE_RULES_FILE", path);
        }
        f();
        unsafe {
            std::env::remove_var("COREPOLICY_TEST_PROFILE_RULES_FILE");
        }
    }

    fn with_test_tweak_paths(root: &Path, f: impl FnOnce()) {
        unsafe {
            std::env::set_var("COREPOLICY_TEST_PROC_ROOT", root.join("proc"));
            std::env::set_var("COREPOLICY_TEST_SYS_ROOT", root.join("sys"));
            std::env::set_var("COREPOLICY_TEST_CPUSET_ROOT", root.join("dev/cpuset"));
            std::env::set_var("COREPOLICY_TEST_CPUCTL_ROOT", root.join("dev/cpuctl"));
            std::env::set_var(
                "COREPOLICY_TEST_TWEAK_CACHE_FILE",
                root.join("tweak_cache.json"),
            );
            std::env::set_var(
                "COREPOLICY_TEST_TWEAK_STATUS_FILE",
                root.join("tweak_status.json"),
            );
        }
        f();
        unsafe {
            std::env::remove_var("COREPOLICY_TEST_PROC_ROOT");
            std::env::remove_var("COREPOLICY_TEST_SYS_ROOT");
            std::env::remove_var("COREPOLICY_TEST_CPUSET_ROOT");
            std::env::remove_var("COREPOLICY_TEST_CPUCTL_ROOT");
            std::env::remove_var("COREPOLICY_TEST_TWEAK_CACHE_FILE");
            std::env::remove_var("COREPOLICY_TEST_TWEAK_STATUS_FILE");
        }
    }

    fn write_file(path: &Path, content: &str) {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(path, content).unwrap();
    }

    #[test]
    fn foreground_event_updates_inline_features_without_extra_worker_logic() {
        let mut daemon = Daemon::new(DaemonConfig::default());

        daemon.process_foreground_snapshot(ForegroundSnapshot {
            pid: Some(1234),
            package: Some("com.example.app".to_string()),
            last_skip_reason: None,
        });

        assert_eq!(
            daemon.status.foreground.package.as_deref(),
            Some("com.example.app")
        );
        assert_eq!(daemon.status.foreground.pid, Some(1234));
        assert!(daemon.status.foreground.session_started_ms.is_some());
        assert!(daemon.status.features.usage);
        assert!(daemon.status.features.profile);
        assert!(daemon.status_dirty);
        assert!(daemon.profile_dirty);
    }

    #[test]
    fn category_reverse_cache_classifies_known_package_correctly() {
        let _guard = test_env_lock().lock().unwrap();
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
        let _guard = test_env_lock().lock().unwrap();
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
        let mut daemon = Daemon::new(DaemonConfig::default());
        daemon.last_pressure_refresh_ms = now_ms();
        assert!(!daemon.should_refresh_pressure(false, daemon.last_pressure_refresh_ms + 1_000));
        assert!(daemon.should_refresh_pressure(false, daemon.last_pressure_refresh_ms + 5_000));
        assert!(daemon.should_refresh_pressure(true, daemon.last_pressure_refresh_ms + 1_000));
    }

    #[test]
    fn profile_rules_select_by_privilege() {
        let _guard = test_env_lock().lock().unwrap();
        let root =
            std::env::temp_dir().join(format!("coreshift_profile_rules_{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        let categories_path = root.join("profiles_category.json");
        let rules_path = root.join("profile_rules.json");
        fs::write(
            &categories_path,
            r#"{"version":1,"updated_ms":1,"categories":{"game":["com.example.game"],"social":[],"tool":[],"launcher":[],"keyboard":[],"system":[]}}"#,
        )
        .unwrap();
        fs::write(
            &rules_path,
            r#"{"schema_version":1,"rules":{"game":{"root":{"preload":true,"priority":"performance","commands":[]},"shell":{"preload":false,"priority":"balanced","commands":[]}}}}"#,
        )
        .unwrap();

        with_test_categories_file(&categories_path, || {
            with_test_profile_rules_file(&rules_path, || {
                unsafe {
                    std::env::set_var("COREPOLICY_TEST_UID", "0");
                }
                let mut daemon = Daemon::new(DaemonConfig::default());
                daemon.process_foreground_snapshot(ForegroundSnapshot {
                    pid: Some(1234),
                    package: Some("com.example.game".to_string()),
                    last_skip_reason: None,
                });
                let profile_status = daemon.build_profile_status();
                assert_eq!(profile_status.current_class, "game");
                assert_eq!(profile_status.privilege, "root");
                assert!(profile_status.selected_profile.preload);
                assert_eq!(
                    profile_status.selected_profile.priority,
                    crate::features::profile::ProfilePriority::Performance
                );
                unsafe {
                    std::env::remove_var("COREPOLICY_TEST_UID");
                }
            });
        });

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn profile_commands_execute_on_profile_switch() {
        let _guard = test_env_lock().lock().unwrap();
        let root =
            std::env::temp_dir().join(format!("coreshift_profile_tweaks_{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        let categories_path = root.join("profiles_category.json");
        let rules_path = root.join("profile_rules.json");
        fs::write(
            &categories_path,
            r#"{"version":1,"updated_ms":1,"categories":{"game":["com.example.game"],"social":[],"tool":[],"launcher":[],"keyboard":[],"system":[]}}"#,
        )
        .unwrap();
        fs::write(
            &rules_path,
            r#"{"schema_version":1,"rules":{"game":{"root":{"preload":true,"priority":"performance","commands":["tweak write /proc/sys/vm/swappiness 5"]}}}}"#,
        )
        .unwrap();
        write_file(&root.join("proc/sys/vm/swappiness"), "60");

        with_test_categories_file(&categories_path, || {
            with_test_profile_rules_file(&rules_path, || {
                with_test_tweak_paths(&root, || {
                    unsafe {
                        std::env::set_var("COREPOLICY_TEST_UID", "0");
                    }
                    let mut daemon = Daemon::new(DaemonConfig::default());
                    daemon.process_foreground_snapshot(ForegroundSnapshot {
                        pid: Some(100),
                        package: Some("com.example.game".to_string()),
                        last_skip_reason: None,
                    });
                    assert_eq!(
                        fs::read_to_string(root.join("proc/sys/vm/swappiness")).unwrap(),
                        "5"
                    );
                    unsafe {
                        std::env::remove_var("COREPOLICY_TEST_UID");
                    }
                });
            });
        });

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn same_profile_command_set_is_not_spammed() {
        let _guard = test_env_lock().lock().unwrap();
        let root = std::env::temp_dir().join(format!(
            "coreshift_profile_tweak_dedupe_{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        let categories_path = root.join("profiles_category.json");
        let rules_path = root.join("profile_rules.json");
        fs::write(
            &categories_path,
            r#"{"version":1,"updated_ms":1,"categories":{"game":["com.example.game"],"social":[],"tool":[],"launcher":[],"keyboard":[],"system":[]}}"#,
        )
        .unwrap();
        fs::write(
            &rules_path,
            r#"{"schema_version":1,"rules":{"game":{"root":{"preload":true,"priority":"performance","commands":["tweak write /proc/sys/vm/swappiness 5"]}}}}"#,
        )
        .unwrap();
        write_file(&root.join("proc/sys/vm/swappiness"), "60");

        with_test_categories_file(&categories_path, || {
            with_test_profile_rules_file(&rules_path, || {
                with_test_tweak_paths(&root, || {
                    unsafe {
                        std::env::set_var("COREPOLICY_TEST_UID", "0");
                    }
                    let mut daemon = Daemon::new(DaemonConfig::default());
                    daemon.process_foreground_snapshot(ForegroundSnapshot {
                        pid: Some(100),
                        package: Some("com.example.game".to_string()),
                        last_skip_reason: None,
                    });
                    write_file(&root.join("proc/sys/vm/swappiness"), "60");
                    daemon.process_foreground_snapshot(ForegroundSnapshot {
                        pid: Some(100),
                        package: Some("com.example.game".to_string()),
                        last_skip_reason: None,
                    });
                    assert_eq!(
                        fs::read_to_string(root.join("proc/sys/vm/swappiness")).unwrap(),
                        "60"
                    );
                    unsafe {
                        std::env::remove_var("COREPOLICY_TEST_UID");
                    }
                });
            });
        });

        let _ = fs::remove_dir_all(&root);
    }
}
