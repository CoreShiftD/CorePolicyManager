use crate::core::ExecutionState;

pub fn verify_global(state: &ExecutionState) {
    let core = &state.core;
    let _warmup = &state.warmup;
    let _timeout = &state.timeout;
    let _result = &state.result;

    // Verify jobs and runtime
    let mut actual_process_count = 0;
    let mut actual_io_count = 0;

    // We can't iterate Arena without implementing iter, but let's check id_map instead
    for (u64_id, handle) in &core.job_id_map {
        let job = core.jobs.get(handle.index, handle.generation).expect("dangling job_id_map entry");
        assert_eq!(*u64_id, job.id);

        let rt = core.runtime.get(handle.index as usize)
            .expect("missing runtime vector entry")
            .as_ref()
            .expect("job missing runtime mapping");

        assert!(job.process == rt.process, "job process handle mismatch with runtime");
        assert!(job.io == rt.io, "job io handle mismatch with runtime");

        if let Some(p) = rt.process {
            actual_process_count += 1;
            let p_handle = core.process_index.get(p.index as usize)
                .expect("missing process vector entry")
                .as_ref()
                .expect("process index dangling");
            assert_eq!(*p_handle, *handle, "process index mismatch");
        }

        if let Some(io) = rt.io {
            actual_io_count += 1;
            let io_handle = core.io_index.get(io.index as usize)
                .expect("missing io vector entry")
                .as_ref()
                .expect("io index dangling");
            assert_eq!(*io_handle, *handle, "io index mismatch");
        }
    }

    assert_eq!(core.process_count, actual_process_count, "process count drift");
    assert_eq!(core.io_count, actual_io_count, "io count drift");
}
