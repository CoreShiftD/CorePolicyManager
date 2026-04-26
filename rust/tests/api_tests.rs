use coreshift_policy::api::{json, config, features, daemon};

#[test]
fn test_json_api() {
    let test_file = "test_api.json";
    let data = features::ProfileRulesFile::default();
    
    json::write_json_file(test_file, &data).expect("Failed to write JSON");
    
    let content = std::fs::read_to_string(test_file).expect("Failed to read file");
    assert!(json::validate_json::<features::ProfileRulesFile>(&content));
    
    let loaded: features::ProfileRulesFile = json::read_json_file(test_file).expect("Failed to load JSON");
    assert_eq!(data.schema_version, loaded.schema_version);
    
    let pretty = json::to_pretty_json(&data).expect("Failed to serialize");
    assert!(pretty.contains("\"schema_version\": 1"));
    
    std::fs::remove_file(test_file).unwrap();
}

#[test]
fn test_config_api() {
    let all = config::all_features();
    assert!(!all.is_empty());
    
    let daemon_cfg = config::daemon_config_from_features(&all);
    // Based on daemon_config_from_features implementation, it should match feature presence
    assert!(daemon_cfg.preload);
    assert!(daemon_cfg.app_index);
    assert!(daemon_cfg.profile);
}

#[test]
fn test_features_api() {
    // Test tweak parsing with correct prefix
    let raw_cmd = "tweak write /sys/devices/system/cpu/cpu0/cpufreq/scaling_governor performance";
    let parsed_raw = features::parse_tweak_command_line(raw_cmd);
    assert!(parsed_raw.is_ok(), "Failed to parse raw command: {:?}", parsed_raw.err());
    
    if let Ok(features::TweakCommand::Write { path, value }) = parsed_raw {
        assert_eq!(path, "/sys/devices/system/cpu/cpu0/cpufreq/scaling_governor");
        assert_eq!(value, "performance");
    } else {
        panic!("Parsed command is not a Write command");
    }
}

#[test]
fn test_daemon_api() {
    let cfg = daemon::DaemonConfig {
        preload: true,
        usage: false,
        pressure: false,
        app_index: false,
        profile: false,
    };
    assert!(cfg.preload);
    assert!(!cfg.usage);
}

#[test]
fn test_resolver_api() {
    use coreshift_policy::api::resolver::{ForegroundResolver, ForegroundSnapshot};
    let _resolver = ForegroundResolver::new(vec!["com.android.launcher".to_string()]);
    // We can't easily test resolution without a real /proc/cpuset/top-app/tasks but we can test type existence
    let _snapshot = ForegroundSnapshot {
        pid: Some(1234),
        package: Some("com.test.app".to_string()),
        last_skip_reason: None,
    };
}
