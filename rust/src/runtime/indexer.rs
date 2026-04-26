use crate::features::preload::RuntimeAbi;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::path::PathBuf;
use std::sync::RwLock;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

const DATA_APP_DIR: &str = "/data/app";
const MAX_DIRS_VISITED: usize = 256;
const MAX_FILES_PER_PACKAGE: usize = 64;
const MAX_BYTES_PER_PACKAGE: u64 = 256 * 1024 * 1024;
const MAX_DEPTH_UNDER_PKG: usize = 4;

#[derive(Debug, Clone, PartialEq)]
struct IndexedCandidate {
    path: PathBuf,
    priority: u8,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct AppPathEntry {
    pub candidates: Vec<PathBuf>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default, PartialEq)]
pub struct AppPathIndex {
    pub built_ms: u64,
    pub last_rebuild_ms: u64,
    pub last_build_duration_ms: u64,
    pub entries: HashMap<String, AppPathEntry>,
    pub stale: bool,
}

pub struct AppIndexer {
    pub index: RwLock<AppPathIndex>,
}

impl Default for AppIndexer {
    fn default() -> Self {
        Self::new()
    }
}

impl AppIndexer {
    pub fn new() -> Self {
        Self {
            index: RwLock::new(AppPathIndex::default()),
        }
    }

    pub fn rebuild(&self, abi: RuntimeAbi) {
        self.rebuild_from_root(Path::new(DATA_APP_DIR), abi);
    }

    pub fn rebuild_from_root(&self, data_app: &Path, abi: RuntimeAbi) {
        let start = Instant::now();
        let new_entries = build_index_entries(data_app, abi);
        let built_ms = unix_time_ms();

        let mut index = self.index.write().unwrap();
        index.entries = new_entries;
        index.built_ms = built_ms;
        index.last_rebuild_ms = built_ms;
        index.last_build_duration_ms = start.elapsed().as_millis() as u64;
        index.stale = false;
    }

    pub fn get_candidates(&self, package: &str) -> Vec<PathBuf> {
        self.index
            .read()
            .unwrap()
            .entries
            .get(package)
            .map(|entry| entry.candidates.clone())
            .unwrap_or_default()
    }

    pub fn snapshot(&self) -> AppPathIndex {
        self.index.read().unwrap().clone()
    }
}

fn build_index_entries(data_app: &Path, abi: RuntimeAbi) -> HashMap<String, AppPathEntry> {
    let mut entries = HashMap::new();
    let mut dirs_visited = 0;

    if let Ok(level1_entries) = fs::read_dir(data_app) {
        for level1 in level1_entries.flatten() {
            if dirs_visited >= MAX_DIRS_VISITED {
                break;
            }

            let level1_path = level1.path();
            if !level1_path.is_dir() {
                continue;
            }
            dirs_visited += 1;

            if let Ok(package_entries) = fs::read_dir(&level1_path) {
                for package_entry in package_entries.flatten() {
                    if dirs_visited >= MAX_DIRS_VISITED {
                        break;
                    }

                    let package_dir = package_entry.path();
                    if !package_dir.is_dir() {
                        continue;
                    }
                    dirs_visited += 1;

                    let Some(package_name) = package_name_from_dir(&package_dir) else {
                        continue;
                    };

                    let ordered_candidates =
                        collect_package_candidates(&package_dir, &mut dirs_visited, abi)
                            .into_iter()
                            .map(|candidate| candidate.path)
                            .collect::<Vec<_>>();

                    if !ordered_candidates.is_empty() {
                        entries.insert(
                            package_name,
                            AppPathEntry {
                                candidates: ordered_candidates,
                            },
                        );
                    }
                }
            }
        }
    }

    entries
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
    dirs_visited: &mut usize,
    abi: RuntimeAbi,
) -> Vec<IndexedCandidate> {
    let mut candidates = Vec::new();
    let mut total_bytes = 0;
    collect_files_recursive(
        package_dir,
        &mut candidates,
        &mut total_bytes,
        dirs_visited,
        0,
        abi,
    );
    candidates.sort_by_key(|candidate| candidate.priority);
    candidates
}

fn collect_files_recursive(
    dir: &Path,
    candidates: &mut Vec<IndexedCandidate>,
    total_bytes: &mut u64,
    dirs_visited: &mut usize,
    depth: usize,
    abi: RuntimeAbi,
) {
    if *dirs_visited >= MAX_DIRS_VISITED
        || candidates.len() >= MAX_FILES_PER_PACKAGE
        || depth > MAX_DEPTH_UNDER_PKG
    {
        return;
    }

    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
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
                *dirs_visited += 1;
                collect_files_recursive(
                    &path,
                    candidates,
                    total_bytes,
                    dirs_visited,
                    depth + 1,
                    abi,
                );
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn indexer_builds_non_empty_index_from_fake_data_app() {
        let temp =
            std::env::temp_dir().join(format!("coreshift_test_indexer_{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp);

        let pkg_root = temp.join("~~randomized");
        let pkg_dir = pkg_root.join("com.termux-hashvalue");
        fs::create_dir_all(pkg_dir.join("lib/arm64")).unwrap();
        fs::create_dir_all(pkg_dir.join("oat/arm64")).unwrap();
        fs::write(pkg_dir.join("base.apk"), b"apk").unwrap();
        fs::write(pkg_dir.join("lib/arm64/libtermux.so"), b"lib").unwrap();
        fs::write(pkg_dir.join("oat/arm64/base.vdex"), b"vdex").unwrap();

        let indexer = AppIndexer::new();
        indexer.rebuild_from_root(&temp, RuntimeAbi::Arm64);

        let snapshot = indexer.snapshot();
        let entry = snapshot.entries.get("com.termux").unwrap();
        assert_eq!(entry.candidates.len(), 2);
        assert!(entry.candidates[0].ends_with("base.vdex"));
        assert!(entry.candidates[1].ends_with("libtermux.so"));
        assert!(snapshot.built_ms > 0);
        assert_eq!(snapshot.last_rebuild_ms, snapshot.built_ms);
        assert!(!snapshot.stale);

        let _ = fs::remove_dir_all(&temp);
    }
}
