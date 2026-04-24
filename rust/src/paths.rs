pub const BASE_DIR: &str = "/data/local/tmp/coreshift";
pub const CONTROL_DIR: &str = "/data/local/tmp/coreshift/control";
pub const ADDONS_DIR: &str = "/data/local/tmp/coreshift/addons";

pub const SOCKET_PATH: &str = "/data/local/tmp/coreshift/coreshift.sock";
pub const CORE_LOG_PATH: &str = "/data/local/tmp/coreshift/core.log";

pub const ENABLE_PRELOAD_PATH: &str = "/data/local/tmp/coreshift/control/enable_preload";
pub const LOG_DEBUG_PATH: &str = "/data/local/tmp/coreshift/control/log_debug";
pub const LOG_TRACE_PATH: &str = "/data/local/tmp/coreshift/control/log_trace";

pub fn ensure_dirs() -> std::io::Result<()> {
    std::fs::create_dir_all(BASE_DIR)?;
    std::fs::create_dir_all(CONTROL_DIR)?;
    std::fs::create_dir_all(ADDONS_DIR)?;
    Ok(())
}

pub fn addon_log_path(addon_id: u32) -> String {
    format!("{}/addon_{}.log", ADDONS_DIR, addon_id)
}
