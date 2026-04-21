use crate::core::{Action, Event, Module};

pub struct ProcessModule;

impl Module for ProcessModule {
    fn handle(
        &self,
        state: &dyn crate::core::state_view::StateView,
        action: &Action,
    ) -> Vec<Action> {
        let mut actions = Vec::new();
        match action {
            Action::Control { id, signal } => {
                if let Some(job) = state.job(*id)
                    && let Some(process) = job.process
                {
                    actions.push(Action::SignalProcess {
                        process,
                        signal: *signal,
                    });
                    actions.push(Action::Controlled { id: *id });
                }
            }
            Action::TimeoutReached { id } => {
                if let Some(job) = state.job(*id)
                    && let Some(process) = job.process
                {
                    actions.push(Action::SignalProcess {
                        process,
                        signal: crate::core::ControlSignal::GracefulStop,
                    });
                }
            }
            Action::KillDeadlineReached { id } => {
                if let Some(job) = state.job(*id)
                    && let Some(process) = job.process
                {
                    actions.push(Action::SignalProcess {
                        process,
                        signal: crate::core::ControlSignal::ForceKill,
                    });
                }
            }
            // `StartProcess`, `SignalProcess`, and `PollProcess` are mapped directly to `Effect`s in `resolve_effects`.
            // We don't generate additional intent from them here.
            _ => {}
        }
        actions
    }

    fn handle_event(
        &self,
        state: &dyn crate::core::state_view::StateView,
        event: &Event,
    ) -> Vec<Action> {
        let mut actions = Vec::new();
        match event {
            Event::ProcessSpawnFailed { id, err } => {
                let job = state.job(*id);
                let owner = job.as_ref().map(|j| j.owner).unwrap_or(0);
                let was_submitted = job
                    .as_ref()
                    .map(|j| j.lifecycle == crate::core::JobLifecycle::Submitted)
                    .unwrap_or(false);
                actions.push(Action::Finished {
                    id: *id,
                    owner,
                    was_submitted,
                    result: Err(crate::core::ExecError::SpawnFailed),
                });
                actions.push(Action::EmitLog {
                    owner: crate::core::CORE_OWNER,
                    level: crate::core::LogLevel::Error,
                    event: crate::core::LogEvent::Error {
                        id: *id,
                        err: err.clone(),
                    },
                });
            }
            Event::KillProcessFailed { process, err } => {
                actions.push(Action::HandleProcessFailure {
                    process: *process,
                    err: err.clone(),
                });
                if let Some(job) = state.job_by_process(*process) {
                    actions.push(Action::CleanupJob { id: job.id });
                }
            }
            Event::TimeAdvanced(delta) => {
                actions.push(Action::AdvanceTime { delta: *delta });
            }
            Event::ReactorError { err } => {
                actions.push(Action::EmitLog {
                    owner: crate::core::CORE_OWNER,
                    level: crate::core::LogLevel::Error,
                    event: crate::core::LogEvent::Error {
                        id: 0,
                        err: err.clone(),
                    },
                });
            }
            _ => {}
        }
        actions
    }
}
