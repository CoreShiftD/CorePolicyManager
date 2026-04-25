// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/

//! `runtime` owns side effects, logging, process execution, and system services.

mod control;
mod effects;
mod inotify;
mod logging;
pub mod status;
mod system_services;

pub use effects::{EffectExecutor, RuntimeDrain, RuntimeProcess};
pub use inotify::{
    CGROUP_PROCS_PATH, PACKAGES_LIST_PATH, PACKAGES_XML_PATH, PreloadInotify, PreloadInotifyEvent,
    parse_top_app_pid,
};
pub use logging::{LogRouter, log_runtime_event};
pub use status::assemble_daemon_status;
