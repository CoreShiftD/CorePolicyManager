use crate::runtime::logging;
use crate::runtime::status::{PreloadResult, PreloadStatusFile};
use coreshift_lowlevel::sys::readahead;
use std::collections::HashMap;
use std::fs;
use std::time::{Duration, Instant};

const PRELOAD_COOLDOWN: Duration = Duration::from_secs(300); // 5 minutes

#[derive(Default, Debug, Clone, Copy, PartialEq)]
pub enum RuntimeAbi {
    #[default]
    Arm64,
    Arm32,
}

impl RuntimeAbi {
    pub fn current() -> Self {
        if cfg!(target_arch = "aarch64") {
            RuntimeAbi::Arm64
        } else {
            RuntimeAbi::Arm32
        }
    }
}

pub struct PreloadFeature {
    cooldowns: HashMap<String, Instant>,
}

impl Default for PreloadFeature {
    fn default() -> Self {
        Self::new()
    }
}

impl PreloadFeature {
    pub fn new() -> Self {
        Self {
            cooldowns: HashMap::new(),
        }
    }

    pub fn on_foreground_package(
        &mut self,
        pkg: &str,
        candidates: &[std::path::PathBuf],
        status: &mut PreloadStatusFile,
    ) {
        // Opportunistic cleanup
        let now = Instant::now();
        self.cooldowns
            .retain(|_, time| now.duration_since(*time) < PRELOAD_COOLDOWN * 2);

        if let Some(last_time) = self.cooldowns.get(pkg)
            && last_time.elapsed() < PRELOAD_COOLDOWN
        {
            // ... (keep existing cooldown logic)
            let remaining = PRELOAD_COOLDOWN.saturating_sub(last_time.elapsed());
            let remaining_secs = remaining.as_secs();
            logging::dedup_debug(
                &format!("preload_cooldown:{}", pkg),
                &format!(
                    "Preload: cooldown package={} remaining_secs={}",
                    pkg, remaining_secs
                ),
                Duration::from_secs(30),
            );
            status.last_package = Some(pkg.to_string());
            status.result = Some(PreloadResult::Cooldown);
            return;
        }

        let start_total = Instant::now();
        let discovery_ms = 0;

        if candidates.is_empty() {
            // ... (keep existing no_candidates logic)
            let total_ms = start_total.elapsed().as_millis() as u64;
            status.last_package = Some(pkg.to_string());
            status.file_count = 0;
            status.files_failed = 0;
            status.bytes = 0;
            status.discovery_ms = discovery_ms;
            status.readahead_ms = 0;
            status.total_ms = total_ms;
            status.result = Some(PreloadResult::NoCandidates);
            self.cooldowns.insert(pkg.to_string(), Instant::now());
            return;
        }

        // ... (rest of logic: readahead, result status updates)
        let start_readahead = Instant::now();
        let mut bytes_done = 0;
        let mut files_done = 0;
        let mut files_failed = 0;

        for candidate in candidates {
            match fs::File::open(candidate) {
                Ok(file) => {
                    let size = file.metadata().map(|metadata| metadata.len()).unwrap_or(0);
                    if readahead(file, 0, size as usize).is_ok() {
                        bytes_done += size;
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
            PreloadResult::Failed
        } else if files_failed > 0 {
            PreloadResult::Partial
        } else {
            PreloadResult::Ok
        };

        status.last_package = Some(pkg.to_string());
        status.file_count = files_done;
        status.files_failed = files_failed;
        status.bytes = bytes_done;
        status.discovery_ms = discovery_ms;
        status.readahead_ms = readahead_ms;
        status.total_ms = total_ms;
        status.result = Some(result);
        self.cooldowns.insert(pkg.to_string(), Instant::now());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_cooldown_skip_updates_status() {
        let mut preload = PreloadFeature::new();
        let mut status = PreloadStatusFile::default();
        preload
            .cooldowns
            .insert("com.example.app".to_string(), Instant::now());

        preload.on_foreground_package("com.example.app", &[], &mut status);

        assert_eq!(status.last_package.as_deref(), Some("com.example.app"));
        assert_eq!(status.result, Some(PreloadResult::Cooldown));
    }

    #[test]
    fn test_cached_candidates_no_scan_no_candidates() {
        let mut preload = PreloadFeature::new();
        let mut status = PreloadStatusFile::default();

        preload.on_foreground_package("com.example.app", &[], &mut status);

        assert_eq!(status.result, Some(PreloadResult::NoCandidates));
        assert_eq!(status.discovery_ms, 0);
    }

    #[test]
    fn test_cached_candidates_are_used() {
        let mut preload = PreloadFeature::new();
        let mut status = PreloadStatusFile::default();
        let temp = std::env::temp_dir().join(format!(
            "coreshift_preload_cached_candidates_{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&temp);
        fs::create_dir_all(&temp).unwrap();

        let cached = temp.join("libexample.so");
        fs::write(&cached, b"cached-bytes").unwrap();

        preload.on_foreground_package("com.example.app", &[PathBuf::from(&cached)], &mut status);

        assert_eq!(status.result, Some(PreloadResult::Ok));
        assert_eq!(status.file_count, 1);
        assert_eq!(status.files_failed, 0);
        assert_eq!(status.discovery_ms, 0);

        let _ = fs::remove_dir_all(&temp);
    }
}
