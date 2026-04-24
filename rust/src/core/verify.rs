// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/

use crate::core::ExecutionState;

/// Verify cross-index invariants without panicking the daemon.
///
/// The runtime uses this as a periodic drift detector. Failures should surface
/// as explicit errors so long-lived sessions can log and continue rather than
/// crashing on invariant-check infrastructure.
pub fn verify_global(state: &ExecutionState) -> Result<(), String> {
    let core = &state.core;
    let _timeout = &state.timeout;
    let _result = &state.result;

    // Verify jobs and runtime
    let mut actual_process_count = 0;
    let mut actual_io_count = 0;

    // We can't iterate Arena without implementing iter, but let's check id_map instead
    for (u64_id, handle) in &core.job_id_map {
        let job = core
            .jobs
            .get(handle.index, handle.generation)
            .ok_or_else(|| format!("dangling job_id_map entry id={}", u64_id))?;
        if *u64_id != job.id {
            return Err(format!(
                "job id mismatch map_id={} stored_id={}",
                u64_id, job.id
            ));
        }

        let rt = core
            .runtime
            .get(handle.index as usize)
            .ok_or_else(|| format!("missing runtime vector entry index={}", handle.index))?
            .as_ref()
            .ok_or_else(|| format!("job missing runtime mapping index={}", handle.index))?;

        if job.process != rt.process {
            return Err(format!(
                "job process handle mismatch id={} job={:?} runtime={:?}",
                job.id, job.process, rt.process
            ));
        }
        if job.io != rt.io {
            return Err(format!(
                "job io handle mismatch id={} job={:?} runtime={:?}",
                job.id, job.io, rt.io
            ));
        }

        if let Some(p) = rt.process {
            actual_process_count += 1;
            let p_handle = core
                .process_index
                .get(p.index as usize)
                .ok_or_else(|| format!("missing process vector entry index={}", p.index))?
                .as_ref()
                .ok_or_else(|| format!("process index dangling index={}", p.index))?;
            if *p_handle != *handle {
                return Err(format!(
                    "process index mismatch process_index={} expected_job_index={} actual_job_index={}",
                    p.index, handle.index, p_handle.index
                ));
            }
        }

        if let Some(io) = rt.io {
            actual_io_count += 1;
            let io_handle = core
                .io_index
                .get(io.index as usize)
                .ok_or_else(|| format!("missing io vector entry index={}", io.index))?
                .as_ref()
                .ok_or_else(|| format!("io index dangling index={}", io.index))?;
            if *io_handle != *handle {
                return Err(format!(
                    "io index mismatch io_index={} expected_job_index={} actual_job_index={}",
                    io.index, handle.index, io_handle.index
                ));
            }
        }
    }

    if core.process_count != actual_process_count {
        return Err(format!(
            "process count drift expected={} actual={}",
            core.process_count, actual_process_count
        ));
    }
    if core.io_count != actual_io_count {
        return Err(format!(
            "io count drift expected={} actual={}",
            core.io_count, actual_io_count
        ));
    }

    Ok(())
}
