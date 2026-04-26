use crate::features::profile::{CategoryDatabase, ProfileFeature};
use crate::paths::{APP_INDEX_STATUS_FILE, PRELOAD_STATUS_FILE, PROFILE_STATUS_FILE, STATUS_FILE};
use crate::runtime::foreground::ForegroundSnapshot;
use crate::runtime::pressure::PressureMetrics;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

fn schema_version_1() -> u32 {
    1
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum PreloadResult {
    Ok,
    Cooldown,
    Partial,
    Failed,
    NoCandidates,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct DaemonStatus {
    #[serde(default = "schema_version_1")]
    pub schema_version: u32,
    pub daemon: DaemonInfo,
    pub foreground: ForegroundInfo,
    pub features: FeatureFlags,
    pub pressure: PressureStatus,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default, PartialEq)]
pub struct DaemonInfo {
    pub alive: bool,
    pub started_ms: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub device_uptime_secs: Option<u64>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default, PartialEq)]
pub struct ForegroundInfo {
    pub package: Option<String>,
    pub pid: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_started_ms: Option<u64>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default, PartialEq)]
pub struct FeatureFlags {
    pub preload: bool,
    pub profile: bool,
    pub pressure: bool,
    pub app_index: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default, PartialEq)]
pub struct PressureStatus {
    pub supported: bool,
    pub cpu_avg10: Option<f32>,
    pub memory_avg10: Option<f32>,
    pub io_avg10: Option<f32>,
    pub last_refresh_ms: u64,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct ProfileStatusFile {
    #[serde(default = "schema_version_1")]
    pub schema_version: u32,
    pub foreground_switch_count: u64,
    pub top_apps: Vec<ProfileAppStat>,
    pub current_class: String,
    pub recommendation: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default, PartialEq)]
pub struct ProfileAppStat {
    pub package: String,
    pub total_secs: u64,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct PreloadStatusFile {
    #[serde(default = "schema_version_1")]
    pub schema_version: u32,
    pub last_package: Option<String>,
    pub file_count: usize,
    pub files_failed: usize,
    pub bytes: u64,
    pub discovery_ms: u64,
    pub readahead_ms: u64,
    pub total_ms: u64,
    pub result: Option<PreloadResult>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct AppIndexStatusFile {
    #[serde(default = "schema_version_1")]
    pub schema_version: u32,
    pub ready: bool,
    pub packages: usize,
    pub built_ms: u64,
    pub rebuild_ms: u64,
    pub duration_ms: u64,
    pub stale: bool,
    pub rebuild_success_count: u64,
    pub rebuild_fail_count: u64,
    pub last_error: Option<String>,
}

#[derive(Serialize, Debug, Clone, PartialEq)]
pub struct PublicStatus {
    pub alive: bool,
    pub uptime_secs: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub device_uptime_secs: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub app: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pid: Option<i32>,
    pub session_secs: u64,
    pub features: FeatureFlags,
    pub pressure: PublicPressure,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub profile: Option<PublicProfile>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub preload: Option<PublicPreload>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub app_index: Option<PublicAppIndex>,
}

#[derive(Serialize, Debug, Clone, PartialEq)]
pub struct PublicPressure {
    pub cpu_avg10: Option<f32>,
    pub memory_avg10: Option<f32>,
    pub io_avg10: Option<f32>,
}

#[derive(Serialize, Debug, Clone, PartialEq)]
pub struct PublicProfile {
    pub class: String,
    pub recommendation: String,
    pub switch_count: u64,
    pub top_apps: Vec<ProfileAppStat>,
}

#[derive(Serialize, Debug, Clone, PartialEq)]
pub struct PublicPreload {
    pub result: Option<PreloadResult>,
    pub total_ms: u64,
}

#[derive(Serialize, Debug, Clone, PartialEq)]
pub struct PublicAppIndex {
    pub ready: bool,
    pub packages: usize,
    pub stale: bool,
}

impl Default for DaemonStatus {
    fn default() -> Self {
        Self {
            schema_version: schema_version_1(),
            daemon: DaemonInfo::default(),
            foreground: ForegroundInfo::default(),
            features: FeatureFlags::default(),
            pressure: PressureStatus::default(),
        }
    }
}

impl Default for ProfileStatusFile {
    fn default() -> Self {
        Self {
            schema_version: schema_version_1(),
            foreground_switch_count: 0,
            top_apps: Vec::new(),
            current_class: String::new(),
            recommendation: String::new(),
        }
    }
}

impl Default for PreloadStatusFile {
    fn default() -> Self {
        Self {
            schema_version: schema_version_1(),
            last_package: None,
            file_count: 0,
            files_failed: 0,
            bytes: 0,
            discovery_ms: 0,
            readahead_ms: 0,
            total_ms: 0,
            result: None,
        }
    }
}

impl Default for AppIndexStatusFile {
    fn default() -> Self {
        Self {
            schema_version: schema_version_1(),
            ready: false,
            packages: 0,
            built_ms: 0,
            rebuild_ms: 0,
            duration_ms: 0,
            stale: false,
            rebuild_success_count: 0,
            rebuild_fail_count: 0,
            last_error: None,
        }
    }
}

impl DaemonStatus {
    pub fn apply_foreground_snapshot(&mut self, snapshot: &ForegroundSnapshot) {
        self.foreground.pid = snapshot.pid;
        self.foreground.package = snapshot.package.clone();
    }

    pub fn write_if_changed(
        &self,
        last_written: &mut Option<DaemonStatus>,
    ) -> Result<bool, std::io::Error> {
        write_json_file_if_changed(STATUS_FILE, self, last_written)
    }

    pub fn read() -> Option<Self> {
        read_json_file(STATUS_FILE)
    }
}

impl PressureStatus {
    pub fn from_metrics(metrics: &PressureMetrics) -> Self {
        Self {
            supported: metrics.supported,
            cpu_avg10: metrics.cpu_some_avg10,
            memory_avg10: metrics.memory_some_avg10,
            io_avg10: metrics.io_some_avg10,
            last_refresh_ms: metrics.last_refresh_ms,
        }
    }
}

impl ProfileStatusFile {
    pub fn from_feature(
        profile: &ProfileFeature,
        current_pkg: Option<&str>,
        db: &CategoryDatabase,
    ) -> Self {
        let class = db.classify(current_pkg.unwrap_or(""));
        let recommendation = ProfileFeature::get_recommendation(&class);
        let top_apps = profile
            .snapshot_top_apps()
            .into_iter()
            .map(|(package, total_secs)| ProfileAppStat {
                package,
                total_secs,
            })
            .collect();

        Self {
            schema_version: 1,
            foreground_switch_count: profile.foreground_switch_count,
            top_apps,
            current_class: class.to_string(),
            recommendation: recommendation.to_string(),
        }
    }

    pub fn write_if_changed(
        &self,
        last_written: &mut Option<ProfileStatusFile>,
    ) -> Result<bool, std::io::Error> {
        write_json_file_if_changed(PROFILE_STATUS_FILE, self, last_written)
    }
}

impl PreloadStatusFile {
    pub fn write_if_changed(
        &self,
        last_written: &mut Option<PreloadStatusFile>,
    ) -> Result<bool, std::io::Error> {
        write_json_file_if_changed(PRELOAD_STATUS_FILE, self, last_written)
    }
}

impl AppIndexStatusFile {
    pub fn write_if_changed(
        &self,
        last_written: &mut Option<AppIndexStatusFile>,
    ) -> Result<bool, std::io::Error> {
        write_json_file_if_changed(APP_INDEX_STATUS_FILE, self, last_written)
    }
}

pub fn write_json_file_if_changed<T>(
    path: &str,
    value: &T,
    last_written: &mut Option<T>,
) -> Result<bool, std::io::Error>
where
    T: Serialize + Clone + PartialEq,
{
    if last_written.as_ref() == Some(value) {
        return Ok(false);
    }

    let path_obj = Path::new(path);
    if let Some(parent) = path_obj.parent() {
        fs::create_dir_all(parent)?;
    }

    let temp_path = format!("{}.tmp", path);
    let json = serde_json::to_string_pretty(value).map_err(std::io::Error::other)?;
    fs::write(&temp_path, json)?;
    fs::rename(&temp_path, path)?;
    *last_written = Some(value.clone());
    Ok(true)
}

pub fn read_public_status(db: &CategoryDatabase) -> Option<PublicStatus> {
    read_public_status_from_paths(
        STATUS_FILE,
        PROFILE_STATUS_FILE,
        PRELOAD_STATUS_FILE,
        APP_INDEX_STATUS_FILE,
        db,
    )
}

pub fn read_public_status_from_paths(
    core_path: &str,
    profile_path: &str,
    preload_path: &str,
    app_index_path: &str,
    _db: &CategoryDatabase,
) -> Option<PublicStatus> {
    let core: DaemonStatus = read_json_file(core_path)?;
    let profile: Option<ProfileStatusFile> = read_json_file(profile_path);
    let preload: Option<PreloadStatusFile> = read_json_file(preload_path);
    let app_index: Option<AppIndexStatusFile> = read_json_file(app_index_path);

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64;

    let uptime_secs = now.saturating_sub(core.daemon.started_ms) / 1000;
    let session_secs = if let Some(session_started_ms) = core.foreground.session_started_ms {
        now.saturating_sub(session_started_ms) / 1000
    } else {
        0
    };

    Some(PublicStatus {
        alive: core.daemon.alive,
        uptime_secs,
        device_uptime_secs: core.daemon.device_uptime_secs,
        app: core.foreground.package,
        pid: core.foreground.pid,
        session_secs,
        features: core.features.clone(),
        pressure: PublicPressure {
            cpu_avg10: core.pressure.cpu_avg10,
            memory_avg10: core.pressure.memory_avg10,
            io_avg10: core.pressure.io_avg10,
        },
        profile: if core.features.profile {
            profile.map(|profile| PublicProfile {
                class: profile.current_class,
                recommendation: profile.recommendation,
                switch_count: profile.foreground_switch_count,
                top_apps: profile.top_apps,
            })
        } else {
            None
        },
        preload: if core.features.preload {
            preload.map(|preload| PublicPreload {
                result: preload.result,
                total_ms: preload.total_ms,
            })
        } else {
            None
        },
        app_index: if core.features.app_index {
            app_index.map(|app_index| PublicAppIndex {
                ready: app_index.ready,
                packages: app_index.packages,
                stale: app_index.stale,
            })
        } else {
            None
        },
    })
}

pub fn read_device_uptime_secs() -> Option<u64> {
    let content = fs::read_to_string("/proc/uptime").ok()?;
    let first = content.split_whitespace().next()?;
    let secs = first.parse::<f64>().ok()?;
    Some(secs.floor() as u64)
}

fn read_json_file<T: DeserializeOwned>(path: &str) -> Option<T> {
    let content = fs::read_to_string(path).ok()?;
    serde_json::from_str(&content).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write_json<T: Serialize>(path: &Path, value: &T) {
        fs::write(path, serde_json::to_string_pretty(value).unwrap()).unwrap();
    }

    fn test_paths(name: &str) -> (String, String, String, String) {
        let root = std::env::temp_dir().join(format!(
            "coreshift_status_test_{}_{}",
            name,
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        (
            root.join("status.json").to_string_lossy().into_owned(),
            root.join("profile_status.json")
                .to_string_lossy()
                .into_owned(),
            root.join("preload_status.json")
                .to_string_lossy()
                .into_owned(),
            root.join("app_index_status.json")
                .to_string_lossy()
                .into_owned(),
        )
    }

    fn core_status() -> DaemonStatus {
        DaemonStatus {
            schema_version: 1,
            daemon: DaemonInfo {
                alive: true,
                started_ms: 1,
                device_uptime_secs: Some(42),
            },
            foreground: ForegroundInfo {
                package: Some("com.example.game".to_string()),
                pid: Some(1234),
                session_started_ms: Some(11),
            },
            features: FeatureFlags {
                preload: true,
                profile: true,
                pressure: true,
                app_index: true,
            },
            pressure: PressureStatus {
                supported: true,
                cpu_avg10: Some(1.0),
                memory_avg10: Some(0.0),
                io_avg10: Some(0.0),
                last_refresh_ms: 20,
            },
        }
    }

    #[test]
    fn cli_computes_uptime_and_session() {
        let (core, profile, preload, app_index) = test_paths("uptime");
        write_json(Path::new(&core), &core_status());

        let db = CategoryDatabase::default();
        let public =
            read_public_status_from_paths(&core, &profile, &preload, &app_index, &db).unwrap();
        assert!(public.uptime_secs > 0);
        assert!(public.session_secs > 0);
    }

    #[test]
    fn missing_profile_file_degrades_gracefully() {
        let (core, profile, preload, app_index) = test_paths("missing_profile");
        write_json(Path::new(&core), &core_status());

        let db = CategoryDatabase::default();
        let public =
            read_public_status_from_paths(&core, &profile, &preload, &app_index, &db).unwrap();
        assert!(public.profile.is_none());
    }

    #[test]
    fn missing_preload_file_degrades_gracefully() {
        let (core, profile, preload, app_index) = test_paths("missing_preload");
        write_json(Path::new(&core), &core_status());

        let db = CategoryDatabase::default();
        let public =
            read_public_status_from_paths(&core, &profile, &preload, &app_index, &db).unwrap();
        assert!(public.preload.is_none());
    }

    #[test]
    fn missing_index_file_degrades_gracefully() {
        let (core, profile, preload, app_index) = test_paths("missing_index");
        write_json(Path::new(&core), &core_status());

        let db = CategoryDatabase::default();
        let public =
            read_public_status_from_paths(&core, &profile, &preload, &app_index, &db).unwrap();
        assert!(public.app_index.is_none());
    }

    #[test]
    fn no_duplicated_fields_in_merged_output() {
        let (core, profile, preload, app_index) = test_paths("dedupe");
        write_json(Path::new(&core), &core_status());

        let mut db = CategoryDatabase::default();
        assert!(db.add("game", "com.example.game"));
        write_json(
            Path::new(&profile),
            &ProfileStatusFile {
                schema_version: 1,
                foreground_switch_count: 7,
                top_apps: vec![ProfileAppStat {
                    package: "com.example.game".to_string(),
                    total_secs: 99,
                }],
                current_class: "game".to_string(),
                recommendation: "performance".to_string(),
            },
        );
        write_json(
            Path::new(&preload),
            &PreloadStatusFile {
                schema_version: 1,
                total_ms: 3,
                result: Some(PreloadResult::Ok),
                ..Default::default()
            },
        );
        write_json(
            Path::new(&app_index),
            &AppIndexStatusFile {
                schema_version: 1,
                ready: true,
                packages: 10,
                stale: false,
                ..Default::default()
            },
        );

        let public =
            read_public_status_from_paths(&core, &profile, &preload, &app_index, &db).unwrap();
        let json = serde_json::to_value(public).unwrap();
        let object = json.as_object().unwrap();
        assert!(!object.contains_key("started_ms"));
        assert!(!object.contains_key("session_started_ms"));
        assert!(!object.contains_key("enabled"));
        assert!(object.contains_key("features"));
    }

    #[test]
    fn feature_flags_only_sourced_from_status_json() {
        let (core, profile, preload, app_index) = test_paths("flags");
        let mut core_status = core_status();
        core_status.features.profile = false;
        core_status.features.preload = false;
        core_status.features.app_index = false;
        write_json(Path::new(&core), &core_status);
        write_json(
            Path::new(&profile),
            &ProfileStatusFile {
                schema_version: 1,
                current_class: "social".to_string(),
                recommendation: "balanced".to_string(),
                ..Default::default()
            },
        );
        write_json(
            Path::new(&preload),
            &PreloadStatusFile {
                schema_version: 1,
                result: Some(PreloadResult::Ok),
                ..Default::default()
            },
        );
        write_json(
            Path::new(&app_index),
            &AppIndexStatusFile {
                schema_version: 1,
                ready: true,
                ..Default::default()
            },
        );

        let db = CategoryDatabase::default();
        let public =
            read_public_status_from_paths(&core, &profile, &preload, &app_index, &db).unwrap();
        assert!(!public.features.profile);
        assert!(!public.features.preload);
        assert!(!public.features.app_index);
        assert!(public.profile.is_none());
        assert!(public.preload.is_none());
        assert!(public.app_index.is_none());
    }

    #[test]
    fn device_uptime_omitted_cleanly_when_unavailable() {
        let (core, profile, preload, app_index) = test_paths("device_uptime");
        let mut core_status = core_status();
        core_status.daemon.device_uptime_secs = None;
        write_json(Path::new(&core), &core_status);

        let db = CategoryDatabase::default();
        let public =
            read_public_status_from_paths(&core, &profile, &preload, &app_index, &db).unwrap();
        let json = serde_json::to_value(public).unwrap();
        assert!(json.get("device_uptime_secs").is_none());
    }

    #[test]
    fn schema_version_appears_in_all_raw_status_structs() {
        assert_eq!(DaemonStatus::default().schema_version, 1);
        assert_eq!(ProfileStatusFile::default().schema_version, 1);
        assert_eq!(PreloadStatusFile::default().schema_version, 1);
        assert_eq!(AppIndexStatusFile::default().schema_version, 1);
    }

    #[test]
    fn old_files_without_schema_version_still_parse() {
        let (core, profile, preload, app_index) = test_paths("old_schema");
        fs::write(
            &core,
            r#"{
  "daemon": {"alive": true, "started_ms": 1},
  "foreground": {"package": "com.example.game", "pid": 1234, "session_started_ms": 11},
  "features": {"preload": true, "profile": true, "pressure": true, "app_index": true},
  "pressure": {"supported": true, "cpu_avg10": 1.0, "memory_avg10": 0.0, "io_avg10": 0.0, "last_refresh_ms": 20}
}"#,
        )
        .unwrap();
        fs::write(
            &profile,
            r#"{
  "foreground_switch_count": 7,
  "top_apps": [],
  "current_class": "game",
  "recommendation": "performance"
}"#,
        )
        .unwrap();
        fs::write(
            &preload,
            r#"{
  "last_package": "com.example.game",
  "file_count": 1,
  "files_failed": 0,
  "bytes": 1,
  "discovery_ms": 1,
  "readahead_ms": 1,
  "total_ms": 2,
  "result": "ok"
}"#,
        )
        .unwrap();
        fs::write(
            &app_index,
            r#"{
  "ready": true,
  "packages": 3,
  "built_ms": 1,
  "rebuild_ms": 2,
  "duration_ms": 3,
  "stale": false,
  "rebuild_success_count": 1,
  "rebuild_fail_count": 0,
  "last_error": null
}"#,
        )
        .unwrap();

        let db = CategoryDatabase::default();
        let public =
            read_public_status_from_paths(&core, &profile, &preload, &app_index, &db).unwrap();
        assert!(public.alive);

        let core_raw: DaemonStatus = read_json_file(&core).unwrap();
        let profile_raw: ProfileStatusFile = read_json_file(&profile).unwrap();
        let preload_raw: PreloadStatusFile = read_json_file(&preload).unwrap();
        let app_index_raw: AppIndexStatusFile = read_json_file(&app_index).unwrap();
        assert_eq!(core_raw.schema_version, 1);
        assert_eq!(profile_raw.schema_version, 1);
        assert_eq!(preload_raw.schema_version, 1);
        assert_eq!(app_index_raw.schema_version, 1);
    }
}
