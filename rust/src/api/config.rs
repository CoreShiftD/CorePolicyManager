use crate::daemon::runtime::DaemonConfig;
use crate::runtime::status::{ALL_FEATURES, Feature};
use std::collections::BTreeSet;

pub use crate::daemon::runtime::DaemonConfig as RuntimeConfig;
pub use crate::runtime::status::{ALL_FEATURES as DEFAULT_FEATURES, Feature as RuntimeFeature};

pub fn daemon_config_from_features(features: &BTreeSet<Feature>) -> DaemonConfig {
    DaemonConfig {
        preload: features.contains(&Feature::Preload),
        usage: features.contains(&Feature::Usage),
        pressure: features.contains(&Feature::Pressure),
        app_index: features.contains(&Feature::AppIndex),
        profile: features.contains(&Feature::Profile),
    }
}

pub fn all_features() -> BTreeSet<Feature> {
    ALL_FEATURES.iter().copied().collect()
}
