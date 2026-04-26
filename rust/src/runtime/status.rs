use crate::features::profile::CategoryDatabase;
use crate::features::profile::ProfileFeature;
use crate::paths::STATUS_FILE;
use crate::runtime::foreground::ForegroundSnapshot;
use crate::runtime::indexer::AppPathIndex;
use crate::runtime::pressure::PressureMetrics;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum PreloadResult {
    Ok,
    Cooldown,
    Partial,
    Failed,
    NoCandidates,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default, PartialEq)]
pub struct DaemonStatus {
    pub daemon: DaemonInfo,
    pub foreground: ForegroundInfo,
    pub pressure: PressureMetrics,
    pub features: FeatureState,
    pub app_index: AppPathIndex,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default, PartialEq)]
pub struct DaemonInfo {
    pub alive: bool,
    pub mode: String,
    pub started_ms: u64,
    pub warnings: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default, PartialEq)]
pub struct ForegroundInfo {
    pub package: Option<String>,
    pub pid: Option<i32>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default, PartialEq)]
pub struct FeatureState {
    pub preload: PreloadInfo,
    pub profile: ProfileFeature,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default, PartialEq)]
pub struct PreloadInfo {
    pub last_package: Option<String>,
    pub file_count: usize,
    pub files_failed: usize,
    pub bytes: u64,
    pub discovery_ms: u64,
    pub readahead_ms: u64,
    pub total_ms: u64,
    pub result_code: Option<PreloadResult>,
}

#[derive(Serialize)]
pub struct PublicPressure {
    pub supported: bool,
    pub cpu_avg10: Option<f32>,
    pub memory_avg10: Option<f32>,
    pub io_avg10: Option<f32>,
}

#[derive(Serialize)]
pub struct PublicDaemon {
    pub alive: bool,
    pub mode: String,
    pub uptime_secs: u64,
}

#[derive(Serialize)]
pub struct PublicForeground {
    pub package: Option<String>,
    pub pid: Option<i32>,
    pub session_secs: u64,
}

#[derive(Serialize)]
pub struct PublicProfile {
    pub class: String,
    pub recommendation: String,
    pub switch_count: u64,
    pub top_apps: Vec<PublicAppStat>,
}

#[derive(Serialize)]
pub struct PublicAppStat {
    pub package: String,
    pub total_secs: u64,
}

#[derive(Serialize)]
pub struct PublicPreload {
    pub result: Option<PreloadResult>,
    pub bytes: u64,
    pub duration_ms: u64,
}

#[derive(Serialize)]
pub struct PublicFeatures {
    pub preload: PublicPreload,
    pub profile: PublicProfile,
}

#[derive(Serialize)]
pub struct PublicStatus {
    pub daemon: PublicDaemon,
    pub foreground: PublicForeground,
    pub pressure: PublicPressure,
    pub features: PublicFeatures,
    pub app_index: PublicAppIndex,
}

#[derive(Serialize)]
pub struct PublicAppIndex {
    pub ready: bool,
    pub packages: usize,
}

impl DaemonStatus {
    pub fn to_public_status(&self, db: &CategoryDatabase) -> PublicStatus {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;

        let uptime_secs = now.saturating_sub(self.daemon.started_ms) / 1000;
        let session_secs = if self.foreground.package.is_some() {
            now.saturating_sub(self.features.profile.session_started_ms) / 1000
        } else {
            0
        };

        let mut top_apps = self.features.profile.top_apps.clone();
        if let Some(pkg) = &self.foreground.package {
            let active_elapsed =
                now.saturating_sub(self.features.profile.session_started_ms) / 1000;
            *top_apps.entry(pkg.clone()).or_insert(0) += active_elapsed;
        }

        let mut sorted_apps: Vec<_> = top_apps.into_iter().collect();
        sorted_apps.sort_by_key(|&(_, total)| std::cmp::Reverse(total));
        sorted_apps.truncate(3);

        let class = db.classify(self.foreground.package.as_deref().unwrap_or(""));
        let rec = crate::features::profile::ProfileFeature::get_recommendation(&class);

        PublicStatus {
            daemon: PublicDaemon {
                alive: self.daemon.alive,
                mode: self.daemon.mode.clone(),
                uptime_secs,
            },
            foreground: PublicForeground {
                package: self.foreground.package.clone(),
                pid: self.foreground.pid,
                session_secs,
            },
            pressure: PublicPressure {
                supported: self.pressure.supported,
                cpu_avg10: self.pressure.cpu_some_avg10,
                memory_avg10: self.pressure.memory_some_avg10,
                io_avg10: self.pressure.io_some_avg10,
            },
            features: PublicFeatures {
                preload: PublicPreload {
                    result: self.features.preload.result_code,
                    bytes: self.features.preload.bytes,
                    duration_ms: self.features.preload.total_ms,
                },
                profile: PublicProfile {
                    class: class.to_string(),
                    recommendation: rec.to_string(),
                    switch_count: self.features.profile.foreground_switch_count,
                    top_apps: sorted_apps
                        .into_iter()
                        .map(|(p, t)| PublicAppStat {
                            package: p,
                            total_secs: t,
                        })
                        .collect(),
                },
            },
            app_index: PublicAppIndex {
                ready: !self.app_index.stale && self.app_index.built_ms > 0,
                packages: self.app_index.entries.len(),
            },
        }
    }

    pub fn apply_foreground_snapshot(&mut self, snapshot: &ForegroundSnapshot) {
        self.foreground.pid = snapshot.pid;
        self.foreground.package = snapshot.package.clone();
    }

    pub fn write(&self) -> Result<(), std::io::Error> {
        let path = Path::new(STATUS_FILE);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let temp_path = format!("{}.tmp", STATUS_FILE);
        let json = serde_json::to_string_pretty(self).map_err(std::io::Error::other)?;
        fs::write(&temp_path, json)?;
        fs::rename(&temp_path, STATUS_FILE)?;
        Ok(())
    }

    pub fn read() -> Option<Self> {
        if let Ok(content) = fs::read_to_string(STATUS_FILE) {
            serde_json::from_str(&content).ok()
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn public_status_uses_injected_db() {
        let mut status = DaemonStatus::default();
        status.daemon.started_ms = 1;
        status.foreground.package = Some("com.example.game".to_string());

        let mut db = CategoryDatabase::default();
        assert!(db.add("game", "com.example.game"));

        let public = status.to_public_status(&db);
        assert_eq!(public.features.profile.class, "game");
    }
}
