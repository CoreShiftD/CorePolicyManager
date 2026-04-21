use crate::core::{Action, Module, TimeoutEntry};
use std::collections::BTreeMap;

pub struct TimeoutStateStore {
    pub timeouts: BTreeMap<u64, TimeoutEntry>,
    pub hash: u64,
}

impl Default for TimeoutStateStore {
    fn default() -> Self {
        Self::new()
    }
}

impl TimeoutStateStore {
    pub fn new() -> Self {
        Self {
            timeouts: BTreeMap::new(),
            hash: 0,
        }
    }
}

pub struct TimeoutPolicyModule;

impl Default for TimeoutPolicyModule {
    fn default() -> Self {
        Self::new()
    }
}

impl TimeoutPolicyModule {
    pub fn new() -> Self {
        Self {}
    }
}

impl Module for TimeoutPolicyModule {
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
        event: &crate::core::Event,
    ) -> Vec<Action> {
        let mut actions = Vec::new();
        if let crate::core::Event::Tick = event {
            let now = state.now();

            // Phase 1: Collect expired items deterministically
            // BTreeMap guarantees deterministic order
            for entry in state.timeouts() {
                match entry.state {
                    crate::core::TimeoutState::WaitingForDeadline => {
                        if now >= entry.deadline {
                            actions.push(Action::UpdateTimeoutState {
                                id: entry.id,
                                state: crate::core::TimeoutState::WaitingForKillGrace(
                                    now + (entry.kill_grace_ms as u64),
                                ),
                            });
                            actions.push(Action::TimeoutReached { id: entry.id });
                        }
                    }
                    crate::core::TimeoutState::WaitingForKillGrace(grace_deadline) => {
                        if now >= grace_deadline {
                            actions.push(Action::KillDeadlineReached { id: entry.id });
                            actions.push(Action::UntrackTimeout { id: entry.id });
                        }
                    }
                }
            }
        }

        actions
    }
}

#[derive(Default)]
pub struct AdmissionControlModule;

impl Module for AdmissionControlModule {
    fn handle(
        &self,
        state: &dyn crate::core::state_view::StateView,
        action: &Action,
    ) -> Vec<Action> {
        let mut actions = Vec::new();
        if let Action::Submit { id, owner, .. } = action {
            if state.active_jobs() >= state.max_jobs() {
                actions.push(Action::Rejected {
                    id: *id,
                    owner: *owner,
                    was_submitted: false,
                });
            } else {
                actions.push(Action::Admitted { id: *id });
            }
        }
        actions
    }

    fn handle_event(
        &self,
        _state: &dyn crate::core::state_view::StateView,
        _event: &crate::core::Event,
    ) -> Vec<Action> {
        Vec::new()
    }
}
