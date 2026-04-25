// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/

use crate::core::{Action, Event, Module, StoredResult};
use smallvec::SmallVec;
use std::collections::{BTreeMap, VecDeque};

pub struct ResultState {
    pub results: BTreeMap<u64, StoredResult>,
    pub result_order: VecDeque<u64>,
    pub active_jobs: usize,
    pub max_jobs: usize,
    pub hash: u64,
}

impl Default for ResultState {
    fn default() -> Self {
        Self::new()
    }
}

impl ResultState {
    pub fn new() -> Self {
        Self {
            results: BTreeMap::new(),
            result_order: VecDeque::new(),
            active_jobs: 0,
            max_jobs: 64,
            hash: 0,
        }
    }
}

pub struct ResultModule;

impl Module for ResultModule {
    fn handle(
        &self,
        state: &dyn crate::core::state_view::StateView,
        action: &Action,
    ) -> crate::core::ActionList {
        let mut actions = SmallVec::new();
        match action {
            Action::Query { id } => {
                if let Some(r) = state.result(*id) {
                    let outcome = crate::core::ExecOutcome {
                        id: *id,
                        result: r.result.clone(),
                    };
                    actions.push(Action::QueryResult {
                        id: *id,
                        result: Some(outcome),
                    });
                } else {
                    actions.push(Action::QueryResult {
                        id: *id,
                        result: None,
                    });
                }
            }
            Action::Finished {
                id,
                result: _,
                owner: _,
                was_submitted: _,
            } => {
                actions.push(Action::CleanupJob { id: *id });
            }
            Action::Rejected {
                id,
                owner: _,
                was_submitted: _,
            } => {
                actions.push(Action::CleanupJob { id: *id });
            }
            _ => {}
        }
        actions
    }

    fn handle_event(
        &self,
        state: &dyn crate::core::state_view::StateView,
        event: &Event,
    ) -> crate::core::ActionList {
        let mut actions = SmallVec::new();
        if let Event::ProcessExited { process, status } = event
            && let Some(job) = state.job_by_process(*process)
        {
            // For now, assume process exits without explicit buffered IO completion.
            // A proper design will receive IoDataReceived or the drain contents from runtime.
            // We simply synthesize empty stdout/stderr if the drain parts aren't forwarded.
            let result = crate::core::ExecResult {
                status: *status,
                stdout: Vec::new(),
                stderr: Vec::new(),
                timed_out: job.timed_out,
            };
            let was_submitted = job.lifecycle == crate::core::JobLifecycle::Submitted;
            actions.push(Action::Finished {
                id: job.id,
                owner: job.owner,
                was_submitted,
                result: Ok(result),
            });
        }
        actions
    }
}
