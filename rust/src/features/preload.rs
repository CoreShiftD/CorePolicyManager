use crate::runtime::logging;
use crate::runtime::status::DaemonStatus;
use coreshift_lowlevel::sys::readahead;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

const MAX_DIRS_VISITED: usize = 256;
const MAX_FILES_PER_PACKAGE: usize = 64;
const MAX_BYTES_PER_PACKAGE: u64 = 256 * 1024 * 1024; // 256 MB
const MAX_DEPTH_UNDER_PKG: usize = 4;
const PRELOAD_COOLDOWN: Duration = Duration::from_secs(300); // 5 minutes

struct PreloadCandidate {
    path: PathBuf,
    size: u64,
    priority: u8,
}

#[derive(Default)]
struct PreloadPlan {
    candidates: Vec<PreloadCandidate>,
    total_bytes: u64,
}

#[derive(Default)]
pub struct PreloadFeature {
    cooldowns: HashMap<String, Instant>,
}

impl PreloadFeature {
    pub fn new() -> Self {
        Self {
            cooldowns: HashMap::new(),
        }
    }

    pub fn on_foreground_package(&mut self, pkg: &str, status: &mut DaemonStatus) {
        if let Some(last_time) = self.cooldowns.get(pkg)
            && last_time.elapsed() < PRELOAD_COOLDOWN
        {
            return;
        }

        let start_total = Instant::now();
        let start_discovery = Instant::now();
        let mut plan = discover_app_paths(pkg, Path::new("/data/app"));
        let discovery_ms = start_discovery.elapsed().as_millis() as u64;

        if plan.candidates.is_empty() {
            let total_ms = start_total.elapsed().as_millis() as u64;
            logging::info(&format!(
                "preload package={} files_done=0 files_failed=0 bytes=0 discovery_ms={} readahead_ms=0 total_ms={} result=no_candidates",
                pkg, discovery_ms, total_ms
            ));
            status.last_preload_package = Some(pkg.to_string());
            status.last_preload_file_count = 0;
            status.last_preload_files_failed = 0;
            status.last_preload_bytes = 0;
            status.last_preload_discovery_ms = discovery_ms;
            status.last_preload_readahead_ms = 0;
            status.last_preload_total_ms = total_ms;
            status.last_preload_result = Some("no_candidates".to_string());
            self.cooldowns.insert(pkg.to_string(), Instant::now());
            return;
        }

        plan.candidates.sort_by_key(|c| c.priority);

        let start_readahead = Instant::now();
        let mut bytes_done = 0;
        let mut files_done = 0;
        let mut files_failed = 0;

        for candidate in &plan.candidates {
            match fs::File::open(&candidate.path) {
                Ok(file) => {
                    if readahead(file, 0, candidate.size as usize).is_ok() {
                        bytes_done += candidate.size;
                        files_done += 1;
                    } else {
                        files_failed += 1;
                    }
                }
                Err(_) => files_failed += 1,
            }
        }

        let readahead_ms = start_readahead.elapsed().as_millis() as u64;
        let total_ms = start_total.elapsed().as_millis() as u64;
        let result = if files_done == 0 {
            "failed"
        } else if files_failed > 0 {
            "partial"
        } else {
            "ok"
        };

        logging::info(&format!(
            "preload package={} files_done={} files_failed={} bytes={} discovery_ms={} readahead_ms={} total_ms={} result={}",
            pkg, files_done, files_failed, bytes_done, discovery_ms, readahead_ms, total_ms, result
        ));

        status.last_preload_package = Some(pkg.to_string());
        status.last_preload_file_count = files_done;
        status.last_preload_files_failed = files_failed;
        status.last_preload_bytes = bytes_done;
        status.last_preload_discovery_ms = discovery_ms;
        status.last_preload_readahead_ms = readahead_ms;
        status.last_preload_total_ms = total_ms;
        status.last_preload_result = Some(result.to_string());
        self.cooldowns.insert(pkg.to_string(), Instant::now());
    }
}

fn discover_app_paths(package: &str, data_app: &Path) -> PreloadPlan {
    let mut plan = PreloadPlan::default();
    let prefix = format!("{}-", package);
    let mut dirs_visited = 0;

    // 1. Scan /data/app one level deep (e.g. ~~random)
    if let Ok(entries) = fs::read_dir(data_app) {
        for entry in entries.flatten() {
            if dirs_visited >= MAX_DIRS_VISITED {
                break;
            }
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }
            dirs_visited += 1;

            // 2. Scan direct children for <package>-...
            if let Ok(children) = fs::read_dir(&path) {
                for child in children.flatten() {
                    let child_path = child.path();
                    if !child_path.is_dir() {
                        continue;
                    }

                    if let Some(name) = child_path.file_name().and_then(|n| n.to_str())
                        && name.starts_with(&prefix)
                    {
                        // Found package directory
                        collect_files_recursive(&child_path, &mut plan, &mut dirs_visited, 0);
                        return plan;
                    }
                }
            }
        }
    }

    plan
}

fn collect_files_recursive(
    dir: &Path,
    plan: &mut PreloadPlan,
    dirs_visited: &mut usize,
    depth: usize,
) {
    if *dirs_visited >= MAX_DIRS_VISITED
        || plan.candidates.len() >= MAX_FILES_PER_PACKAGE
        || depth > MAX_DEPTH_UNDER_PKG
    {
        return;
    }

    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            let name_os = path.file_name().unwrap_or_default();
            let name = name_os.to_string_lossy().to_lowercase();

            // Safety check for dangerous/useless dirs
            if name.contains("cache")
                || name.contains("tmp")
                || name.contains("log")
                || name.contains("metadata")
            {
                continue;
            }

            if path.is_dir() {
                *dirs_visited += 1;
                collect_files_recursive(&path, plan, dirs_visited, depth + 1);
            } else {
                if plan.candidates.len() >= MAX_FILES_PER_PACKAGE {
                    break;
                }

                // Explicitly skip unwanted file types
                if name.ends_with(".apk") || name.ends_with(".dm") || name.contains("digests") {
                    continue;
                }

                if let Some(priority) = get_preload_priority(&name, &path)
                    && let Ok(metadata) = fs::metadata(&path)
                {
                    let size = metadata.len();
                    if plan.total_bytes + size <= MAX_BYTES_PER_PACKAGE {
                        plan.total_bytes += size;
                        plan.candidates.push(PreloadCandidate {
                            path,
                            size,
                            priority,
                        });
                    }
                }
            }
        }
    }
}

fn get_preload_priority(name: &str, path: &Path) -> Option<u8> {
    let path_str = path.to_string_lossy();

    // Tier 1: oat/arm64 artifacts
    if path_str.contains("/oat/arm64/")
        && (name.ends_with(".vdex") || name.ends_with(".odex") || name.ends_with(".art"))
    {
        return Some(1);
    }

    // Tier 2: lib/arm64 native libraries
    if path_str.contains("/lib/arm64/") && name.ends_with(".so") {
        return Some(2);
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_randomized_path_discovery() {
        let temp = std::env::temp_dir().join("coreshift_test_data_app_v2");
        let _ = fs::remove_dir_all(&temp);
        fs::create_dir_all(&temp).unwrap();

        let pkg = "com.termux";
        let level1 = temp.join("~~v6l4tFdlGk9oRyCFJgZt5Q==");
        let pkg_dir = level1.join("com.termux-utL_47ya6kPw1TByUMc6IQ==");
        fs::create_dir_all(&pkg_dir).unwrap();

        let apk = pkg_dir.join("base.apk");
        fs::write(&apk, "fake apk").unwrap();

        let lib_dir = pkg_dir.join("lib/arm64");
        fs::create_dir_all(&lib_dir).unwrap();
        let so = lib_dir.join("libtest.so");
        fs::write(&so, "fake so").unwrap();

        let oat_dir = pkg_dir.join("oat/arm64");
        fs::create_dir_all(&oat_dir).unwrap();
        let vdex = oat_dir.join("base.vdex");
        fs::write(&vdex, "fake vdex").unwrap();

        let plan = discover_app_paths(pkg, &temp);
        // Expecting 2: so and vdex. APK is skipped.
        assert_eq!(plan.candidates.len(), 2);

        let paths: Vec<PathBuf> = plan.candidates.iter().map(|c| c.path.clone()).collect();
        assert!(!paths.contains(&apk)); // APK MUST be skipped
        assert!(paths.contains(&so));
        assert!(paths.contains(&vdex));
        assert!(plan.total_bytes > 0);

        let _ = fs::remove_dir_all(&temp);
    }
}
