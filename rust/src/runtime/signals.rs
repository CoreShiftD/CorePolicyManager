use coreshift_lowlevel::sys::{SignalRuntime, install_shutdown_flag};
use std::sync::atomic::AtomicBool;

pub static SHUTDOWN: AtomicBool = AtomicBool::new(false);

pub fn setup() {
    // Unblock all signals
    let _ = SignalRuntime::unblock_all();
    let _ = install_shutdown_flag(&SHUTDOWN);
}
