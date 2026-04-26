use crate::features::preload::RuntimeAbi;
use crate::runtime::status::AppIndexStatusFile;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Condvar, Mutex, RwLock};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

const DATA_APP_DIR: &str = "/data/app";
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

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, Default, PartialEq)]
pub struct AppIndexStats {
    pub built_ms: u64,
    pub rebuild_ms: u64,
    pub duration_ms: u64,
    pub packages: usize,
    pub ready: bool,
    pub stale: bool,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct AppIndexMetrics {
    pub rebuild_success_count: u64,
    pub rebuild_fail_count: u64,
    pub last_error: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq)]
struct AppIndexState {
    stats: AppIndexStats,
    metrics: AppIndexMetrics,
    entries: HashMap<String, AppPathEntry>,
}

#[derive(Debug, Clone)]
struct RebuildRequest {
    generation: u64,
    root: PathBuf,
    abi: RuntimeAbi,
}

#[derive(Debug, Default)]
struct WorkerControl {
    requested_generation: u64,
    completed_generation: u64,
    request: Option<RebuildRequest>,
    shutting_down: bool,
}

struct AppIndexerInner {
    state: RwLock<AppIndexState>,
    control: Mutex<WorkerControl>,
    cv: Condvar,
}

pub struct AppIndexer {
    inner: Arc<AppIndexerInner>,
}

impl Default for AppIndexer {
    fn default() -> Self {
        Self::new()
    }
}

impl AppIndexer {
    pub fn new() -> Self {
        let inner = Arc::new(AppIndexerInner {
            state: RwLock::new(AppIndexState::default()),
            control: Mutex::new(WorkerControl::default()),
            cv: Condvar::new(),
        });

        let worker_inner = Arc::clone(&inner);
        std::thread::Builder::new()
            .name("app-indexer".to_string())
            .spawn(move || worker_loop(worker_inner))
            .expect("failed to start app indexer worker");

        Self { inner }
    }

    pub fn request_rebuild(&self, abi: RuntimeAbi) -> u64 {
        self.request_rebuild_from_root(Path::new(DATA_APP_DIR), abi)
    }

    fn request_rebuild_from_root(&self, data_app: &Path, abi: RuntimeAbi) -> u64 {
        let mut control = self.inner.control.lock().unwrap();
        control.requested_generation += 1;
        let generation = control.requested_generation;
        control.request = Some(RebuildRequest {
            generation,
            root: data_app.to_path_buf(),
            abi,
        });
        self.inner.cv.notify_one();
        generation
    }

    pub fn get_candidates(&self, package: &str) -> Option<Arc<[PathBuf]>> {
        self.inner
            .state
            .read()
            .unwrap()
            .entries
            .get(package)
            .map(|entry| Arc::clone(&entry.candidates))
    }

    pub fn snapshot_stats(&self) -> AppIndexStats {
        self.inner.state.read().unwrap().stats.clone()
    }

    pub fn snapshot_metrics(&self) -> AppIndexMetrics {
        self.inner.state.read().unwrap().metrics.clone()
    }

    #[cfg(test)]
    fn wait_for_generation(&self, generation: u64, timeout: Duration) -> bool {
        let deadline = Instant::now() + timeout;
        loop {
            if self.inner.control.lock().unwrap().completed_generation >= generation {
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
        self.inner.state.write().unwrap().stats.rebuild_ms = rebuild_ms;
    }
}

impl Drop for AppIndexer {
    fn drop(&mut self) {
        if Arc::strong_count(&self.inner) == 1 {
            let mut control = self.inner.control.lock().unwrap();
            control.shutting_down = true;
            self.inner.cv.notify_one();
        }
    }
}

fn worker_loop(inner: Arc<AppIndexerInner>) {
    let mut last_written_status = None;

    loop {
        let request = {
            let mut control = inner.control.lock().unwrap();
            while control.request.is_none() && !control.shutting_down {
                control = inner.cv.wait(control).unwrap();
            }
            if control.shutting_down {
                return;
            }
            control.request.clone().unwrap()
        };

        if !begin_rebuild(&inner, &mut last_written_status) {
            mark_generation_completed(&inner, request.generation);
            continue;
        }

        let start = Instant::now();
        match build_index_entries(&request.root, request.abi) {
            Ok(entries) => finish_rebuild_success(
                &inner,
                request.generation,
                start,
                entries,
                &mut last_written_status,
            ),
            Err(error) => finish_rebuild_failure(
                &inner,
                request.generation,
                start,
                &error,
                &mut last_written_status,
            ),
        }
    }
}

fn begin_rebuild(
    inner: &Arc<AppIndexerInner>,
    last_written_status: &mut Option<AppIndexStatusFile>,
) -> bool {
    let rebuild_ms = unix_time_ms();
    let mut state = inner.state.write().unwrap();
    if rebuild_ms.saturating_sub(state.stats.rebuild_ms) < REBUILD_DEBOUNCE.as_millis() as u64 {
        return false;
    }
    state.stats.rebuild_ms = rebuild_ms;
    state.stats.stale = true;
    state.stats.ready = false;
    let status = persisted_status(&state);
    drop(state);
    let _ = status.write_if_changed(last_written_status);
    true
}

fn finish_rebuild_success(
    inner: &Arc<AppIndexerInner>,
    generation: u64,
    start: Instant,
    entries: HashMap<String, AppPathEntry>,
    last_written_status: &mut Option<AppIndexStatusFile>,
) {
    let status = {
        let built_ms = unix_time_ms();
        let mut state = inner.state.write().unwrap();
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
    mark_generation_completed(inner, generation);
}

fn finish_rebuild_failure(
    inner: &Arc<AppIndexerInner>,
    generation: u64,
    start: Instant,
    error: &std::io::Error,
    last_written_status: &mut Option<AppIndexStatusFile>,
) {
    let status = {
        let mut state = inner.state.write().unwrap();
        state.stats.duration_ms = start.elapsed().as_millis() as u64;
        state.stats.ready = false;
        state.stats.stale = true;
        state.metrics.rebuild_fail_count += 1;
        state.metrics.last_error = Some(error.to_string());
        persisted_status(&state)
    };
    let _ = status.write_if_changed(last_written_status);
    mark_generation_completed(inner, generation);
}

fn mark_generation_completed(inner: &Arc<AppIndexerInner>, generation: u64) {
    let mut control = inner.control.lock().unwrap();
    control.completed_generation = generation;
    if control.request.as_ref().map(|req| req.generation) == Some(generation) {
        control.request = None;
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
        enabled: true,
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

    #[test]
    fn indexer_builds_non_empty_index_from_fake_data_app() {
        let temp =
            std::env::temp_dir().join(format!("coreshift_test_indexer_{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp);
        create_index_fixture(&temp, "~~randomized", "com.termux-hashvalue");

        let indexer = AppIndexer::new();
        let generation = indexer.request_rebuild_from_root(&temp, RuntimeAbi::Arm64);
        assert!(indexer.wait_for_generation(generation, Duration::from_secs(2)));

        let candidates = indexer.get_candidates("com.termux").unwrap();
        assert_eq!(candidates.len(), 2);
        assert!(candidates[0].ends_with("base.vdex"));
        assert!(candidates[1].ends_with("libtermux.so"));

        let stats = indexer.snapshot_stats();
        assert!(stats.built_ms > 0);
        assert_eq!(stats.packages, 1);
        assert!(stats.ready);
        assert!(!stats.stale);

        let metrics = indexer.snapshot_metrics();
        assert_eq!(metrics.rebuild_success_count, 1);
        assert_eq!(metrics.rebuild_fail_count, 0);
        assert_eq!(metrics.last_error, None);

        let _ = fs::remove_dir_all(&temp);
    }

    #[test]
    fn rebuild_is_debounced() {
        let temp = std::env::temp_dir().join(format!(
            "coreshift_test_indexer_debounce_{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&temp);
        create_index_fixture(&temp, "~~randomized", "com.termux-hashvalue");

        let indexer = AppIndexer::new();
        let first = indexer.request_rebuild_from_root(&temp, RuntimeAbi::Arm64);
        assert!(indexer.wait_for_generation(first, Duration::from_secs(2)));

        let second = indexer.request_rebuild_from_root(&temp, RuntimeAbi::Arm64);
        assert!(indexer.wait_for_generation(second, Duration::from_secs(2)));

        let metrics = indexer.snapshot_metrics();
        assert_eq!(metrics.rebuild_success_count, 1);

        let _ = fs::remove_dir_all(&temp);
    }

    #[test]
    fn stale_transitions_on_failed_rebuild() {
        let temp = std::env::temp_dir().join(format!(
            "coreshift_test_indexer_stale_{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&temp);
        create_index_fixture(&temp, "~~randomized", "com.termux-hashvalue");

        let indexer = AppIndexer::new();
        let first = indexer.request_rebuild_from_root(&temp, RuntimeAbi::Arm64);
        assert!(indexer.wait_for_generation(first, Duration::from_secs(2)));

        indexer.force_last_rebuild_ms(0);
        fs::remove_dir_all(&temp).unwrap();

        let second = indexer.request_rebuild_from_root(&temp, RuntimeAbi::Arm64);
        assert!(indexer.wait_for_generation(second, Duration::from_secs(2)));

        let stats = indexer.snapshot_stats();
        assert!(stats.stale);
        assert!(!stats.ready);

        let metrics = indexer.snapshot_metrics();
        assert_eq!(metrics.rebuild_fail_count, 1);
        assert!(metrics.last_error.is_some());
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

        let indexer = AppIndexer::new();
        let generation = indexer.request_rebuild_from_root(&temp, RuntimeAbi::Arm64);
        assert!(indexer.wait_for_generation(generation, Duration::from_secs(2)));

        let stats = indexer.snapshot_stats();
        assert_eq!(stats.packages, 2);
        assert!(stats.ready);
        assert!(!stats.stale);
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

        let indexer = AppIndexer::new();
        let generation = indexer.request_rebuild_from_root(&temp, RuntimeAbi::Arm64);
        assert!(indexer.wait_for_generation(generation, Duration::from_secs(2)));

        assert!(indexer.get_candidates("com.example.app").is_some());
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
