use crate::features::preload::RuntimeAbi;
use crate::runtime::logging;
use crate::runtime::status::AppIndexStatusFile;
use coreshift_lowlevel::inotify::{PACKAGE_FILE_MASK, add_watch, read_events};
use coreshift_lowlevel::reactor::{Fd, Reactor};
use std::collections::HashMap;
use std::fs;
use std::os::fd::IntoRawFd;
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, MutexGuard, RwLock, RwLockReadGuard, RwLockWriteGuard};
use std::thread::JoinHandle;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

const DATA_APP_DIR: &str = "/data/app";
const PACKAGES_LIST_PATH: &str = "/data/system/packages.list";
const MAX_FILES_PER_PACKAGE: usize = 64;
const MAX_BYTES_PER_PACKAGE: u64 = 256 * 1024 * 1024;
const MAX_DEPTH_UNDER_PKG: usize = 4;
const REBUILD_DEBOUNCE: Duration = Duration::from_secs(5);
const SCAN_TIME_BUDGET: Duration = Duration::from_secs(15);

#[derive(Debug, Clone, PartialEq)]
struct IndexedCandidate {
    path: PathBuf,
    priority: u8,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AppPathEntry {
    pub candidates: Arc<[PathBuf]>,
}

#[derive(Debug, Clone, Default, PartialEq)]
struct AppIndexStats {
    built_ms: u64,
    rebuild_ms: u64,
    duration_ms: u64,
    packages: usize,
    ready: bool,
    stale: bool,
}

#[derive(Debug, Clone, Default, PartialEq)]
struct AppIndexMetrics {
    rebuild_success_count: u64,
    rebuild_fail_count: u64,
    last_error: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq)]
struct AppIndexState {
    stats: AppIndexStats,
    metrics: AppIndexMetrics,
    entries: HashMap<String, AppPathEntry>,
}

#[derive(Debug, Default)]
struct WorkerControl {
    requested_generation: u64,
    completed_generation: u64,
    shutdown: bool,
}

struct AppIndexInner {
    state: RwLock<AppIndexState>,
    control: Mutex<WorkerControl>,
}

pub struct AppIndexFeature {
    enabled: bool,
    inner: Arc<AppIndexInner>,
    command_writer: Option<UnixStream>,
    worker: Option<JoinHandle<()>>,
}

impl AppIndexFeature {
    pub fn new(enabled: bool, abi: RuntimeAbi) -> Self {
        let inner = Arc::new(AppIndexInner {
            state: RwLock::new(AppIndexState::default()),
            control: Mutex::new(WorkerControl::default()),
        });

        if !enabled {
            return Self {
                enabled,
                inner,
                command_writer: None,
                worker: None,
            };
        }

        let (command_reader, command_writer) =
            UnixStream::pair().expect("failed to create command socket pair");
        command_reader
            .set_nonblocking(true)
            .expect("failed to set command reader nonblocking");
        command_writer
            .set_nonblocking(true)
            .expect("failed to set command writer nonblocking");

        {
            let mut control = lock_control(&inner, "initial request generation");
            control.requested_generation = 1;
        }

        let worker_inner = Arc::clone(&inner);
        let worker = std::thread::Builder::new()
            .name("app-index-feature".to_string())
            .spawn(move || worker_loop(worker_inner, abi, command_reader))
            .expect("failed to start app index worker");

        Self {
            enabled,
            inner,
            command_writer: Some(command_writer),
            worker: Some(worker),
        }
    }

    pub fn name(&self) -> &'static str {
        "app_index"
    }

    pub fn enabled(&self) -> bool {
        self.enabled
    }

    pub fn get_candidates(&self, package: &str) -> Option<Arc<[PathBuf]>> {
        lock_state_read(&self.inner, "get_candidates")
            .entries
            .get(package)
            .map(|entry| Arc::clone(&entry.candidates))
    }

    pub fn shutdown(&mut self) {
        if !self.enabled {
            return;
        }

        if let Ok(mut control) = self.inner.control.lock() {
            control.shutdown = true;
        }
        self.send_command(b's');

        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
        self.command_writer = None;
    }

    #[cfg(test)]
    fn request_rebuild_from_root(&self, data_app: &Path) -> u64 {
        let generation = {
            let mut control = lock_control(&self.inner, "request_rebuild_from_root");
            control.requested_generation += 1;
            control.requested_generation
        };
        self.send_command(b'r');
        let _ = data_app;
        generation
    }

    #[cfg(test)]
    fn wait_for_generation(&self, generation: u64, timeout: Duration) -> bool {
        let deadline = Instant::now() + timeout;
        loop {
            if lock_control(&self.inner, "wait_for_generation").completed_generation >= generation {
                return true;
            }
            if Instant::now() >= deadline {
                return false;
            }
            std::thread::sleep(Duration::from_millis(10));
        }
    }

    #[cfg(test)]
    fn force_last_rebuild_ms(&self, rebuild_ms: u64) {
        lock_state_write(&self.inner, "force_last_rebuild_ms")
            .stats
            .rebuild_ms = rebuild_ms;
    }

    #[cfg(test)]
    fn snapshot_metrics(&self) -> (u64, u64) {
        let state = lock_state_read(&self.inner, "snapshot_metrics");
        (
            state.metrics.rebuild_success_count,
            state.metrics.rebuild_fail_count,
        )
    }

    fn send_command(&self, byte: u8) {
        if let Some(writer) = &self.command_writer {
            let _ = std::io::Write::write_all(&mut &*writer, &[byte]);
        }
    }
}

impl Drop for AppIndexFeature {
    fn drop(&mut self) {
        self.shutdown();
    }
}

fn worker_loop(inner: Arc<AppIndexInner>, abi: RuntimeAbi, command_reader: UnixStream) {
    let mut last_written_status = None;
    let mut reactor = Reactor::new().expect("failed to create app index reactor");
    let command_fd = Fd::new(command_reader.into_raw_fd(), "app_index_command")
        .expect("failed to wrap app index command fd");
    let command_token = reactor
        .add(&command_fd, true, false)
        .expect("failed to register app index command fd");

    let mut inotify_token = None;
    let mut inotify_fd = None;
    let mut package_watch = None;

    match reactor.setup_inotify() {
        Ok(fd) => {
            inotify_token = reactor.inotify_token;
            if let Ok(wd) = add_watch(&fd, PACKAGES_LIST_PATH, PACKAGE_FILE_MASK) {
                package_watch = Some(wd);
            } else {
                update_failure_status(
                    &inner,
                    "failed to watch packages.list".to_string(),
                    0,
                    &mut last_written_status,
                );
            }
            inotify_fd = Some(fd);
        }
        Err(error) => {
            update_failure_status(
                &inner,
                format!("failed to setup inotify: {}", error),
                0,
                &mut last_written_status,
            );
        }
    }

    let root_override = {
        #[cfg(test)]
        {
            std::env::var_os("COREPOLICY_TEST_DATA_APP_ROOT").map(PathBuf::from)
        }
        #[cfg(not(test))]
        {
            None
        }
    };
    let data_root = root_override.unwrap_or_else(|| PathBuf::from(DATA_APP_DIR));
    let mut events = Vec::with_capacity(8);

    loop {
        if try_rebuild(&inner, &data_root, abi, &mut last_written_status) {
            continue;
        }

        events.clear();
        if reactor.wait(&mut events, 8, -1).is_err() {
            continue;
        }

        let mut package_change = false;
        let mut command_ready = false;
        for event in &events {
            if Some(event.token) == inotify_token {
                package_change = true;
            }
            if event.token == command_token {
                command_ready = true;
            }
        }

        if command_ready {
            drain_command_fd(&command_fd);
            if lock_control(&inner, "worker shutdown check").shutdown {
                break;
            }
        }

        if package_change
            && let Some(package_watch) = package_watch
            && let Some(inotify_fd) = &inotify_fd
            && let Ok(inotify_events) = read_events(inotify_fd)
            && inotify_events.iter().any(|event| event.wd == package_watch)
        {
            let mut control = lock_control(&inner, "package watch generation bump");
            control.requested_generation += 1;
        }
    }
}

fn try_rebuild(
    inner: &Arc<AppIndexInner>,
    data_root: &Path,
    abi: RuntimeAbi,
    last_written_status: &mut Option<AppIndexStatusFile>,
) -> bool {
    let requested_generation = {
        let control = lock_control(inner, "try_rebuild requested generation");
        if control.requested_generation <= control.completed_generation {
            return false;
        }
        control.requested_generation
    };

    let rebuild_ms = unix_time_ms();
    {
        let mut state = lock_state_write(inner, "try_rebuild start");
        if rebuild_ms.saturating_sub(state.stats.rebuild_ms) < REBUILD_DEBOUNCE.as_millis() as u64 {
            let mut control = lock_control(inner, "try_rebuild debounce");
            control.completed_generation = requested_generation;
            return false;
        }
        state.stats.rebuild_ms = rebuild_ms;
        state.stats.stale = true;
        state.stats.ready = false;
        let status = persisted_status(&state);
        drop(state);
        let _ = status.write_if_changed(last_written_status);
    }

    let start = Instant::now();
    match build_index_entries(data_root, abi) {
        Ok(entries) => {
            let status = {
                let built_ms = unix_time_ms();
                let mut state = lock_state_write(inner, "try_rebuild success");
                state.entries = entries;
                state.stats.built_ms = built_ms;
                state.stats.duration_ms = start.elapsed().as_millis() as u64;
                state.stats.packages = state.entries.len();
                state.stats.ready = built_ms > 0;
                state.stats.stale = false;
                state.metrics.rebuild_success_count += 1;
                state.metrics.last_error = None;
                persisted_status(&state)
            };
            let _ = status.write_if_changed(last_written_status);
        }
        Err(error) => {
            update_failure_status(
                inner,
                error.to_string(),
                start.elapsed().as_millis() as u64,
                last_written_status,
            );
        }
    }

    let mut control = lock_control(inner, "try_rebuild completion");
    control.completed_generation = requested_generation;
    true
}

fn update_failure_status(
    inner: &Arc<AppIndexInner>,
    error: String,
    duration_ms: u64,
    last_written_status: &mut Option<AppIndexStatusFile>,
) {
    let status = {
        let mut state = lock_state_write(inner, "update_failure_status");
        state.stats.duration_ms = duration_ms;
        state.stats.ready = false;
        state.stats.stale = true;
        state.metrics.rebuild_fail_count += 1;
        state.metrics.last_error = Some(error);
        persisted_status(&state)
    };
    let _ = status.write_if_changed(last_written_status);
}

fn lock_control<'a>(inner: &'a AppIndexInner, context: &str) -> MutexGuard<'a, WorkerControl> {
    match inner.control.lock() {
        Ok(guard) => guard,
        Err(poisoned) => {
            logging::error(&format!(
                "AppIndexFeature control lock poisoned during {}; recovering",
                context
            ));
            poisoned.into_inner()
        }
    }
}

fn lock_state_read<'a>(
    inner: &'a AppIndexInner,
    context: &str,
) -> RwLockReadGuard<'a, AppIndexState> {
    match inner.state.read() {
        Ok(guard) => guard,
        Err(poisoned) => {
            logging::error(&format!(
                "AppIndexFeature state read lock poisoned during {}; recovering",
                context
            ));
            poisoned.into_inner()
        }
    }
}

fn lock_state_write<'a>(
    inner: &'a AppIndexInner,
    context: &str,
) -> RwLockWriteGuard<'a, AppIndexState> {
    match inner.state.write() {
        Ok(guard) => guard,
        Err(poisoned) => {
            logging::error(&format!(
                "AppIndexFeature state write lock poisoned during {}; recovering",
                context
            ));
            poisoned.into_inner()
        }
    }
}

fn drain_command_fd(fd: &Fd) {
    let mut buf = [0u8; 64];
    loop {
        match fd.read_slice(&mut buf) {
            Ok(Some(0)) | Ok(None) | Err(_) => break,
            Ok(Some(_)) => continue,
        }
    }
}

fn build_index_entries(
    data_app: &Path,
    abi: RuntimeAbi,
) -> std::io::Result<HashMap<String, AppPathEntry>> {
    let mut entries = HashMap::new();
    let deadline = Instant::now() + SCAN_TIME_BUDGET;

    let level1_entries = fs::read_dir(data_app)?;
    for level1 in level1_entries.flatten() {
        if Instant::now() >= deadline {
            break;
        }

        let level1_path = level1.path();
        if !level1_path.is_dir() {
            continue;
        }

        if let Ok(package_entries) = fs::read_dir(&level1_path) {
            for package_entry in package_entries.flatten() {
                if Instant::now() >= deadline {
                    break;
                }

                let package_dir = package_entry.path();
                if !package_dir.is_dir() {
                    continue;
                }

                let Some(package_name) = package_name_from_dir(&package_dir) else {
                    continue;
                };

                let ordered_candidates = collect_package_candidates(&package_dir, deadline, abi)
                    .into_iter()
                    .map(|candidate| candidate.path)
                    .collect::<Vec<_>>();

                if !ordered_candidates.is_empty() {
                    entries.insert(
                        package_name,
                        AppPathEntry {
                            candidates: Arc::from(ordered_candidates),
                        },
                    );
                }
            }
        }
    }

    Ok(entries)
}

fn package_name_from_dir(package_dir: &Path) -> Option<String> {
    let name = package_dir.file_name()?.to_str()?;
    let split = name.rfind('-')?;
    let package = &name[..split];
    if package.is_empty() || !package.contains('.') {
        return None;
    }
    Some(package.to_string())
}

fn collect_package_candidates(
    package_dir: &Path,
    deadline: Instant,
    abi: RuntimeAbi,
) -> Vec<IndexedCandidate> {
    let mut candidates = Vec::new();
    let mut total_bytes = 0;
    collect_files_recursive(
        package_dir,
        &mut candidates,
        &mut total_bytes,
        0,
        deadline,
        abi,
    );
    candidates.sort_by_key(|candidate| candidate.priority);
    candidates
}

fn collect_files_recursive(
    dir: &Path,
    candidates: &mut Vec<IndexedCandidate>,
    total_bytes: &mut u64,
    depth: usize,
    deadline: Instant,
    abi: RuntimeAbi,
) {
    if Instant::now() >= deadline
        || candidates.len() >= MAX_FILES_PER_PACKAGE
        || depth > MAX_DEPTH_UNDER_PKG
    {
        return;
    }

    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            if Instant::now() >= deadline {
                break;
            }

            let path = entry.path();
            let name = path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_lowercase();

            if name.contains("cache")
                || name.contains("tmp")
                || name.contains("log")
                || name.contains("metadata")
            {
                continue;
            }

            if path.is_dir() {
                collect_files_recursive(&path, candidates, total_bytes, depth + 1, deadline, abi);
                continue;
            }

            if candidates.len() >= MAX_FILES_PER_PACKAGE {
                break;
            }

            if name.ends_with(".apk") || name.ends_with(".dm") || name.contains("digests") {
                continue;
            }

            if let Some(priority) = get_preload_priority(&name, &path, abi)
                && let Ok(metadata) = fs::metadata(&path)
            {
                let size = metadata.len();
                if *total_bytes + size <= MAX_BYTES_PER_PACKAGE {
                    *total_bytes += size;
                    candidates.push(IndexedCandidate { path, priority });
                }
            }
        }
    }
}

fn get_preload_priority(name: &str, path: &Path, abi: RuntimeAbi) -> Option<u8> {
    let path_str = path.to_string_lossy();
    let is_odex = name.ends_with(".vdex") || name.ends_with(".odex") || name.ends_with(".art");
    let is_lib = name.ends_with(".so");

    match abi {
        RuntimeAbi::Arm64 => {
            if path_str.contains("/oat/arm64/") && is_odex {
                return Some(1);
            }
            if path_str.contains("/oat/arm/") && is_odex {
                return Some(3);
            }
            if path_str.contains("/lib/arm64/") && is_lib {
                return Some(2);
            }
            if path_str.contains("/lib/arm/") && is_lib {
                return Some(4);
            }
        }
        RuntimeAbi::Arm32 => {
            if path_str.contains("/oat/arm/") && is_odex {
                return Some(1);
            }
            if path_str.contains("/lib/arm/") && is_lib {
                return Some(2);
            }
        }
    }

    None
}

fn unix_time_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

fn persisted_status(state: &AppIndexState) -> AppIndexStatusFile {
    AppIndexStatusFile {
        schema_version: 1,
        ready: state.stats.ready,
        packages: state.stats.packages,
        built_ms: state.stats.built_ms,
        rebuild_ms: state.stats.rebuild_ms,
        duration_ms: state.stats.duration_ms,
        stale: state.stats.stale,
        rebuild_success_count: state.metrics.rebuild_success_count,
        rebuild_fail_count: state.metrics.rebuild_fail_count,
        last_error: state.metrics.last_error.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_index_fixture(root: &Path, shard_dir_name: &str, package_dir_name: &str) {
        let pkg_dir = root.join(shard_dir_name).join(package_dir_name);
        fs::create_dir_all(pkg_dir.join("lib/arm64")).unwrap();
        fs::create_dir_all(pkg_dir.join("oat/arm64")).unwrap();
        fs::write(pkg_dir.join("base.apk"), b"apk").unwrap();
        fs::write(pkg_dir.join("lib/arm64/libtermux.so"), b"lib").unwrap();
        fs::write(pkg_dir.join("oat/arm64/base.vdex"), b"vdex").unwrap();
    }

    fn with_test_data_root(root: &Path, f: impl FnOnce()) {
        unsafe {
            std::env::set_var("COREPOLICY_TEST_DATA_APP_ROOT", root);
        }
        f();
        unsafe {
            std::env::remove_var("COREPOLICY_TEST_DATA_APP_ROOT");
        }
    }

    #[test]
    fn worker_rebuild_does_not_block_requester() {
        let temp =
            std::env::temp_dir().join(format!("coreshift_test_indexer_{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp);
        create_index_fixture(&temp, "~~randomized", "com.termux-hashvalue");

        with_test_data_root(&temp, || {
            let feature = AppIndexFeature::new(true, RuntimeAbi::Arm64);
            assert!(feature.wait_for_generation(1, Duration::from_secs(2)));
            feature.force_last_rebuild_ms(0);
            let start = Instant::now();
            let generation = feature.request_rebuild_from_root(&temp);
            assert!(start.elapsed() < Duration::from_millis(50));
            assert!(feature.wait_for_generation(generation, Duration::from_secs(2)));
            let candidates = feature.get_candidates("com.termux").unwrap();
            assert_eq!(candidates.len(), 2);
            let (success, _) = feature.snapshot_metrics();
            assert!(success >= 1);
        });

        let _ = fs::remove_dir_all(&temp);
    }

    #[test]
    fn daemon_shutdown_cleanly_stops_worker_thread() {
        let mut feature = AppIndexFeature::new(true, RuntimeAbi::Arm64);
        feature.shutdown();
        assert!(feature.worker.is_none());
    }

    #[test]
    fn rebuild_is_debounced() {
        let temp = std::env::temp_dir().join(format!(
            "coreshift_test_indexer_debounce_{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&temp);
        create_index_fixture(&temp, "~~randomized", "com.termux-hashvalue");

        with_test_data_root(&temp, || {
            let feature = AppIndexFeature::new(true, RuntimeAbi::Arm64);
            assert!(feature.wait_for_generation(1, Duration::from_secs(2)));
            feature.force_last_rebuild_ms(0);
            let second = feature.request_rebuild_from_root(&temp);
            assert!(feature.wait_for_generation(second, Duration::from_secs(2)));
            let (success, _) = feature.snapshot_metrics();
            assert!(success >= 1);
        });

        let _ = fs::remove_dir_all(&temp);
    }

    #[test]
    fn stale_transitions_on_failed_rebuild() {
        let inner = Arc::new(AppIndexInner {
            state: RwLock::new(AppIndexState::default()),
            control: Mutex::new(WorkerControl::default()),
        });
        let mut last_written_status = None;

        update_failure_status(
            &inner,
            "forced failure".to_string(),
            12,
            &mut last_written_status,
        );

        let status = persisted_status(&inner.state.read().unwrap());
        assert!(status.stale);
        assert!(!status.ready);
        assert_eq!(status.duration_ms, 12);
        assert!(status.last_error.is_some());
    }

    #[test]
    fn package_count_stats_are_lightweight() {
        let temp = std::env::temp_dir().join(format!(
            "coreshift_test_indexer_count_{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&temp);
        create_index_fixture(&temp, "~~a", "com.a-hash");
        create_index_fixture(&temp, "~~b", "com.b-hash");

        let entries = build_index_entries(&temp, RuntimeAbi::Arm64).unwrap();
        assert_eq!(entries.len(), 2);
        assert!(entries.contains_key("com.a"));
        assert!(entries.contains_key("com.b"));
    }

    #[test]
    fn package_dir_naming_variants_are_supported() {
        let temp = std::env::temp_dir().join(format!(
            "coreshift_test_indexer_variants_{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&temp);
        create_index_fixture(&temp, "~~a", "com.example.app-123");
        create_index_fixture(&temp, "~~b", "com.example.app-ABCDEF==");
        create_index_fixture(&temp, "~~c", "com.example.app-split-name-v2");

        let entries = build_index_entries(&temp, RuntimeAbi::Arm64).unwrap();
        assert!(entries.contains_key("com.example.app"));
        assert_eq!(
            package_name_from_dir(Path::new("com.example.app-123")),
            Some("com.example.app".to_string())
        );
        assert_eq!(
            package_name_from_dir(Path::new("com.example.app-ABCDEF==")),
            Some("com.example.app".to_string())
        );
        assert_eq!(
            package_name_from_dir(Path::new("com.example.app-split-name-v2")),
            Some("com.example.app-split-name".to_string())
        );
    }
}
