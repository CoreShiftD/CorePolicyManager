// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/

use crate::core::{Action, Event, IoStream, Module};

pub struct IoModule;

impl Module for IoModule {
    fn handle(
        &self,
        _state: &dyn crate::core::state_view::StateView,
        _action: &Action,
    ) -> Vec<Action> {
        Vec::new()
    }

    fn handle_event(
        &self,
        state: &dyn crate::core::state_view::StateView,
        event: &Event,
    ) -> Vec<Action> {
        let mut actions = Vec::new();
        match event {
            Event::ProcessStarted { io, .. } => {
                // Here the core module issues Watch actions purely via intent.
                // It doesn't know about `DrainState` or slots anymore.
                // We request watching for all default streams.
                // EffectExecutor will decide if the stream actually exists or needs a watch based on its internal DrainState.
                actions.push(Action::RegisterInterest {
                    io: *io,
                    stream: IoStream::Stdout,
                });
                actions.push(Action::RegisterInterest {
                    io: *io,
                    stream: IoStream::Stderr,
                });
                actions.push(Action::RegisterInterest {
                    io: *io,
                    stream: IoStream::Stdin,
                });
            }
            Event::IoReady { io, .. } => {
                if let Some(job) = state.job_by_io(*io) {
                    actions.push(Action::SetJobIoState {
                        id: job.id,
                        state: crate::core::JobIoState::Active,
                    });
                    actions.push(Action::PerformIo { io: *io });
                }
            }
            Event::IoClosed { io } => {
                if let Some(job) = state.job_by_io(*io) {
                    actions.push(Action::SetJobIoState {
                        id: job.id,
                        state: crate::core::JobIoState::Closed,
                    });
                }
            }
            Event::WatchStreamFailed { io, err } => {
                actions.push(Action::HandleIoFailure {
                    io: *io,
                    reason: err.clone(),
                });
                if let Some(job) = state.job_by_io(*io) {
                    actions.push(Action::CleanupJob { id: job.id });
                }
            }
            Event::IoFailed { io, reason } => {
                actions.push(Action::HandleIoFailure {
                    io: *io,
                    reason: reason.clone(),
                });
                if let Some(job) = state.job_by_io(*io) {
                    actions.push(Action::CleanupJob { id: job.id });
                }
            }
            _ => {}
        }
        actions
    }
}
