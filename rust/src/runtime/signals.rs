use coreshift_lowlevel::sys::install_shutdown_flag;
use std::sync::atomic::AtomicBool;

pub static SHUTDOWN: AtomicBool = AtomicBool::new(false);

pub fn setup() {
    let _ = install_shutdown_flag(&SHUTDOWN);
}
