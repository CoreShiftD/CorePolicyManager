use crate::blocklist::{
    load_or_create_blocklist_with_generated, parse_blocklist_file,
    resolve_input_method_blocklist_packages_with, resolve_launcher_blocklist_package_with,
};
use crate::defaults::{
    ANDROID_APP_ID_MIN, ANDROID_APP_ID_MODULUS, AndroidForegroundConfig,
    effective_android_cgroup_v2_roots, input_method_setting_get_argv,
    launcher_resolve_activity_argv, accessibility_service_query_argv, parse_accessibility_uids_stdout, package_list_argv,
    parse_android_package_list_stdout,
};
use crate::engine_io_error;
use crate::game::GameList;
use coreshift_engine::config::ExecConfig;
use coreshift_engine::exec::ExecRunner;
use coreshift_engine::services::foreground::{
    CgroupV1CpusetSource, CgroupV2EventsSource, ForegroundCandidate, ForegroundCandidateFilter,
    ForegroundSource, ForegroundSourceKind, FsPackageFileStatProvider, PackageFileStat,
    PackageFileStatProvider, PackageProviderSource, PackageUidProvider, PackageUidSnapshot,
    UidPackageCache, resolve_activity_stdout,
};
use std::collections::BTreeSet;
use std::io;
use std::io::Read;
use std::path::{Path, PathBuf};

pub trait ForegroundPackageProvider {
    fn current_package(&mut self) -> io::Result<Option<String>>;
}

struct AndroidPackageListProvider {
    exec: ExecRunner,
    cmd_path: PathBuf,
}

impl AndroidPackageListProvider {
    fn new(exec: ExecRunner, cmd_path: PathBuf) -> Self {
        Self { exec, cmd_path }
    }
}

impl PackageUidProvider for AndroidPackageListProvider {
    fn load(&self) -> Result<PackageUidSnapshot, coreshift_engine::EngineError> {
        let output = self
            .exec
            .run_capture_stdout(package_list_argv(&self.cmd_path, 0))?;
        let entries = parse_android_package_list_stdout(&output.stdout);
        Ok(PackageUidSnapshot {
            coherent: !entries.is_empty(),
            entries,
            source: PackageProviderSource::CmdPackageList,
        })
    }
}

pub struct AndroidForegroundPackageProvider {
    pub(crate) config: AndroidForegroundConfig,
    exec: ExecRunner,
    cache: UidPackageCache,
    blocked_packages: BTreeSet<String>,
    blocked_uids: BTreeSet<u32>,
    blocklist_fingerprint: Option<BlocklistFingerprint>,
    packages_xml_fingerprint: Option<PackageFileStat>,
    foreground_changes: u64,
    last_real_package: Option<String>,
    package_cache_invalidated: bool,
    v1: CgroupV1CpusetSource,
    v2: CgroupV2EventsSource,
    watch_hint_paths: Vec<PathBuf>,
    v1_available: bool,
}

enum ForegroundResolution {
    Unavailable,
    Unknown,
    Resolved(ForegroundCandidate),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum CgroupV1SourceName {
    Cpuset,
    Cpuctl,
    Stune,
}

impl CgroupV1SourceName {
    fn as_str(self) -> &'static str {
        match self {
            Self::Cpuset => "cpuset",
            Self::Cpuctl => "cpuctl",
            Self::Stune => "stune",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct CgroupV1SourceCandidate {
    name: CgroupV1SourceName,
    path: PathBuf,
}

impl AndroidForegroundPackageProvider {
    pub fn new(config: AndroidForegroundConfig) -> io::Result<Self> {
        let exec = ExecRunner::new(ExecConfig::default());
        let cache = UidPackageCache::warm_from(&AndroidPackageListProvider::new(
            exec.clone(),
            config.cmd_path.clone(),
        ))
        .map_err(engine_io_error)?;
        let blocked_packages = load_or_create_blocklist_with_generated(
            &config.blocklist_path,
            || Ok(Vec::new()),
            resolve_generated_blocklist_packages(&exec, &config.cmd_path),
        )?
        .packages;
        let blocked_uids = exec
            .run_capture_stdout(crate::defaults::accessibility_service_query_argv(&config.cmd_path, 0))
            .map(|output| crate::defaults::parse_accessibility_uids_stdout(&output.stdout))
            .unwrap_or_default();
        let filter = android_foreground_filter(&blocked_packages, blocked_uids.clone());
        let blocklist_fingerprint = blocklist_fingerprint(&config.blocklist_path).ok();
        let mut statter = FsPackageFileStatProvider;
        let packages_xml_fingerprint = statter.stat_package_marker(&config.packages_xml_path).ok();
        
        let selected_v1 = select_cgroup_v1_source(&config.cgroup_v1_procs_path);
        log_selected_cgroup_v1_source(selected_v1.name);
        let v1 =
            CgroupV1CpusetSource::new(selected_v1.path, config.proc_root.clone(), filter.clone())
                .with_package_cache(cache.clone());
        let roots = effective_android_cgroup_v2_roots(&config)?;
        let v2 = CgroupV2EventsSource::new(roots, filter).with_package_cache(cache.clone());
        let v1_available = v1.is_available();
        let mut provider = Self {
            config,
            exec,
            cache,
            blocked_packages,
            blocked_uids,
            blocklist_fingerprint,
            packages_xml_fingerprint,
            foreground_changes: 0,
            last_real_package: None,
            package_cache_invalidated: false,
            v1,
            v2,
            watch_hint_paths: Vec::new(),
            v1_available,
        };
        provider.refresh_watch_hint_paths();
        Ok(provider)
    }

    pub fn foreground_package_changed(&mut self, package: Option<&str>) -> io::Result<bool> {
        let Some(package) = package.map(str::to_string) else {
            return Ok(false);
        };
        if self.last_real_package.as_deref() == Some(package.as_str()) {
            return Ok(false);
        }
        let counted = self.last_real_package.is_some();
        self.last_real_package = Some(package);
        if !counted {
            return Ok(false);
        }
        self.maybe_refresh_cache()?;
        Ok(true)
    }

    pub fn take_package_cache_invalidated(&mut self) -> bool {
        let invalidated = self.package_cache_invalidated;
        self.package_cache_invalidated = false;
        invalidated
    }

    fn maybe_refresh_cache(&mut self) -> io::Result<()> {
        self.foreground_changes = self.foreground_changes.saturating_add(1);
        if self.config.cache_check_foreground_changes == 0
            || self.foreground_changes % self.config.cache_check_foreground_changes != 0
        {
            return Ok(());
        }
        let mut statter = FsPackageFileStatProvider;
        let Ok(current) = statter.stat_package_marker(&self.config.packages_xml_path) else {
            return Ok(());
        };
        if self.packages_xml_fingerprint.as_ref() != Some(&current) {
            self.cache
                .update_from(&AndroidPackageListProvider::new(
                    self.exec.clone(),
                    self.config.cmd_path.clone(),
                ))
                .map_err(engine_io_error)?;
            self.v1.set_package_cache(self.cache.clone());
            self.v2.set_package_cache(self.cache.clone());
            self.package_cache_invalidated = true;
            self.blocked_uids = self.exec
                .run_capture_stdout(accessibility_service_query_argv(&self.config.cmd_path, 0))
                .map(|output| parse_accessibility_uids_stdout(&output.stdout))
                .unwrap_or_default();
            let filter = android_foreground_filter(&self.blocked_packages, self.blocked_uids.clone());
            self.v1.set_filter(filter.clone());
            self.v2.set_filter(filter);
        }
        self.packages_xml_fingerprint = Some(current);
        Ok(())
    }
    fn current_candidate(&mut self) -> io::Result<Option<ForegroundCandidate>> {
        self.maybe_reload_blocklist()?;
        self.current_candidate_after_blocklist_reload()
    }

    fn current_candidate_after_blocklist_reload(
        &mut self,
    ) -> io::Result<Option<ForegroundCandidate>> {
        let v1 = self.v1_resolution();
        if let Some(candidate) = self.allowed_candidate_from_resolution(v1) {
            return Ok(Some(candidate));
        }
        match self.v2_resolution() {
            ForegroundResolution::Resolved(candidate) => {
                if self.v1.filter.candidate_for_uid(candidate.source, candidate.pid, candidate.uid.unwrap_or(0), Some(&self.cache)).is_some() {
                    Ok(Some(candidate))
                } else {
                    self.activity_resolution()
                }
            }
            ForegroundResolution::Unknown => Ok(None),
            ForegroundResolution::Unavailable => self.activity_resolution(),
        }
    }

    fn maybe_reload_blocklist(&mut self) -> io::Result<()> {
        let Ok(current) = blocklist_fingerprint(&self.config.blocklist_path) else {
            return Ok(());
        };
        if self.blocklist_fingerprint.as_ref() == Some(&current) {
            return Ok(());
        }
        let content = std::fs::read_to_string(&self.config.blocklist_path)?;
        self.blocked_packages = parse_blocklist_file(&content);
        self.blocklist_fingerprint = Some(current);
        Ok(())
    }

    fn allowed_candidate_from_resolution(
        &self,
        resolution: ForegroundResolution,
    ) -> Option<ForegroundCandidate> {
        match resolution {
            ForegroundResolution::Resolved(candidate) if self.v1.filter.candidate_for_uid(candidate.source, candidate.pid, candidate.uid.unwrap_or(0), Some(&self.cache)).is_some() => {
                Some(candidate)
            }
            ForegroundResolution::Resolved(_)
            | ForegroundResolution::Unknown
            | ForegroundResolution::Unavailable => None,
        }
    }


    pub fn base_apk_path_for_package(&self, package: &str) -> Option<PathBuf> {
        self.cache
            .base_apk_path_for_package(package)
            .map(Path::to_path_buf)
    }

    pub fn cached_package_count(&self) -> usize {
        self.cache.entry_count()
    }

    pub fn cached_installed_game_targets(&self, game_list: &GameList) -> GameList {
        GameList::from_packages(
            game_list
                .packages()
                .filter(|package| self.cache.uid_for_package(package).is_some())
                .map(str::to_string)
                .collect(),
        )
    }

    fn v1_resolution(&mut self) -> ForegroundResolution {
        match self.v1.poll_current() {
            Ok(Some(candidate)) if candidate.identity_resolved => {
                self.set_v1_available(true);
                ForegroundResolution::Resolved(candidate)
            }
            Ok(_) => {
                self.set_v1_available(true);
                ForegroundResolution::Unknown
            }
            Err(_) => {
                self.set_v1_available(false);
                ForegroundResolution::Unavailable
            }
        }
    }

    fn v2_resolution(&mut self) -> ForegroundResolution {
        if !self.v2.is_available() {
            return ForegroundResolution::Unavailable;
        }
        match self.v2.poll_current() {
            Ok(Some(candidate)) if candidate.identity_resolved => {
                ForegroundResolution::Resolved(candidate)
            }
            Ok(_) => ForegroundResolution::Unknown,
            Err(_) => ForegroundResolution::Unavailable,
        }
    }

    fn activity_resolution(&mut self) -> io::Result<Option<ForegroundCandidate>> {
        let output = self
            .exec
            .run_capture_stdout(crate::defaults::activity_stack_list_argv(&self.config.cmd_path))
            .map_err(engine_io_error)?;
        Ok(
            resolve_activity_stdout(&output.stdout, &self.v1.filter)
                .package
                .map(|package| ForegroundCandidate {
                    source: ForegroundSourceKind::ActivityManager,
                    pid: None,
                    uid: None,
                    package: Some(package),
                    identity_resolved: true,
                }),
        )
    }

    fn recover_v1_if_available(
        &mut self,
    ) -> Result<Option<Option<ForegroundCandidate>>, coreshift_engine::EngineError> {
        if self.v1_available {
            return Ok(Some(None));
        }
        match self.v1.poll_current() {
            Ok(candidate) => {
                self.set_v1_available(true);
                Ok(Some(
                    candidate.filter(|candidate| {
                        candidate.identity_resolved && self.v1.filter.candidate_for_uid(candidate.source, candidate.pid, candidate.uid.unwrap_or(0), Some(&self.cache)).is_some()
                    }),
                ))
            }
            Err(_) => Ok(None),
        }
    }

    fn set_v1_available(&mut self, available: bool) {
        if self.v1_available == available {
            return;
        }
        self.v1_available = available;
        self.refresh_watch_hint_paths();
    }

    fn refresh_watch_hint_paths(&mut self) {
        self.watch_hint_paths = self.v1.watch_hint_paths().to_vec();
        if self.v1_available {
            return;
        }
        for path in self.v2.watch_hint_paths() {
            if !self.watch_hint_paths.contains(path) {
                self.watch_hint_paths.push(path.clone());
            }
        }
    }
}

impl ForegroundPackageProvider for AndroidForegroundPackageProvider {
    fn current_package(&mut self) -> io::Result<Option<String>> {
        Ok(self
            .current_candidate()?
            .and_then(|candidate| candidate.package))
    }
}

impl ForegroundSource for AndroidForegroundPackageProvider {
    fn kind(&self) -> ForegroundSourceKind {
        ForegroundSourceKind::Auto
    }

    fn poll_current(
        &mut self,
    ) -> Result<Option<ForegroundCandidate>, coreshift_engine::EngineError> {
        Ok(self.current_candidate()?)
    }

    fn handle_fs_event(
        &mut self,
        path: &Path,
    ) -> Result<Option<ForegroundCandidate>, coreshift_engine::EngineError> {
        self.handle_fs_event_with_mask(path, 0)
    }

    fn handle_fs_event_with_mask(
        &mut self,
        path: &Path,
        mask: u32,
    ) -> Result<Option<ForegroundCandidate>, coreshift_engine::EngineError> {
        if self.v1.watch_hint_paths().iter().any(|hint| hint == path) {
            return match self.v1.handle_fs_event_with_mask(path, mask) {
                Ok(candidate) => {
                    self.set_v1_available(true);
                    self.maybe_reload_blocklist()?;
                    let Some(candidate) = candidate.filter(|candidate| {
                        candidate.identity_resolved && self.v1.filter.candidate_for_uid(candidate.source, candidate.pid, candidate.uid.unwrap_or(0), Some(&self.cache)).is_some()
                    }) else {
                        return Ok(self.current_candidate_after_blocklist_reload()?);
                    };
                    Ok(Some(candidate))
                }
                Err(_) => {
                    self.set_v1_available(false);
                    Ok(self.current_candidate()?)
                }
            };
        }
        if self
            .v2
            .watch_hint_paths()
            .iter()
            .any(|hint| hint == path || path.parent() == Some(hint.as_path()))
        {
            if let Some(candidate) = self.recover_v1_if_available()? {
                return Ok(candidate);
            }
            self.maybe_reload_blocklist()?;
            if self.v2.handle_fs_event_with_mask(path, mask).is_ok() {
                return Ok(match self.v2_resolution() {
                    ForegroundResolution::Resolved(candidate) if self.v1.filter.candidate_for_uid(candidate.source, candidate.pid, candidate.uid.unwrap_or(0), Some(&self.cache)).is_some() => {
                        Some(candidate)
                    }
                    ForegroundResolution::Resolved(_) => self.activity_resolution()?,
                    ForegroundResolution::Unknown | ForegroundResolution::Unavailable => None,
                });
            }
            return Ok(self.activity_resolution()?);
        }
        self.poll_current()
    }

    fn watch_hint_paths(&self) -> &[PathBuf] {
        &self.watch_hint_paths
    }

    fn is_available(&self) -> bool {
        self.v1_available || self.v2.is_available()
    }

    fn watch_hint_mask(&self, path: &Path) -> u32 {
        if self.v1.watch_hint_paths().iter().any(|hint| hint == path) {
            return self.v1.watch_hint_mask(path);
        }
        self.v2.watch_hint_mask(path)
    }

    fn register_priority_fds(
        &self,
        reactor: &mut coreshift_engine::services::foreground::source::Reactor,
        registered_paths: &std::collections::BTreeSet<PathBuf>,
    ) -> Result<
        Vec<(
            coreshift_engine::services::foreground::source::Token,
            PathBuf,
        )>,
        coreshift_engine::EngineError,
    > {
        if self.v1_available {
            return Ok(Vec::new());
        }
        self.v2.register_priority_fds(reactor, registered_paths)
    }

    fn priority_hint_paths(&self) -> Vec<PathBuf> {
        if self.v1_available {
            return Vec::new();
        }
        self.v2.priority_hint_paths()
    }

    fn priority_hint_keys(&self) -> Vec<(PathBuf, u64)> {
        if self.v1_available {
            return Vec::new();
        }
        self.v2.priority_hint_keys()
    }

    fn handle_priority_event(
        &mut self,
        path: &Path,
    ) -> Result<Option<ForegroundCandidate>, coreshift_engine::EngineError> {
        if let Some(candidate) = self.recover_v1_if_available()? {
            return Ok(candidate);
        }
        self.maybe_reload_blocklist()?;
        if self.v2.handle_priority_event(path).is_ok() {
            return Ok(match self.v2_resolution() {
                ForegroundResolution::Resolved(candidate) if self.v1.filter.candidate_for_uid(candidate.source, candidate.pid, candidate.uid.unwrap_or(0), Some(&self.cache)).is_some() => {
                    Some(candidate)
                }
                ForegroundResolution::Resolved(_) => self.activity_resolution()?,
                ForegroundResolution::Unknown | ForegroundResolution::Unavailable => None,
            });
        }
        Ok(self.activity_resolution()?)
    }

    fn handle_stale_priority_event(
        &mut self,
        path: &Path,
    ) -> Result<Option<ForegroundCandidate>, coreshift_engine::EngineError> {
        if let Some(candidate) = self.recover_v1_if_available()? {
            return Ok(candidate);
        }
        self.maybe_reload_blocklist()?;
        if self.v2.handle_stale_priority_event(path).is_ok() {
            return Ok(match self.v2_resolution() {
                ForegroundResolution::Resolved(candidate) if self.v1.filter.candidate_for_uid(candidate.source, candidate.pid, candidate.uid.unwrap_or(0), Some(&self.cache)).is_some() => {
                    Some(candidate)
                }
                ForegroundResolution::Resolved(_) => self.activity_resolution()?,
                ForegroundResolution::Unknown | ForegroundResolution::Unavailable => None,
            });
        }
        Ok(self.activity_resolution()?)
    }

    fn unregister_priority_fd(
        &self,
        reactor: &coreshift_engine::services::foreground::source::Reactor,
        path: &Path,
    ) -> Result<bool, coreshift_engine::EngineError> {
        self.v2.unregister_priority_fd(reactor, path)
    }
}

fn android_foreground_filter(
    blocked_packages: &BTreeSet<String>,
    accessibility_uids: BTreeSet<u32>,
) -> ForegroundCandidateFilter {
    ForegroundCandidateFilter {
        blocked_packages: blocked_packages.clone(),
        accessibility_uids,
        uid_remainder_filter: Some(coreshift_engine::services::foreground::UidRemainderFilter {
            modulus: ANDROID_APP_ID_MODULUS,
            min_remainder: ANDROID_APP_ID_MIN,
        }),
        allow_system_uids: true, ..Default::default()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct BlocklistFingerprint {
    len: u64,
    modified_ns: u128,
}

fn blocklist_fingerprint(path: &Path) -> io::Result<BlocklistFingerprint> {
    let metadata = std::fs::metadata(path)?;
    let modified_ns = metadata
        .modified()
        .ok()
        .and_then(|modified| modified.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|duration| duration.as_nanos())
        .unwrap_or_default();
    Ok(BlocklistFingerprint {
        len: metadata.len(),
        modified_ns,
    })
}

fn resolve_generated_blocklist_packages(exec: &ExecRunner, cmd_path: &Path) -> BTreeSet<String> {
    let mut packages = resolve_input_method_blocklist_packages_with(|key| {
        exec.run_capture_stdout(input_method_setting_get_argv(cmd_path, key))
            .map(|output| output.stdout)
            .map_err(engine_io_error)
    });
    if let Some(package) = resolve_launcher_blocklist_package_with(|| {
        exec.run_capture_stdout(launcher_resolve_activity_argv(cmd_path))
            .map(|output| output.stdout)
            .map_err(engine_io_error)
    }) {
        packages.insert(package);
    }
    packages.insert("com.google.android.gms".to_string());
    packages.insert("com.google.android.googlequicksearchbox".to_string());
    packages.insert("com.aiworks.faceidservice".to_string());
    packages.insert("com.android.camera".to_string());
    packages.insert("com.google.android.apps.turbo".to_string());
    packages.insert("com.google.android.as".to_string());
    packages.insert("com.google.android.apps.wellbeing".to_string());
    packages.insert("com.android.vending".to_string());
    packages.insert("com.google.android.tts".to_string());
    packages.insert("com.pri.childrenspace".to_string());
    packages.insert("com.android.providers.calendar".to_string());
    packages.insert("com.android.providers.downloads".to_string());
    packages.insert("com.android.providers.media".to_string());
    packages
}

fn select_cgroup_v1_source(configured_cpuset_path: &Path) -> CgroupV1SourceCandidate {
    let candidates = cgroup_v1_source_candidates(configured_cpuset_path);
    candidates
        .iter()
        .find(|candidate| candidate.name == CgroupV1SourceName::Cpuset && cgroup_v1_source_usable(&candidate.path))
        .cloned()
        .unwrap_or_else(|| candidates[0].clone())
}

fn cgroup_v1_source_candidates(configured_cpuset_path: &Path) -> Vec<CgroupV1SourceCandidate> {
    let cpuset = CgroupV1SourceCandidate {
        name: CgroupV1SourceName::Cpuset,
        path: configured_cpuset_path.to_path_buf(),
    };
    let Some(top_app_dir) = configured_cpuset_path.parent() else {
        return vec![cpuset];
    };
    if top_app_dir.file_name().and_then(|name| name.to_str()) != Some("top-app") {
        return vec![cpuset];
    }
    let Some(controller_dir) = top_app_dir.parent() else {
        return vec![cpuset];
    };
    if controller_dir.file_name().and_then(|name| name.to_str()) != Some("cpuset") {
        return vec![cpuset];
    }
    let Some(cgroup_root) = controller_dir.parent() else {
        return vec![cpuset];
    };
    vec![
        cpuset,
        CgroupV1SourceCandidate {
            name: CgroupV1SourceName::Cpuctl,
            path: cgroup_root.join("cpuctl/top-app/cgroup.procs"),
        },
        CgroupV1SourceCandidate {
            name: CgroupV1SourceName::Stune,
            path: cgroup_root.join("stune/top-app/cgroup.procs"),
        },
    ]
}

fn cgroup_v1_source_usable(path: &Path) -> bool {
    let Ok(mut file) = std::fs::File::open(path) else {
        return false;
    };
    let mut probe = [0u8; 1];
    file.read(&mut probe).is_ok()
}

fn log_selected_cgroup_v1_source(source: CgroupV1SourceName) {
    if std::env::var_os("COREPOLICY_DEBUG").is_some() {
        coreshift_core::alog_info!("corepolicy", "foreground-cgroup-v1 source={}", source.as_str());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    static TEST_ID: AtomicU64 = AtomicU64::new(0);

    fn unique_test_id() -> String {
        format!(
            "{}-{}-{}",
            std::process::id(),
            TEST_ID.fetch_add(1, Ordering::Relaxed),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        )
    }

    fn temp_dir(name: &str) -> PathBuf {
        let path = std::env::temp_dir().join(format!(
            "coreshift-policy-v1-source-{name}-{}",
            unique_test_id()
        ));
        std::fs::create_dir(&path).unwrap();
        path
    }

    fn v1_paths(root: &Path) -> (PathBuf, PathBuf, PathBuf) {
        (
            root.join("cpuset/top-app/cgroup.procs"),
            root.join("cpuctl/top-app/cgroup.procs"),
            root.join("stune/top-app/cgroup.procs"),
        )
    }

    fn write_source(path: &Path) {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, "").unwrap();
    }

    #[test]
    fn cgroup_v1_selector_prefers_cpuset_over_cpuctl_and_stune() {
        let root = temp_dir("cpuset");
        let (cpuset, cpuctl, stune) = v1_paths(&root);
        write_source(&cpuset);
        write_source(&cpuctl);
        write_source(&stune);

        let selected = select_cgroup_v1_source(&cpuset);

        assert_eq!(selected.name, CgroupV1SourceName::Cpuset);
        assert_eq!(selected.path, cpuset);
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn cgroup_v1_selector_falls_back_to_cpuctl_before_stune() {
        let root = temp_dir("cpuctl");
        let (cpuset, cpuctl, stune) = v1_paths(&root);
        write_source(&cpuctl);
        write_source(&stune);

        let selected = select_cgroup_v1_source(&cpuset);

        assert_eq!(selected.name, CgroupV1SourceName::Cpuctl);
        assert_eq!(selected.path, cpuctl);
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn cgroup_v1_selector_falls_back_to_stune_after_cpuctl() {
        let root = temp_dir("stune");
        let (cpuset, _cpuctl, stune) = v1_paths(&root);
        write_source(&stune);

        let selected = select_cgroup_v1_source(&cpuset);

        assert_eq!(selected.name, CgroupV1SourceName::Stune);
        assert_eq!(selected.path, stune);
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn cgroup_v1_selector_skips_unusable_cpuset() {
        let root = temp_dir("unusable-cpuset");
        let (cpuset, cpuctl, _stune) = v1_paths(&root);
        std::fs::create_dir_all(cpuset.parent().unwrap()).unwrap();
        std::fs::create_dir(&cpuset).unwrap();
        write_source(&cpuctl);

        let selected = select_cgroup_v1_source(&cpuset);

        assert_eq!(selected.name, CgroupV1SourceName::Cpuctl);
        assert_eq!(selected.path, cpuctl);
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn cgroup_v1_selector_returns_cpuset_when_no_source_usable() {
        let root = temp_dir("none");
        let (cpuset, _cpuctl, _stune) = v1_paths(&root);

        let selected = select_cgroup_v1_source(&cpuset);

        assert_eq!(selected.name, CgroupV1SourceName::Cpuset);
        assert_eq!(selected.path, cpuset);
        assert!(!cgroup_v1_source_usable(&selected.path));
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn cgroup_v1_selector_ignores_tasks_without_cgroup_procs() {
        let root = temp_dir("tasks-ignored");
        let (cpuset, cpuctl, stune) = v1_paths(&root);
        let cpuctl_tasks = cpuctl.parent().unwrap().join("tasks");
        let stune_tasks = stune.parent().unwrap().join("tasks");
        write_source(&cpuctl_tasks);
        write_source(&stune_tasks);

        let selected = select_cgroup_v1_source(&cpuset);

        assert_eq!(selected.name, CgroupV1SourceName::Cpuset);
        assert_eq!(selected.path, cpuset);
        assert!(!cgroup_v1_source_usable(&selected.path));
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn cgroup_v1_selector_keeps_custom_path_single_source() {
        let root = temp_dir("custom");
        let configured = root.join("custom.procs");
        write_source(&configured);

        let selected = select_cgroup_v1_source(&configured);

        assert_eq!(selected.name, CgroupV1SourceName::Cpuset);
        assert_eq!(selected.path, configured);
        let _ = std::fs::remove_dir_all(root);
    }
}
