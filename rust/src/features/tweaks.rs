use crate::runtime::status::write_json_file_if_changed;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::time::{SystemTime, UNIX_EPOCH};

const TWEAK_CACHE_FILE: &str = "/data/local/tmp/coreshift/tweak_cache.json";
pub const TWEAK_STATUS_FILE: &str = "/data/local/tmp/coreshift/tweak_status.json";
const LITTLE_CORE_CAP_THRESHOLD: u32 = 512;
const ALLOWED_ROOTS: [&str; 4] = ["/proc/sys/", "/sys/", "/dev/cpuset/", "/dev/cpuctl/"];

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
        let value = match self {
            Self::Balance => "balance",
            Self::Performance => "performance",
            Self::Power => "power",
        };
        write!(f, "{}", value)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct TweakApplySummary {
    #[serde(default, alias = "profile_name")]
    pub source: String,
    #[serde(default)]
    pub requested_commands: usize,
    #[serde(default)]
    pub executed_commands: usize,
    #[serde(default)]
    pub attempted_writes: u32,
    #[serde(default)]
    pub successful_writes: u32,
    #[serde(default)]
    pub skipped_writes: u32,
    #[serde(default)]
    pub failed_writes: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub first_error: Option<String>,
}

impl TweakApplySummary {
    fn record_write_result(&mut self, result: WriteResult) {
        self.attempted_writes += 1;
        match result {
            WriteResult::Success => self.successful_writes += 1,
            WriteResult::Skipped | WriteResult::PathMissing | WriteResult::GovernorUnsupported => {
                self.skipped_writes += 1
            }
            WriteResult::Failed(error) => {
                self.failed_writes += 1;
                if self.first_error.is_none() {
                    self.first_error = Some(error);
                }
            }
        }
    }

    fn record_command_error(&mut self, error: String) {
        self.failed_writes += 1;
        if self.first_error.is_none() {
            self.first_error = Some(error);
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct TweakStatus {
    #[serde(default = "schema_version_1")]
    pub schema_version: u32,
    #[serde(default, alias = "last_profile")]
    pub last_source: String,
    #[serde(default)]
    pub last_commands: usize,
    #[serde(default)]
    pub last_successful: u32,
    #[serde(default)]
    pub last_failed: u32,
    #[serde(default)]
    pub last_applied_ms: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub summary: Option<TweakApplySummary>,
}

impl Default for TweakStatus {
    fn default() -> Self {
        Self {
            schema_version: schema_version_1(),
            last_source: String::new(),
            last_commands: 0,
            last_successful: 0,
            last_failed: 0,
            last_applied_ms: 0,
            summary: None,
        }
    }
}

impl TweakStatus {
    fn from_summary(summary: &TweakApplySummary) -> Self {
        Self {
            schema_version: schema_version_1(),
            last_source: summary.source.clone(),
            last_commands: summary.executed_commands,
            last_successful: summary.successful_writes,
            last_failed: summary.failed_writes,
            last_applied_ms: now_ms(),
            summary: Some(summary.clone()),
        }
    }

    pub fn write_if_changed(
        &self,
        last_written: &mut Option<TweakStatus>,
    ) -> Result<bool, std::io::Error> {
        let path = tweak_status_file_path();
        write_json_file_if_changed(path.to_string_lossy().as_ref(), self, last_written)
    }
}

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct TweakCache {
    pub schema_version: u32,
    #[serde(default)]
    pub updated_ms: u64,
    pub entries: HashMap<String, String>,
    #[serde(skip)]
    pub dirty: bool,
}

impl TweakCache {
    pub fn load() -> Self {
        Self::load_from_path(&tweak_cache_file_path())
    }

    fn load_from_path(path: &Path) -> Self {
        fs::read_to_string(path)
            .ok()
            .and_then(|content| serde_json::from_str(&content).ok())
            .filter(|cache: &Self| cache.schema_version == 1)
            .unwrap_or_default()
    }

    pub fn save(&mut self) -> io::Result<()> {
        self.save_to_path(&tweak_cache_file_path())
    }

    fn save_to_path(&mut self, path: &Path) -> io::Result<()> {
        self.schema_version = 1;
        self.updated_ms = now_ms();

        if self.entries.len() > 256 {
            return Err(io::Error::other("Cache size exceeds limit"));
        }

        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let temp_path = path.with_extension("json.tmp");
        fs::write(&temp_path, serde_json::to_string_pretty(self)?)?;
        fs::rename(&temp_path, path)?;
        self.dirty = false;
        Ok(())
    }

    pub fn clear() -> io::Result<()> {
        Self::clear_path(&tweak_cache_file_path())
    }

    fn clear_path(path: &Path) -> io::Result<()> {
        match fs::remove_file(path) {
            Ok(_) => Ok(()),
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
            Err(error) => Err(error),
        }
    }

    fn insert(&mut self, key: impl Into<String>, value: impl Into<String>) {
        let key = key.into();
        let value = value.into();
        if let Some(previous) = self.entries.insert(key, value.clone()) {
            if previous != value {
                self.dirty = true;
            }
        } else {
            self.dirty = true;
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TweakCommand {
    Write {
        path: String,
        value: String,
    },
    GovernorCpu {
        target: String,
    },
    GovernorDevfreq {
        target: String,
    },
    Cpuset {
        group: String,
        target: CpusetTarget,
    },
    Uclamp {
        group: String,
        bound: UclampBound,
        value: u32,
    },
    Preset(TweakProfile),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CpusetTarget {
    All,
    Little,
}

impl fmt::Display for CpusetTarget {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::All => write!(f, "all"),
            Self::Little => write!(f, "little"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UclampBound {
    Min,
    Max,
}

impl fmt::Display for UclampBound {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Min => write!(f, "min"),
            Self::Max => write!(f, "max"),
        }
    }
}

enum WriteResult {
    Success,
    Skipped,
    PathMissing,
    GovernorUnsupported,
    Failed(String),
}

pub fn run_tweak_command_line(
    source: &str,
    command_line: &str,
) -> Result<TweakApplySummary, String> {
    expand_command_line(command_line)?;
    Ok(apply_tweak_commands(source, &[command_line.to_string()]))
}

pub fn apply_tweak_preset(profile: TweakProfile) -> TweakApplySummary {
    apply_tweak_commands(
        &format!("preset:{}", profile),
        &preset_command_lines(profile),
    )
}

pub fn apply_tweak_commands(source: &str, commands: &[String]) -> TweakApplySummary {
    let mut cache = TweakCache::load();
    let mut summary = TweakApplySummary {
        source: source.to_string(),
        requested_commands: commands.len(),
        ..Default::default()
    };

    discover_cpu_topology(&mut cache);

    for command_line in commands {
        match expand_command_line(command_line) {
            Ok(parsed) => {
                summary.executed_commands += parsed.len();
                for command in parsed {
                    execute_command(&command, &mut cache, &mut summary);
                }
            }
            Err(error) => summary.record_command_error(error),
        }
    }

    if cache.dirty
        && let Err(error) = cache.save()
    {
        summary.record_command_error(format!("Failed to save tweak cache: {}", error));
    }

    let status = TweakStatus::from_summary(&summary);
    let mut last_written = None;
    if let Err(error) = status.write_if_changed(&mut last_written) {
        summary.record_command_error(format!("Failed to write tweak status: {}", error));
    }

    summary
}

pub fn command_fingerprint(commands: &[String]) -> Result<String, String> {
    let normalized = normalize_commands(commands)?;
    serde_json::to_string(&normalized).map_err(|error| error.to_string())
}

pub fn normalize_commands(commands: &[String]) -> Result<Vec<String>, String> {
    let mut normalized = Vec::new();
    for command_line in commands {
        for command in expand_command_line(command_line)? {
            normalized.push(format_command(&command));
        }
    }
    Ok(normalized)
}

pub fn parse_tweak_command_line(command_line: &str) -> Result<TweakCommand, String> {
    parse_command_line(command_line)
}

fn schema_version_1() -> u32 {
    1
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

fn tweak_cache_file_path() -> PathBuf {
    env_path("COREPOLICY_TEST_TWEAK_CACHE_FILE", TWEAK_CACHE_FILE)
}

fn tweak_status_file_path() -> PathBuf {
    env_path("COREPOLICY_TEST_TWEAK_STATUS_FILE", TWEAK_STATUS_FILE)
}

fn proc_root_path() -> PathBuf {
    env_path("COREPOLICY_TEST_PROC_ROOT", "/proc")
}

fn sys_root_path() -> PathBuf {
    env_path("COREPOLICY_TEST_SYS_ROOT", "/sys")
}

fn cpuset_root_path() -> PathBuf {
    env_path("COREPOLICY_TEST_CPUSET_ROOT", "/dev/cpuset")
}

fn cpuctl_root_path() -> PathBuf {
    env_path("COREPOLICY_TEST_CPUCTL_ROOT", "/dev/cpuctl")
}

fn env_path(key: &str, default: &str) -> PathBuf {
    std::env::var_os(key)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(default))
}

fn cpu_dir() -> PathBuf {
    sys_root_path().join("devices/system/cpu")
}

fn cpufreq_dir() -> PathBuf {
    cpu_dir().join("cpufreq")
}

fn devfreq_dir() -> PathBuf {
    sys_root_path().join("class/devfreq")
}

fn is_allowed_write_path(path: &str) -> bool {
    ALLOWED_ROOTS.iter().any(|root| path.starts_with(root))
}

fn map_runtime_path(path: &str) -> Option<PathBuf> {
    if !is_allowed_write_path(path) {
        return None;
    }

    path.strip_prefix("/proc/sys/")
        .map(|suffix| proc_root_path().join("sys").join(suffix))
        .or_else(|| {
            path.strip_prefix("/sys/")
                .map(|suffix| sys_root_path().join(suffix))
        })
        .or_else(|| {
            path.strip_prefix("/dev/cpuset/")
                .map(|suffix| cpuset_root_path().join(suffix))
        })
        .or_else(|| {
            path.strip_prefix("/dev/cpuctl/")
                .map(|suffix| cpuctl_root_path().join(suffix))
        })
}

fn parse_command_line(command_line: &str) -> Result<TweakCommand, String> {
    let trimmed = command_line.trim();
    if trimmed.is_empty() {
        return Err("empty tweak command".to_string());
    }

    let parts: Vec<&str> = trimmed.split_whitespace().collect();
    match parts.as_slice() {
        ["preset", profile] => Ok(TweakCommand::Preset(parse_profile(profile)?)),
        ["tweak", "write", path, value @ ..] if !value.is_empty() => {
            let value = value.join(" ");
            if !is_allowed_write_path(path) {
                return Err(format!("path '{}' is not allowed", path));
            }
            Ok(TweakCommand::Write {
                path: (*path).to_string(),
                value,
            })
        }
        ["tweak", "governor", "cpu", target] => Ok(TweakCommand::GovernorCpu {
            target: (*target).to_string(),
        }),
        ["tweak", "governor", "devfreq", target] => Ok(TweakCommand::GovernorDevfreq {
            target: (*target).to_string(),
        }),
        ["tweak", "cpuset", group, "all"] => Ok(TweakCommand::Cpuset {
            group: (*group).to_string(),
            target: CpusetTarget::All,
        }),
        ["tweak", "cpuset", group, "little"] => Ok(TweakCommand::Cpuset {
            group: (*group).to_string(),
            target: CpusetTarget::Little,
        }),
        ["tweak", "uclamp", group, "min", value] => Ok(TweakCommand::Uclamp {
            group: (*group).to_string(),
            bound: UclampBound::Min,
            value: parse_uclamp_value(value)?,
        }),
        ["tweak", "uclamp", group, "max", value] => Ok(TweakCommand::Uclamp {
            group: (*group).to_string(),
            bound: UclampBound::Max,
            value: parse_uclamp_value(value)?,
        }),
        _ => Err(format!("invalid tweak command '{}'", command_line)),
    }
}

fn parse_profile(value: &str) -> Result<TweakProfile, String> {
    value
        .parse::<TweakProfile>()
        .map_err(|error| error.to_string())
}

fn parse_uclamp_value(value: &str) -> Result<u32, String> {
    let parsed = value
        .parse::<u32>()
        .map_err(|_| format!("invalid uclamp value '{}'", value))?;
    if parsed > 100 {
        return Err(format!(
            "uclamp value '{}' must be between 0 and 100",
            value
        ));
    }
    Ok(parsed)
}

fn expand_command_line(command_line: &str) -> Result<Vec<TweakCommand>, String> {
    match parse_command_line(command_line)? {
        TweakCommand::Preset(profile) => preset_command_lines(profile)
            .into_iter()
            .map(|command| parse_command_line(&command))
            .collect(),
        command => Ok(vec![command]),
    }
}

fn format_command(command: &TweakCommand) -> String {
    match command {
        TweakCommand::Write { path, value } => format!("tweak write {} {}", path, value),
        TweakCommand::GovernorCpu { target } => format!("tweak governor cpu {}", target),
        TweakCommand::GovernorDevfreq { target } => {
            format!("tweak governor devfreq {}", target)
        }
        TweakCommand::Cpuset { group, target } => {
            format!("tweak cpuset {} {}", group, target)
        }
        TweakCommand::Uclamp {
            group,
            bound,
            value,
        } => format!("tweak uclamp {} {} {}", group, bound, value),
        TweakCommand::Preset(profile) => format!("preset {}", profile),
    }
}

fn preset_command_lines(profile: TweakProfile) -> Vec<String> {
    let (cpu_governor, devfreq_governor, swappiness, dirty_ratio, dirty_background_ratio) =
        match profile {
            TweakProfile::Balance => ("balance", "powersave", "10", "15", "5"),
            TweakProfile::Performance => ("performance", "performance", "5", "30", "10"),
            TweakProfile::Power => ("powersave", "powersave", "60", "5", "2"),
        };
    let (preload_cpuset, top_app_uclamp, background_uclamp) = match profile {
        TweakProfile::Balance => ("all", "20", "10"),
        TweakProfile::Performance => ("all", "60", "5"),
        TweakProfile::Power => ("little", "10", "15"),
    };

    vec![
        format!("tweak governor cpu {}", cpu_governor),
        format!("tweak governor devfreq {}", devfreq_governor),
        format!("tweak write /proc/sys/vm/swappiness {}", swappiness),
        "tweak write /proc/sys/vm/vfs_cache_pressure 50".to_string(),
        format!(
            "tweak write /proc/sys/vm/dirty_background_ratio {}",
            dirty_background_ratio
        ),
        format!("tweak write /proc/sys/vm/dirty_ratio {}", dirty_ratio),
        format!("tweak cpuset top-app {}", preload_cpuset),
        "tweak cpuset foreground all".to_string(),
        "tweak cpuset background little".to_string(),
        "tweak cpuset system-background little".to_string(),
        format!("tweak uclamp top-app min {}", top_app_uclamp),
        "tweak uclamp top-app max 100".to_string(),
        format!("tweak uclamp background max {}", background_uclamp),
    ]
}

fn execute_command(
    command: &TweakCommand,
    cache: &mut TweakCache,
    summary: &mut TweakApplySummary,
) {
    match command {
        TweakCommand::Write { path, value } => {
            summary.record_write_result(write_value(path, value));
        }
        TweakCommand::GovernorCpu { target } => {
            execute_cpu_governor(target, cache, summary);
        }
        TweakCommand::GovernorDevfreq { target } => {
            execute_devfreq_governor(target, summary);
        }
        TweakCommand::Cpuset { group, target } => {
            execute_cpuset(group, *target, cache, summary);
        }
        TweakCommand::Uclamp {
            group,
            bound,
            value,
        } => {
            let path = format!("/dev/cpuctl/{}/cpu.uclamp.{}", group, bound);
            summary.record_write_result(write_value(&path, &value.to_string()));
        }
        TweakCommand::Preset(profile) => {
            for preset_command in preset_command_lines(*profile) {
                if let Ok(expanded) = expand_command_line(&preset_command) {
                    for command in expanded {
                        execute_command(&command, cache, summary);
                    }
                }
            }
        }
    }
}

fn execute_cpu_governor(target: &str, cache: &mut TweakCache, summary: &mut TweakApplySummary) {
    const BALANCE_GOV_PRIORITY: &[&str] =
        &["schedutil", "simple_ondemand", "schedhorizon", "sugov_ext"];

    for_each_dir_entry(&cpufreq_dir(), |name| {
        if !name.starts_with("policy") {
            return;
        }

        let policy_dir = cpufreq_dir().join(name);
        let governor_path = policy_dir.join("scaling_governor");
        let available_path = policy_dir.join("scaling_available_governors");
        if !governor_path.exists() {
            summary.record_write_result(WriteResult::PathMissing);
            return;
        }

        let desired = if target == "balance" {
            let cache_key = format!("{}_governor", name);
            if let Some(cached) = cache.entries.get(&cache_key).cloned() {
                Some(cached)
            } else if let Ok(available) = fs::read_to_string(&available_path) {
                pick_best(&available, BALANCE_GOV_PRIORITY).map(|best| {
                    cache.insert(cache_key, best.to_string());
                    best.to_string()
                })
            } else {
                None
            }
        } else {
            match fs::read_to_string(&available_path) {
                Ok(available) if governor_is_available(&available, target) => {
                    Some(target.to_string())
                }
                Ok(_) => {
                    summary.record_write_result(WriteResult::GovernorUnsupported);
                    None
                }
                Err(error) if error.kind() == io::ErrorKind::NotFound => {
                    summary.record_write_result(WriteResult::GovernorUnsupported);
                    None
                }
                Err(error) => {
                    summary.record_write_result(WriteResult::Failed(format!(
                        "Failed to read available governors for {}: {}",
                        name, error
                    )));
                    None
                }
            }
        };

        if let Some(governor) = desired {
            summary.record_write_result(write_value(
                &format!("/sys/devices/system/cpu/cpufreq/{}/scaling_governor", name),
                &governor,
            ));
        } else if target == "balance" {
            summary.record_write_result(WriteResult::GovernorUnsupported);
        }
    });
}

fn execute_devfreq_governor(target: &str, summary: &mut TweakApplySummary) {
    for_each_dir_entry(&devfreq_dir(), |name| {
        let path = format!("/sys/class/devfreq/{}/governor", name);
        summary.record_write_result(write_value(&path, target));
    });
}

fn execute_cpuset(
    group: &str,
    target: CpusetTarget,
    cache: &mut TweakCache,
    summary: &mut TweakApplySummary,
) {
    discover_cpu_topology(cache);
    let cache_key = match target {
        CpusetTarget::All => "cpuset_all",
        CpusetTarget::Little => "cpuset_little",
    };
    if let Some(cpus) = cache.entries.get(cache_key) {
        let path = format!("/dev/cpuset/{}/cpus", group);
        summary.record_write_result(write_value(&path, cpus));
    } else {
        summary.record_write_result(WriteResult::Skipped);
    }
}

fn write_value(logical_path: &str, value: &str) -> WriteResult {
    let Some(actual_path) = map_runtime_path(logical_path) else {
        return WriteResult::Failed(format!("Path '{}' is not allowed", logical_path));
    };
    if !actual_path.exists() {
        return WriteResult::PathMissing;
    }
    if let Ok(current) = fs::read_to_string(&actual_path)
        && current.trim() == value
    {
        return WriteResult::Skipped;
    }
    match fs::write(&actual_path, value) {
        Ok(_) => WriteResult::Success,
        Err(error) => WriteResult::Failed(format!("{} -> '{}': {}", logical_path, value, error)),
    }
}

fn discover_cpu_topology(cache: &mut TweakCache) {
    if cache.entries.contains_key("cpuset_little") && cache.entries.contains_key("cpuset_all") {
        return;
    }

    let mut little_cores = Vec::new();
    let mut all_cores = Vec::new();
    for_each_dir_entry(&cpu_dir(), |name| {
        if name.starts_with("cpu")
            && name[3..].chars().all(char::is_numeric)
            && let Ok(id) = name[3..].parse::<u32>()
        {
            all_cores.push(id);
            let capacity_path = cpu_dir().join(name).join("cpu_capacity");
            if let Ok(capacity) = fs::read_to_string(capacity_path)
                && let Ok(value) = capacity.trim().parse::<u32>()
                && value > 0
                && value < LITTLE_CORE_CAP_THRESHOLD
            {
                little_cores.push(id);
            }
        }
    });

    if !little_cores.is_empty() {
        cache.insert("cpuset_little", format_cpu_list(&little_cores));
    }
    if !all_cores.is_empty() {
        cache.insert("cpuset_all", format_cpu_list(&all_cores));
    }
}

fn format_cpu_list(cpus: &[u32]) -> String {
    cpus.iter()
        .map(|cpu| cpu.to_string())
        .collect::<Vec<_>>()
        .join(",")
}

fn for_each_dir_entry<F: FnMut(&str)>(dir: &Path, mut f: F) {
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            if let Some(name) = entry.file_name().to_str() {
                f(name);
            }
        }
    }
}

fn pick_best<'a>(available: &str, priorities: &[&'a str]) -> Option<&'a str> {
    priorities
        .iter()
        .copied()
        .find(|target| governor_is_available(available, target))
}

fn governor_is_available(available: &str, target: &str) -> bool {
    available.split_whitespace().any(|entry| entry == target)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::sync::{Mutex, OnceLock};

    fn test_env_lock() -> &'static Mutex<()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
    }

    struct TempDir {
        path: PathBuf,
    }

    impl TempDir {
        fn new(name: &str) -> Self {
            let path = std::env::temp_dir().join(format!(
                "coreshift_tweaks_{}_{}_{}",
                name,
                std::process::id(),
                now_ms()
            ));
            fs::create_dir_all(&path).unwrap();
            Self { path }
        }

        fn path(&self) -> &Path {
            &self.path
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.path);
        }
    }

    fn with_test_env(root: &TempDir, f: impl FnOnce()) {
        unsafe {
            std::env::set_var("COREPOLICY_TEST_PROC_ROOT", root.path().join("proc"));
            std::env::set_var("COREPOLICY_TEST_SYS_ROOT", root.path().join("sys"));
            std::env::set_var(
                "COREPOLICY_TEST_CPUSET_ROOT",
                root.path().join("dev/cpuset"),
            );
            std::env::set_var(
                "COREPOLICY_TEST_CPUCTL_ROOT",
                root.path().join("dev/cpuctl"),
            );
            std::env::set_var(
                "COREPOLICY_TEST_TWEAK_CACHE_FILE",
                root.path().join("tweak_cache.json"),
            );
            std::env::set_var(
                "COREPOLICY_TEST_TWEAK_STATUS_FILE",
                root.path().join("tweak_status.json"),
            );
        }
        fs::create_dir_all(root.path().join("proc/sys/vm")).unwrap();
        fs::create_dir_all(root.path().join("sys/devices/system/cpu/cpufreq/policy0")).unwrap();
        fs::create_dir_all(root.path().join("sys/class/devfreq/gpu0")).unwrap();
        fs::create_dir_all(root.path().join("dev/cpuset/top-app")).unwrap();
        fs::create_dir_all(root.path().join("dev/cpuctl/top-app")).unwrap();
        fs::create_dir_all(root.path().join("dev/cpuctl/background")).unwrap();
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
    fn parses_raw_write_command() {
        assert_eq!(
            parse_tweak_command_line("tweak write /proc/sys/vm/swappiness 5").unwrap(),
            TweakCommand::Write {
                path: "/proc/sys/vm/swappiness".to_string(),
                value: "5".to_string(),
            }
        );
    }

    #[test]
    fn rejects_invalid_command() {
        assert!(parse_tweak_command_line("tweak shell rm -rf /").is_err());
    }

    #[test]
    fn enforces_path_whitelist() {
        assert!(parse_tweak_command_line("tweak write /data/local/tmp/nope 1").is_err());
    }

    #[test]
    fn preset_expansion_produces_commands() {
        let normalized = normalize_commands(&["preset performance".to_string()]).unwrap();
        assert!(!normalized.is_empty());
        assert!(
            normalized
                .iter()
                .any(|command| command == "tweak governor cpu performance")
        );
    }

    #[test]
    fn cache_dirty_save_path_new_entry() {
        let _guard = test_env_lock().lock().unwrap();
        let dir = TempDir::new("cache_save");
        let cache_path = dir.path().join("tweak_cache.json");
        let mut cache = TweakCache {
            schema_version: 1,
            updated_ms: 1,
            entries: HashMap::from([("cpuset_little".to_string(), "0-3".to_string())]),
            dirty: false,
        };

        cache.save_to_path(&cache_path).unwrap();
        assert!(!cache.dirty);

        let mut reloaded = TweakCache::load_from_path(&cache_path);
        reloaded.insert("cpuset_all", "0-7");
        assert!(reloaded.dirty);
    }

    #[test]
    fn tweak_cache_clear_is_idempotent() {
        let _guard = test_env_lock().lock().unwrap();
        let dir = TempDir::new("cache_clear");
        let cache_path = dir.path().join("tweak_cache.json");
        assert!(TweakCache::clear_path(&cache_path).is_ok());
        write_file(&cache_path, "{}");
        assert!(TweakCache::clear_path(&cache_path).is_ok());
        assert!(!cache_path.exists());
    }

    #[test]
    fn governor_validation_skips_unsupported_target() {
        let _guard = test_env_lock().lock().unwrap();
        let dir = TempDir::new("governor_skip");
        with_test_env(&dir, || {
            write_file(
                &dir.path()
                    .join("sys/devices/system/cpu/cpufreq/policy0/scaling_governor"),
                "schedutil",
            );
            write_file(
                &dir.path()
                    .join("sys/devices/system/cpu/cpufreq/policy0/scaling_available_governors"),
                "schedutil powersave",
            );
            let summary =
                apply_tweak_commands("test", &["tweak governor cpu performance".to_string()]);
            assert_eq!(summary.successful_writes, 0);
            assert_eq!(summary.skipped_writes, 1);
            assert_eq!(
                fs::read_to_string(
                    dir.path()
                        .join("sys/devices/system/cpu/cpufreq/policy0/scaling_governor")
                )
                .unwrap(),
                "schedutil"
            );
        });
    }

    #[test]
    fn governor_validation_applies_supported_target() {
        let _guard = test_env_lock().lock().unwrap();
        let dir = TempDir::new("governor_apply");
        with_test_env(&dir, || {
            write_file(
                &dir.path()
                    .join("sys/devices/system/cpu/cpufreq/policy0/scaling_governor"),
                "schedutil",
            );
            write_file(
                &dir.path()
                    .join("sys/devices/system/cpu/cpufreq/policy0/scaling_available_governors"),
                "schedutil performance powersave",
            );
            let summary =
                apply_tweak_commands("test", &["tweak governor cpu performance".to_string()]);
            assert_eq!(summary.successful_writes, 1);
            assert_eq!(
                fs::read_to_string(
                    dir.path()
                        .join("sys/devices/system/cpu/cpufreq/policy0/scaling_governor")
                )
                .unwrap(),
                "performance"
            );
        });
    }

    #[test]
    fn summary_accounting_invariant_holds() {
        let _guard = test_env_lock().lock().unwrap();
        let dir = TempDir::new("summary");
        with_test_env(&dir, || {
            write_file(&dir.path().join("proc/sys/vm/swappiness"), "60");
            let summary = apply_tweak_commands(
                "test",
                &[
                    "tweak write /proc/sys/vm/swappiness 60".to_string(),
                    "tweak write /proc/sys/vm/dirty_ratio 15".to_string(),
                ],
            );
            assert_eq!(
                summary.attempted_writes,
                summary.successful_writes + summary.skipped_writes + summary.failed_writes
            );
        });
    }
}
