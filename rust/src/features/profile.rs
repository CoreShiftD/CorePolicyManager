use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;
use std::fs;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

pub const CATEGORIES_FILE: &str = "/data/local/tmp/coreshift/profiles_category.json";

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
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
        let s = match self {
            ProfileClass::Game => "game",
            ProfileClass::Social => "social",
            ProfileClass::Tool => "tool",
            ProfileClass::Launcher => "launcher",
            ProfileClass::Keyboard => "keyboard",
            ProfileClass::System => "system",
            ProfileClass::Unknown => "unknown",
        };
        write!(f, "{}", s)
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ProfileRecommendation {
    Performance,
    Balanced,
    Conservative,
    Neutral,
}

impl fmt::Display for ProfileRecommendation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            ProfileRecommendation::Performance => "performance",
            ProfileRecommendation::Balanced => "balanced",
            ProfileRecommendation::Conservative => "conservative",
            ProfileRecommendation::Neutral => "neutral",
        };
        write!(f, "{}", s)
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Default, PartialEq)]
pub struct ProfileFeature {
    pub enabled: bool,
    pub session_started_ms: u64,
    pub foreground_switch_count: u64,
    pub top_apps: HashMap<String, u64>,
}

impl ProfileFeature {
    pub fn on_foreground_changed(
        &mut self,
        prev_pkg: Option<&str>,
        new_pkg: Option<&str>,
        now_ms: u64,
    ) {
        if prev_pkg == new_pkg {
            return;
        }

        // Close previous session
        if let Some(pkg) = prev_pkg {
            let elapsed = now_ms.saturating_sub(self.session_started_ms) / 1000;
            *self.top_apps.entry(pkg.to_string()).or_insert(0) += elapsed;
        }

        // Start new session
        if new_pkg.is_some() {
            self.session_started_ms = now_ms;
            if prev_pkg.is_some() {
                self.foreground_switch_count += 1;
            }
        }

        // Enforce max 64 apps
        if self.top_apps.len() > 64
            && let Some(min_key) = self
                .top_apps
                .iter()
                .min_by_key(|&(_, v)| v)
                .map(|(k, _)| k.clone())
        {
            self.top_apps.remove(&min_key);
        }
    }

    pub fn get_recommendation(class: &ProfileClass) -> ProfileRecommendation {
        match class {
            ProfileClass::Game => ProfileRecommendation::Performance,
            ProfileClass::Social | ProfileClass::Tool => ProfileRecommendation::Balanced,
            ProfileClass::Launcher | ProfileClass::Keyboard => ProfileRecommendation::Conservative,
            ProfileClass::System | ProfileClass::Unknown => ProfileRecommendation::Neutral,
        }
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
        if let Ok(content) = fs::read_to_string(CATEGORIES_FILE) {
            serde_json::from_str(&content).unwrap_or_else(|_| CategoryDatabase::default())
        } else {
            CategoryDatabase::default()
        }
    }

    pub fn save(&mut self) -> Result<(), std::io::Error> {
        self.updated_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;

        let path = Path::new(CATEGORIES_FILE);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let temp = format!("{}.tmp", CATEGORIES_FILE);
        fs::write(&temp, serde_json::to_string_pretty(self)?)?;
        fs::rename(&temp, CATEGORIES_FILE)
    }

    pub fn is_supported_category(cat: &str) -> bool {
        matches!(
            cat,
            "game" | "social" | "tool" | "launcher" | "keyboard" | "system"
        )
    }

    pub fn classify(&self, pkg: &str) -> ProfileClass {
        for (cat, pkgs) in &self.categories {
            if pkgs.iter().any(|p| p == pkg) {
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
            pkgs.retain(|p| p != pkg);
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
            pkgs.retain(|p| p != pkg);
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
