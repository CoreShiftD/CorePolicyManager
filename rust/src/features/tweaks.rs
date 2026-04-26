//! Applies system-level performance, balance, and power-saving tweaks.
//
// This module ports and modernizes the logic from a legacy script, providing a safe
// and robust way to apply kernel, CPU, and I/O tuning profiles. It uses a JSON-based
// cache for discovered system properties to avoid unnecessary filesystem scans on
// every run.

use crate::runtime::status::write_json_file_if_changed;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;
use std::fs;
use std::io;
use std::path::Path;
use std::str::FromStr;
use std::time::{SystemTime, UNIX_EPOCH};

// --- Public API ---

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TweakProfile {
    Balance,
    Performance,
    Power,
}

impl FromStr for TweakProfile {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "balance" => Ok(Self::Balance),
            "performance" => Ok(Self::Performance),
            "power" => Ok(Self::Power),
            _ => Err("invalid profile name"),
        }
    }
}

impl fmt::Display for TweakProfile {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::Balance => "balance",
                Self::Performance => "performance",
                Self::Power => "power",
            }
        )
    }
}

/// A summary of the actions taken and the result of applying a tweak profile.
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct TweakApplySummary {
    pub profile_name: String,
    pub attempted_writes: u32,
    pub successful_writes: u32,
    pub skipped_writes: u32,
    pub failed_writes: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub first_error: Option<String>,
}

impl TweakApplySummary {
    fn record_write(&mut self, result: &WriteResult) {
        self.attempted_writes += 1;
        match result {
            WriteResult::Success => self.successful_writes += 1,
            WriteResult::Skipped => self.skipped_writes += 1,
            WriteResult::Failed(e) => {
                self.failed_writes += 1;
                if self.first_error.is_none() {
                    self.first_error = Some(e.clone());
                }
            }
            WriteResult::PathMissing => {}
        }
    }
}

// --- Status Reporting ---
pub const TWEAK_STATUS_FILE: &str = "/data/local/tmp/coreshift/tweak_status.json";

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct TweakStatus {
    pub schema_version: u32,
    pub last_profile: String,
    pub last_applied_ms: u64,
    pub summary: TweakApplySummary,
}

impl TweakStatus {
    pub fn write_if_changed(
        &self,
        last_written: &mut Option<TweakStatus>,
    ) -> Result<bool, std::io::Error> {
        write_json_file_if_changed(TWEAK_STATUS_FILE, self, last_written)
    }
}

// --- Caching ---
const TWEAK_CACHE_FILE: &str = "/data/local/tmp/coreshift/tweak_cache.json";
const LITTLE_CORE_CAP_THRESHOLD: u32 = 512;

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct TweakCache {
    pub schema_version: u32,
    #[serde(default)]
    pub updated_ms: u64,
    pub entries: HashMap<String, String>,
}

impl TweakCache {
    pub fn load() -> Self {
        fs::read_to_string(TWEAK_CACHE_FILE)
            .ok()
            .and_then(|content| serde_json::from_str(&content).ok())
            .filter(|cache: &Self| cache.schema_version == 1)
            .unwrap_or_default()
    }

    pub fn save(&mut self) -> io::Result<()> {
        self.schema_version = 1;
        self.updated_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;

        if self.entries.len() > 256 {
            return Err(io::Error::other("Cache size exceeds limit"));
        }

        let path = Path::new(TWEAK_CACHE_FILE);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let temp_path = format!("{}.tmp", TWEAK_CACHE_FILE);
        fs::write(&temp_path, serde_json::to_string_pretty(self)?)?;
        fs::rename(&temp_path, path)
    }

    pub fn clear() -> io::Result<()> {
        fs::remove_file(TWEAK_CACHE_FILE)
    }
}

// --- Profile Definitions ---
#[derive(Debug, Clone)]
struct ProfileConfig {
    sched_migration_cost: u32,
    sched_min_granularity: u32,
    sched_wakeup_granularity: u32,
    schedutil_up: u32,
    schedutil_down: u32,
    read_ahead_kb: u32,
    nr_requests: u32,
    swappiness: u32,
    vfs_cache_pressure: u32,
    dirty_background_ratio: u32,
    dirty_ratio: u32,
    dirty_expire: u32,
    thp_enabled: &'static str,
    thp_defrag: &'static str,
    uclamp_min_top_app: u32,
    uclamp_max_background: u32,
    watermark_scale_factor: u32,
    page_cluster: u32,
    scan_sleep_millisecs: u32,
}

static CONFIG_BALANCE: ProfileConfig = ProfileConfig {
    sched_migration_cost: 5000000,
    sched_min_granularity: 1500000,
    sched_wakeup_granularity: 2000000,
    schedutil_up: 500,
    schedutil_down: 20000,
    read_ahead_kb: 128,
    nr_requests: 64,
    swappiness: 10,
    vfs_cache_pressure: 50,
    dirty_background_ratio: 5,
    dirty_ratio: 15,
    dirty_expire: 2000,
    thp_enabled: "madvise",
    thp_defrag: "defer",
    uclamp_min_top_app: 20,
    uclamp_max_background: 10,
    watermark_scale_factor: 10,
    page_cluster: 0,
    scan_sleep_millisecs: 10000,
};

static CONFIG_PERFORMANCE: ProfileConfig = ProfileConfig {
    sched_migration_cost: 3000000,
    sched_min_granularity: 1000000,
    sched_wakeup_granularity: 1500000,
    schedutil_up: 250,
    schedutil_down: 40000,
    read_ahead_kb: 256,
    nr_requests: 128,
    swappiness: 5,
    vfs_cache_pressure: 100,
    dirty_background_ratio: 10,
    dirty_ratio: 30,
    dirty_expire: 1000,
    thp_enabled: "always",
    thp_defrag: "always",
    uclamp_min_top_app: 60,
    uclamp_max_background: 5,
    watermark_scale_factor: 15,
    page_cluster: 0,
    scan_sleep_millisecs: 5000,
};

static CONFIG_POWER: ProfileConfig = ProfileConfig {
    sched_migration_cost: 10000000,
    sched_min_granularity: 3000000,
    sched_wakeup_granularity: 4000000,
    schedutil_up: 2000,
    schedutil_down: 10000,
    read_ahead_kb: 64,
    nr_requests: 32,
    swappiness: 60,
    vfs_cache_pressure: 10,
    dirty_background_ratio: 2,
    dirty_ratio: 5,
    dirty_expire: 5000,
    thp_enabled: "never",
    thp_defrag: "never",
    uclamp_min_top_app: 10,
    uclamp_max_background: 15,
    watermark_scale_factor: 5,
    page_cluster: 1,
    scan_sleep_millisecs: 20000,
};

// --- Discovery & Application Logic ---

enum WriteResult {
    Success,
    Skipped,
    PathMissing,
    Failed(String),
}

/// Safely writes a value to a system path.
fn write_value(path_str: &str, value: &str) -> WriteResult {
    let path = Path::new(path_str);
    if !path.exists() {
        return WriteResult::PathMissing;
    }
    if let Ok(current) = fs::read_to_string(path) && current.trim() == value {
        return WriteResult::Skipped;
    }
    match fs::write(path, value) {
        Ok(_) => WriteResult::Success,
        Err(e) => WriteResult::Failed(format!("{} -> '{}': {}", path_str, value, e)),
    }
}

fn for_each_in_dir<F: FnMut(&str)>(dir: &str, mut f: F) {
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            if let Some(name) = entry.file_name().to_str() {
                f(name);
            }
        }
    }
}

fn pick_best<'a>(available: &str, prio: &[&'a str]) -> Option<&'a str> {
    for p in prio {
        if available.contains(p) {
            return Some(*p);
        }
    }
    None
}

/// Discovers CPU topology (little and all cores) and stores it in the cache.
fn discover_cpu_topology(cache: &mut TweakCache) {
    if cache.entries.contains_key("cpuset_little") && cache.entries.contains_key("cpuset_all") {
        return;
    }

    let mut little_cores = Vec::new();
    let mut all_cores = Vec::new();

    for_each_in_dir("/sys/devices/system/cpu", |name| {
        if name.starts_with("cpu")
            && name[3..].chars().all(char::is_numeric)
            && let Ok(id) = name[3..].parse::<u32>()
        {
            all_cores.push(id);
            let cap_path = format!("/sys/devices/system/cpu/{}/cpu_capacity", name);
            if let Ok(cap_str) = fs::read_to_string(&cap_path)
                && let Ok(cap) = cap_str.trim().parse::<u32>()
                && cap > 0
                && cap < LITTLE_CORE_CAP_THRESHOLD
            {
                little_cores.push(id);
            }
        }
    });

    if !little_cores.is_empty() {
        cache
            .entries
            .insert("cpuset_little".to_string(), format_cpu_list(&little_cores));
    }
    if !all_cores.is_empty() {
        cache
            .entries
            .insert("cpuset_all".to_string(), format_cpu_list(&all_cores));
    }
}

fn format_cpu_list(cpus: &[u32]) -> String {
    cpus.iter()
        .map(|c| c.to_string())
        .collect::<Vec<_>>()
        .join(",")
}

fn apply_governors(profile: &TweakProfile, cache: &mut TweakCache, summary: &mut TweakApplySummary) {
    const GOV_PRIO_BALANCE: &[&str] = &["schedutil", "simple_ondemand", "schedhorizon", "sugov_ext"];

    for_each_in_dir("/sys/devices/system/cpu/cpufreq", |name| {
        if !name.starts_with("policy") {
            return;
        }
        let cache_key = format!("{}_governor", name);
        let gov_path = format!("/sys/devices/system/cpu/cpufreq/{}/scaling_governor", name);
        let avail_path = format!(
            "/sys/devices/system/cpu/cpufreq/{}/scaling_available_governors",
            name
        );

        let target_gov = match profile {
            TweakProfile::Balance => {
                if let Some(cached_gov) = cache.entries.get(&cache_key) {
                    cached_gov.clone()
                } else if let Ok(avail) = fs::read_to_string(avail_path) {
                    if let Some(best) = pick_best(&avail, GOV_PRIO_BALANCE) {
                        cache.entries.insert(cache_key, best.to_string());
                        best.to_string()
                    } else {
                        return;
                    }
                } else {
                    return;
                }
            }
            TweakProfile::Performance => "performance".to_string(),
            TweakProfile::Power => "powersave".to_string(),
        };
        summary.record_write(&write_value(&gov_path, &target_gov));
    });

    for_each_in_dir("/sys/class/devfreq", |name| {
        let gov_path = format!("/sys/class/devfreq/{}/governor", name);
        let target_gov = match profile {
            TweakProfile::Performance => "performance",
            TweakProfile::Power => "powersave",
            TweakProfile::Balance => "powersave",
        };
        summary.record_write(&write_value(&gov_path, target_gov));
    });
}

fn apply_kernel_tweaks(cfg: &ProfileConfig, summary: &mut TweakApplySummary) {
    summary.record_write(&write_value(
        "/proc/sys/kernel/sched_migration_cost_ns",
        &cfg.sched_migration_cost.to_string(),
    ));
    summary.record_write(&write_value(
        "/proc/sys/kernel/sched_min_granularity_ns",
        &cfg.sched_min_granularity.to_string(),
    ));
    summary.record_write(&write_value(
        "/proc/sys/kernel/sched_wakeup_granularity_ns",
        &cfg.sched_wakeup_granularity.to_string(),
    ));
    summary.record_write(&write_value(
        "/proc/sys/vm/swappiness",
        &cfg.swappiness.to_string(),
    ));
    summary.record_write(&write_value(
        "/proc/sys/vm/vfs_cache_pressure",
        &cfg.vfs_cache_pressure.to_string(),
    ));
    summary.record_write(&write_value(
        "/proc/sys/vm/dirty_background_ratio",
        &cfg.dirty_background_ratio.to_string(),
    ));
    summary.record_write(&write_value(
        "/proc/sys/vm/dirty_ratio",
        &cfg.dirty_ratio.to_string(),
    ));
    summary.record_write(&write_value(
        "/proc/sys/vm/dirty_expire_centisecs",
        &cfg.dirty_expire.to_string(),
    ));
    summary.record_write(&write_value(
        "/proc/sys/vm/watermark_scale_factor",
        &cfg.watermark_scale_factor.to_string(),
    ));
    summary.record_write(&write_value(
        "/proc/sys/vm/page-cluster",
        &cfg.page_cluster.to_string(),
    ));
    summary.record_write(&write_value(
        "/sys/kernel/mm/transparent_hugepage/enabled",
        cfg.thp_enabled,
    ));
    summary.record_write(&write_value(
        "/sys/kernel/mm/transparent_hugepage/defrag",
        cfg.thp_defrag,
    ));
    summary.record_write(&write_value(
        "/sys/kernel/mm/transparent_hugepage/khugepaged/scan_sleep_millisecs",
        &cfg.scan_sleep_millisecs.to_string(),
    ));
    summary.record_write(&write_value("/proc/sys/kernel/sched_autogroup_enabled", "0"));

    for_each_in_dir("/sys/devices/system/cpu/cpufreq", |name| {
        if !name.starts_with("policy") {
            return;
        }
        let up_path = format!(
            "/sys/devices/system/cpu/cpufreq/{}/schedutil/up_rate_limit_us",
            name
        );
        let down_path = format!(
            "/sys/devices/system/cpu/cpufreq/{}/schedutil/down_rate_limit_us",
            name
        );
        let io_path = format!(
            "/sys/devices/system/cpu/cpufreq/{}/schedutil/iowait_boost_enable",
            name
        );

        summary.record_write(&write_value(&up_path, &cfg.schedutil_up.to_string()));
        summary.record_write(&write_value(&down_path, &cfg.schedutil_down.to_string()));

        let io_boost = if cfg.schedutil_up != 2000 { "1" } else { "0" };
        summary.record_write(&write_value(&io_path, io_boost));
    });

    summary.record_write(&write_value("/sys/block/zram0/max_comp_streams", "2"));
}

fn apply_block_dev_tweaks(
    cfg: &ProfileConfig,
    cache: &mut TweakCache,
    summary: &mut TweakApplySummary,
) {
    const BLK_PRIO_BALANCE: &[&str] = &["mq-deadline", "kyber", "bfq", "deadline", "none", "noop"];

    for_each_in_dir("/sys/block", |name| {
        if name.starts_with("loop") || name.starts_with("ram") || name.starts_with("zram") {
            return;
        }

        let cache_key = format!("block_scheduler_{}", name);
        if let Some(cached_sched) = cache.entries.get(&cache_key) {
            let sched_path = format!("/sys/block/{}/queue/scheduler", name);
            summary.record_write(&write_value(&sched_path, cached_sched));
        } else if let Ok(avail) = fs::read_to_string(format!("/sys/block/{}/queue/scheduler", name))
            && let Some(best) = pick_best(&avail, BLK_PRIO_BALANCE)
        {
            cache.entries.insert(cache_key, best.to_string());
            let sched_path = format!("/sys/block/{}/queue/scheduler", name);
            summary.record_write(&write_value(&sched_path, best));
        }

        let ra_path = format!("/sys/block/{}/queue/read_ahead_kb", name);
        summary.record_write(&write_value(&ra_path, &cfg.read_ahead_kb.to_string()));
        let nr_path = format!("/sys/block/{}/queue/nr_requests", name);
        summary.record_write(&write_value(&nr_path, &cfg.nr_requests.to_string()));
    });
}

fn apply_cpuset_tweaks(cache: &TweakCache, summary: &mut TweakApplySummary) {
    if let Some(little) = cache.entries.get("cpuset_little") {
        summary.record_write(&write_value("/dev/cpuset/background/cpus", little));
        summary.record_write(&write_value("/dev/cpuset/system-background/cpus", little));
        summary.record_write(&write_value("/dev/cpuset/background/sched_load_balance", "0"));
        summary.record_write(&write_value(
            "/dev/cpuset/system-background/sched_load_balance",
            "0",
        ));
    }
    if let Some(all) = cache.entries.get("cpuset_all") {
        summary.record_write(&write_value("/dev/cpuset/foreground/cpus", all));
        summary.record_write(&write_value("/dev/cpuset/top-app/cpus", all));
        summary.record_write(&write_value("/dev/cpuset/foreground/sched_load_balance", "1"));
        summary.record_write(&write_value("/dev/cpuset/top-app/sched_load_balance", "1"));
        summary.record_write(&write_value(
            "/dev/cpuset/top-app/sched_relax_domain_level",
            "1",
        ));
    }
}

fn apply_uclamp_tweaks(cfg: &ProfileConfig, summary: &mut TweakApplySummary) {
    summary.record_write(&write_value("/dev/cpuctl/background/cpu.uclamp.min", "0"));
    summary.record_write(&write_value(
        "/dev/cpuctl/background/cpu.uclamp.max",
        &cfg.uclamp_max_background.to_string(),
    ));
    summary.record_write(&write_value(
        "/dev/cpuctl/system-background/cpu.uclamp.min",
        "0",
    ));
    summary.record_write(&write_value("/dev/cpuctl/system-background/cpu.uclamp.max", "15"));
    summary.record_write(&write_value("/dev/cpuctl/foreground/cpu.uclamp.min", "0"));
    summary.record_write(&write_value("/dev/cpuctl/foreground/cpu.uclamp.max", "25"));
    summary.record_write(&write_value(
        "/dev/cpuctl/top-app/cpu.uclamp.min",
        &cfg.uclamp_min_top_app.to_string(),
    ));
    summary.record_write(&write_value("/dev/cpuctl/top-app/cpu.uclamp.max", "100"));
}

/// The main entry point for applying a profile.
pub fn apply_tweak_profile(profile: TweakProfile) -> TweakApplySummary {
    let mut summary = TweakApplySummary {
        profile_name: profile.to_string(),
        ..Default::default()
    };
    let mut cache = TweakCache::load();
    let needs_save = cache.entries.is_empty();

    let config = match profile {
        TweakProfile::Balance => &CONFIG_BALANCE,
        TweakProfile::Performance => &CONFIG_PERFORMANCE,
        TweakProfile::Power => &CONFIG_POWER,
    };

    discover_cpu_topology(&mut cache);

    apply_governors(&profile, &mut cache, &mut summary);
    apply_kernel_tweaks(config, &mut summary);
    apply_block_dev_tweaks(config, &mut cache, &mut summary);
    apply_cpuset_tweaks(&cache, &mut summary);
    apply_uclamp_tweaks(config, &mut summary);

    if needs_save && let Err(e) = cache.save() {
        summary.failed_writes += 1;
        if summary.first_error.is_none() {
            summary.first_error = Some(format!("Failed to save cache: {}", e));
        }
    }

    let status = TweakStatus {
        schema_version: 1,
        last_profile: profile.to_string(),
        last_applied_ms: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64,
        summary: summary.clone(),
    };

    let mut last_written_status = None;
    if let Err(e) = status.write_if_changed(&mut last_written_status) && summary.first_error.is_none() {
        summary.first_error = Some(format!("Failed to write tweak status: {}", e));
    }

    summary
}
