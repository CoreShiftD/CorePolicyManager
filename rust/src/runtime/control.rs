// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/

use crate::core::ControlSignal;
use crate::low_level::spawn::{ExitStatus, Process, SysError};

pub(super) fn apply_control_signal(
    process: &mut Process,
    is_group: bool,
    signal: ControlSignal,
) -> Result<(), SysError> {
    match signal {
        ControlSignal::GracefulStop => {
            if is_group {
                process.kill_pgroup(libc::SIGTERM)
            } else {
                process.kill(libc::SIGTERM)
            }
        }
        ControlSignal::ForceKill => {
            if is_group {
                process.kill_pgroup(libc::SIGKILL)
            } else {
                process.kill(libc::SIGKILL)
            }
        }
    }
}

pub(super) fn exit_status_code(status: ExitStatus) -> i32 {
    match status {
        ExitStatus::Exited(code) => code,
        ExitStatus::Signaled(signal) => -signal,
    }
}
