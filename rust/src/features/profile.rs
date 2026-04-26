use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

pub const CATEGORIES_FILE: &str = "/data/local/tmp/coreshift/profiles_category.json";
pub const PROFILE_RULES_FILE: &str = "/data/local/tmp/coreshift/profile_rules.json";

pub(crate) fn categories_file_path() -> PathBuf {
    if let Some(path) = std::env::var_os("COREPOLICY_TEST_CATEGORIES_FILE") {
        return PathBuf::from(path);
    }

    PathBuf::from(CATEGORIES_FILE)
}

pub(crate) fn profile_rules_file_path() -> PathBuf {
    if let Some(path) = std::env::var_os("COREPOLICY_TEST_PROFILE_RULES_FILE") {
        return PathBuf::from(path);
    }

    PathBuf::from(PROFILE_RULES_FILE)
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ProfileClass {
    Game,
    Social,
    Tool,
    Launcher,
    Keyboard,
    System,
    Unknown,
}

impl fmt::Display for ProfileClass {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let value = match self {
            Self::Game => "game",
            Self::Social => "social",
            Self::Tool => "tool",
            Self::Launcher => "launcher",
            Self::Keyboard => "keyboard",
            Self::System => "system",
            Self::Unknown => "unknown",
        };
        write!(f, "{}", value)
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum PrivilegeMode {
    Root,
    Shell,
    #[default]
    Unknown,
}

impl fmt::Display for PrivilegeMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let value = match self {
            Self::Root => "root",
            Self::Shell => "shell",
            Self::Unknown => "unknown",
        };
        write!(f, "{}", value)
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum ProfilePriority {
    Performance,
    Balanced,
    #[default]
    Neutral,
}

impl fmt::Display for ProfilePriority {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let value = match self {
            Self::Performance => "performance",
            Self::Balanced => "balanced",
            Self::Neutral => "neutral",
        };
        write!(f, "{}", value)
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct ProfileRuleAction {
    pub preload: bool,
    pub priority: ProfilePriority,
    pub commands: Vec<String>,
}

impl Default for ProfileRuleAction {
    fn default() -> Self {
        Self {
            preload: false,
            priority: ProfilePriority::Neutral,
            commands: Vec::new(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Default)]
pub struct SelectedProfile {
    pub preload: bool,
    pub priority: ProfilePriority,
}

impl SelectedProfile {
    pub fn neutral() -> Self {
        Self::default()
    }
}

impl From<&ProfileRuleAction> for SelectedProfile {
    fn from(value: &ProfileRuleAction) -> Self {
        Self {
            preload: value.preload,
            priority: value.priority.clone(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct ProfileRulesFile {
    pub schema_version: u32,
    pub rules: HashMap<String, HashMap<String, ProfileRuleAction>>,
}

impl Default for ProfileRulesFile {
    fn default() -> Self {
        let mut rules = HashMap::new();
        rules.insert(
            "game".to_string(),
            HashMap::from([
                (
                    "root".to_string(),
                    ProfileRuleAction {
                        preload: true,
                        priority: ProfilePriority::Performance,
                        commands: Vec::new(),
                    },
                ),
                (
                    "shell".to_string(),
                    ProfileRuleAction {
                        preload: true,
                        priority: ProfilePriority::Balanced,
                        commands: Vec::new(),
                    },
                ),
            ]),
        );
        rules.insert(
            "social".to_string(),
            HashMap::from([
                (
                    "root".to_string(),
                    ProfileRuleAction {
                        preload: true,
                        priority: ProfilePriority::Balanced,
                        commands: Vec::new(),
                    },
                ),
                (
                    "shell".to_string(),
                    ProfileRuleAction {
                        preload: true,
                        priority: ProfilePriority::Balanced,
                        commands: Vec::new(),
                    },
                ),
            ]),
        );
        rules.insert(
            "tool".to_string(),
            HashMap::from([
                ("root".to_string(), ProfileRuleAction::default()),
                ("shell".to_string(), ProfileRuleAction::default()),
            ]),
        );

        Self {
            schema_version: 1,
            rules,
        }
    }
}

impl ProfileRulesFile {
    pub fn load() -> Self {
        if let Ok(content) = fs::read_to_string(profile_rules_file_path()) {
            serde_json::from_str(&content).unwrap_or_else(|_| Self::default())
        } else {
            Self::default()
        }
    }

    pub fn resolve(&self, class: &ProfileClass, privilege: &PrivilegeMode) -> SelectedProfile {
        let class_key = class.to_string();
        let privilege_key = privilege.to_string();
        self.rules
            .get(&class_key)
            .and_then(|rules| rules.get(&privilege_key))
            .map(SelectedProfile::from)
            .unwrap_or_else(SelectedProfile::neutral)
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Default, PartialEq)]
pub struct ProfileFeature {
    pub foreground_switch_count: u64,
    pub top_apps: HashMap<String, u64>,
}

impl ProfileFeature {
    pub fn on_foreground_changed(
        &mut self,
        prev_pkg: Option<&str>,
        new_pkg: Option<&str>,
        prev_session_started_ms: Option<u64>,
        now_ms: u64,
    ) {
        if prev_pkg == new_pkg {
            return;
        }

        if let (Some(pkg), Some(session_started_ms)) = (prev_pkg, prev_session_started_ms) {
            let elapsed = now_ms.saturating_sub(session_started_ms) / 1000;
            *self.top_apps.entry(pkg.to_string()).or_insert(0) += elapsed;
        }

        if new_pkg.is_some() && prev_pkg.is_some() {
            self.foreground_switch_count += 1;
        }

        if self.top_apps.len() > 64
            && let Some(min_key) = self
                .top_apps
                .iter()
                .min_by_key(|&(_, value)| value)
                .map(|(key, _)| key.clone())
        {
            self.top_apps.remove(&min_key);
        }
    }

    pub fn snapshot_top_apps(&self) -> Vec<(String, u64)> {
        let mut sorted_apps: Vec<_> = self.top_apps.clone().into_iter().collect();
        sorted_apps.sort_by_key(|&(_, total)| std::cmp::Reverse(total));
        sorted_apps
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CategoryDatabase {
    pub version: u32,
    pub updated_ms: u64,
    pub categories: HashMap<String, Vec<String>>,
}

impl CategoryDatabase {
    pub fn load() -> Self {
        if let Ok(content) = fs::read_to_string(categories_file_path()) {
            serde_json::from_str(&content).unwrap_or_else(|_| Self::default())
        } else {
            Self::default()
        }
    }

    pub fn save(&mut self) -> Result<(), std::io::Error> {
        self.updated_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;

        let path_buf = categories_file_path();
        let path = path_buf.as_path();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let temp = format!("{}.tmp", path.display());
        fs::write(&temp, serde_json::to_string_pretty(self)?)?;
        fs::rename(&temp, path)
    }

    pub fn is_supported_category(cat: &str) -> bool {
        matches!(
            cat,
            "game" | "social" | "tool" | "launcher" | "keyboard" | "system"
        )
    }

    pub fn classify(&self, pkg: &str) -> ProfileClass {
        for (cat, pkgs) in &self.categories {
            if pkgs.iter().any(|entry| entry == pkg) {
                return match cat.as_str() {
                    "game" => ProfileClass::Game,
                    "social" => ProfileClass::Social,
                    "tool" => ProfileClass::Tool,
                    "launcher" => ProfileClass::Launcher,
                    "keyboard" => ProfileClass::Keyboard,
                    "system" => ProfileClass::System,
                    _ => ProfileClass::Unknown,
                };
            }
        }
        ProfileClass::Unknown
    }

    pub fn add(&mut self, cat: &str, pkg: &str) -> bool {
        if !Self::is_supported_category(cat) {
            return false;
        }
        for pkgs in self.categories.values_mut() {
            pkgs.retain(|entry| entry != pkg);
        }
        self.categories
            .entry(cat.to_string())
            .or_default()
            .push(pkg.to_string());
        self.categories.get_mut(cat).unwrap().sort();
        self.categories.get_mut(cat).unwrap().dedup();
        true
    }

    pub fn remove(&mut self, pkg: &str) {
        for pkgs in self.categories.values_mut() {
            pkgs.retain(|entry| entry != pkg);
        }
    }
}

impl Default for CategoryDatabase {
    fn default() -> Self {
        let mut categories = HashMap::new();
        categories.insert("game".to_string(), vec![]);
        categories.insert("social".to_string(), vec![]);
        categories.insert("tool".to_string(), vec![]);
        categories.insert("launcher".to_string(), vec![]);
        categories.insert("keyboard".to_string(), vec![]);
        categories.insert("system".to_string(), vec![]);
        Self {
            version: 1,
            updated_ms: 0,
            categories,
        }
    }
}
