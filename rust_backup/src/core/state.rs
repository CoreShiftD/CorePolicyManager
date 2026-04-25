// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JobLifecycle {
    Submitted,
    Admitted,
    Running,
    Terminating,
    Killed,
    Finished,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JobIoState {
    Pending,
    Active,
    Closed,
}

#[derive(Clone)]
pub struct JobState {
    pub id: u64,
    pub owner: u32,
    pub exec: crate::core::ExecSpec,
    pub policy: crate::core::ExecPolicy,
    pub process: Option<crate::core::ProcessHandle>,
    pub io: Option<crate::core::IoHandle>,
    pub timed_out: bool,
    pub lifecycle: JobLifecycle,
    pub io_state: JobIoState,
}

#[derive(Clone)]
pub struct StoredResult {
    pub result: Result<crate::core::ExecResult, crate::core::ExecError>,
    pub owner: u32,
    pub created: u64,
}

#[derive(Clone)]
pub struct JobRuntime {
    pub process: Option<crate::core::ProcessHandle>,
    pub io: Option<crate::core::IoHandle>,
}

#[derive(Clone)]
pub struct TimeoutEntry {
    pub id: u64,
    pub state: TimeoutState,
    pub deadline: u64,
    pub kill_grace_ms: u32,
}

#[derive(Clone, PartialEq, Eq)]
pub enum TimeoutState {
    WaitingForDeadline,
    WaitingForKillGrace(u64),
}

#[derive(Default, Clone, Serialize, Deserialize, Debug)]
pub struct Metrics {
    pub active_clients: u32,
    pub dropped_actions: u64,
    pub queue_depth: u32,
    pub avg_tick_duration_us: u32,
    pub peak_read_buf_kb: u32,
    pub peak_write_buf_kb: u32,
    pub restart_count: u32,
}

pub struct ExecutionState {
    pub core: crate::core::core_state::CoreState,
    pub timeout: crate::core::policy::TimeoutStateStore,
    pub result: crate::core::result::ResultState,
    pub metrics: Metrics,
    pub clock: u64,
    pub hash: u64,
}

impl Default for ExecutionState {
    fn default() -> Self {
        Self::new()
    }
}

impl ExecutionState {
    pub fn new() -> Self {
        Self {
            core: crate::core::core_state::CoreState::new(),
            timeout: crate::core::policy::TimeoutStateStore::new(),
            result: crate::core::result::ResultState::new(),
            metrics: Metrics::default(),
            clock: 0,
            hash: 0,
        }
    }

    pub fn update_hash(&mut self) {
        self.hash = self.core.hash ^ self.timeout.hash ^ self.result.hash;
    }
}

impl crate::core::state_view::StateView for ExecutionState {
    fn job(&self, id: u64) -> Option<crate::core::state_view::JobView> {
        let h = self.core.job_handle(id)?;
        let j = self.core.job(h)?;
        Some(crate::core::state_view::JobView {
            id: j.id,
            owner: j.owner,
            lifecycle: j.lifecycle,
            io_state: j.io_state,
            process: j.process,
            io: j.io,
            timed_out: j.timed_out,
        })
    }

    fn job_by_process(
        &self,
        process: crate::core::ProcessHandle,
    ) -> Option<crate::core::state_view::JobView> {
        let h = self.core.job_by_process(process)?;
        let j = self.core.job(h)?;
        Some(crate::core::state_view::JobView {
            id: j.id,
            owner: j.owner,
            lifecycle: j.lifecycle,
            io_state: j.io_state,
            process: j.process,
            io: j.io,
            timed_out: j.timed_out,
        })
    }

    fn job_by_io(&self, io: crate::core::IoHandle) -> Option<crate::core::state_view::JobView> {
        let h = self.core.job_by_io(io)?;
        let j = self.core.job(h)?;
        Some(crate::core::state_view::JobView {
            id: j.id,
            owner: j.owner,
            lifecycle: j.lifecycle,
            io_state: j.io_state,
            process: j.process,
            io: j.io,
            timed_out: j.timed_out,
        })
    }

    fn result(&self, id: u64) -> Option<crate::core::state_view::ResultView> {
        self.result
            .results
            .get(&id)
            .map(|r| crate::core::state_view::ResultView {
                result: r.result.clone(),
                owner: r.owner,
            })
    }

    fn active_jobs(&self) -> usize {
        self.result.active_jobs
    }

    fn max_jobs(&self) -> usize {
        self.result.max_jobs
    }

    fn timeouts(&self) -> Vec<crate::core::state_view::TimeoutView> {
        self.timeout
            .timeouts
            .values()
            .map(|t| crate::core::state_view::TimeoutView {
                id: t.id,
                state: t.state.clone(),
                deadline: t.deadline,
                kill_grace_ms: t.kill_grace_ms,
            })
            .collect()
    }

    fn now(&self) -> u64 {
        self.clock
    }
}
