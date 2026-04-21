use crate::core::{IoHandle, JobIoState, JobLifecycle, ProcessHandle};

pub struct JobView {
    pub id: u64,
    pub owner: u32,
    pub lifecycle: JobLifecycle,
    pub io_state: JobIoState,
    pub process: Option<ProcessHandle>,
    pub io: Option<IoHandle>,
    pub timed_out: bool,
}

pub struct TimeoutView {
    pub id: u64,
    pub state: crate::core::TimeoutState,
    pub deadline: u64,
    pub kill_grace_ms: u32,
}

pub struct ResultView {
    pub result: Result<crate::core::ExecResult, crate::core::ExecError>,
    pub owner: u32,
}

pub trait StateView {
    fn job(&self, id: u64) -> Option<JobView>;
    fn job_by_process(&self, process: ProcessHandle) -> Option<JobView>;
    fn job_by_io(&self, io: IoHandle) -> Option<JobView>;

    fn result(&self, id: u64) -> Option<ResultView>;

    fn active_jobs(&self) -> usize;
    fn max_jobs(&self) -> usize;

    fn is_warmup_in_flight(&self, package: &str) -> bool;
    fn warmup_last_run(&self, package: &str) -> Option<u64>;
    fn warmup_negative_cached(&self, package: &str) -> Option<u64>;
    fn warmup_base_dir(&self, package: &str) -> Option<String>;

    fn timeouts(&self) -> Vec<TimeoutView>;

    fn now(&self) -> u64;
}
