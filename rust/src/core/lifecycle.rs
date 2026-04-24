// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/

use crate::core::{Action, Event, JobLifecycle, Module};
use smallvec::SmallVec;

pub struct LifecycleModule;

impl Module for LifecycleModule {
    fn handle(
        &self,
        state: &dyn crate::core::state_view::StateView,
        action: &Action,
    ) -> crate::core::ActionList {
        let mut actions = SmallVec::new();
        match action {
            Action::Admitted { id } => {
                actions.push(Action::SetLifecycle {
                    id: *id,
                    state: JobLifecycle::Admitted,
                });
                actions.push(Action::StartProcess { id: *id });
            }
            Action::SignalProcess { process, signal } => {
                if let Some(job) = state.job_by_process(*process) {
                    if *signal == crate::core::ControlSignal::GracefulStop {
                        actions.push(Action::TimeoutReached { id: job.id });
                        actions.push(Action::SetLifecycle {
                            id: job.id,
                            state: JobLifecycle::Terminating,
                        });
                    } else if *signal == crate::core::ControlSignal::ForceKill {
                        actions.push(Action::SetLifecycle {
                            id: job.id,
                            state: JobLifecycle::Killed,
                        });
                    }
                }
            }
            Action::SetJobIoState {
                id,
                state: io_state,
            } => {
                if *io_state == crate::core::JobIoState::Closed
                    && let Some(job) = state.job(*id)
                    && let Some(process) = job.process
                {
                    actions.push(Action::PollProcess { process });
                }
            }
            Action::TimeoutReached { id } => {
                if let Some(job) = state.job(*id)
                    && job.lifecycle == JobLifecycle::Running
                {
                    actions.push(Action::SetLifecycle {
                        id: *id,
                        state: JobLifecycle::Terminating,
                    });
                    if let Some(process) = job.process {
                        actions.push(Action::SignalProcess {
                            process,
                            signal: crate::core::ControlSignal::GracefulStop,
                        });
                    }
                }
            }
            Action::KillDeadlineReached { id } => {
                if let Some(job) = state.job(*id)
                    && job.lifecycle != JobLifecycle::Finished
                    && job.lifecycle != JobLifecycle::Killed
                {
                    actions.push(Action::SetLifecycle {
                        id: *id,
                        state: JobLifecycle::Killed,
                    });
                    if let Some(process) = job.process {
                        actions.push(Action::SignalProcess {
                            process,
                            signal: crate::core::ControlSignal::ForceKill,
                        });
                    }
                }
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
        match event {
            Event::ProcessStarted {
                id,
                process: process_handle,
                io: io_handle,
            } => {
                actions.push(Action::AssignProcess {
                    id: *id,
                    process: *process_handle,
                });
                actions.push(Action::AssignIo {
                    id: *id,
                    io: *io_handle,
                });
                actions.push(Action::SetLifecycle {
                    id: *id,
                    state: JobLifecycle::Running,
                });
                actions.push(Action::Started { id: *id });
            }
            Event::ProcessSpawnFailed { id, err: _ } => {
                let job = state.job(*id);
                let owner = job.as_ref().map(|j| j.owner).unwrap_or(0);
                let was_submitted = job
                    .as_ref()
                    .map(|j| j.lifecycle == crate::core::JobLifecycle::Submitted)
                    .unwrap_or(false);
                actions.push(Action::SetLifecycle {
                    id: *id,
                    state: JobLifecycle::Finished,
                });
                actions.push(Action::Rejected {
                    id: *id,
                    owner,
                    was_submitted,
                });
            }
            _ => {}
        }
        actions
    }
}
