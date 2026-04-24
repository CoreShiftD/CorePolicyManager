// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/

pub struct Dispatcher {
    pub modules: Vec<Box<dyn crate::core::Module>>,
}

impl Default for Dispatcher {
    fn default() -> Self {
        Self::new()
    }
}

impl Dispatcher {
    pub fn new() -> Self {
        Self {
            modules: vec![
                Box::new(crate::core::policy::AdmissionControlModule),
                Box::new(crate::core::lifecycle::LifecycleModule),
                Box::new(crate::core::process::ProcessModule),
                Box::new(crate::core::io::IoModule),
                Box::new(crate::core::result::ResultModule),
                Box::new(crate::core::policy::TimeoutPolicyModule::new()),
            ],
        }
    }

    pub fn dispatch(
        &self,
        state: &dyn crate::core::state_view::StateView,
        action: &crate::core::Action,
    ) -> Vec<crate::core::Action> {
        let mut actions = Vec::new();
        for module in &self.modules {
            actions.extend(module.handle(state, action));
        }
        actions
    }

    pub fn dispatch_event(
        &self,
        state: &dyn crate::core::state_view::StateView,
        event: &crate::core::Event,
    ) -> Vec<crate::core::Action> {
        let mut actions = Vec::new();
        for module in &self.modules {
            actions.extend(module.handle_event(state, event));
        }
        actions
    }

    pub fn compute_timeout_ms(&self, state: &dyn crate::core::state_view::StateView) -> i32 {
        let mut min_ms: i32 = -1;
        let now = state.now();
        for entry in state.timeouts() {
            let deadline = match entry.state {
                crate::core::TimeoutState::WaitingForDeadline => entry.deadline,
                crate::core::TimeoutState::WaitingForKillGrace(d) => d,
            };

            let ms = if deadline > now {
                (deadline - now) as i32
            } else {
                0
            };

            if min_ms == -1 || ms < min_ms {
                min_ms = ms;
            }
        }
        min_ms
    }
}

pub struct Core {
    pub dispatcher: Dispatcher,
    pub reducers: Vec<Box<dyn crate::core::reducer::Reducer>>,
    pub routing: std::collections::HashMap<crate::core::ActionKind, Vec<usize>>,
}

impl Default for Core {
    fn default() -> Self {
        Self::new()
    }
}

impl Core {
    pub fn new() -> Self {
        let reducers: Vec<Box<dyn crate::core::reducer::Reducer>> = vec![
            Box::new(crate::core::reducer::TimeReducer),
            Box::new(crate::core::reducer::ResultReducer),
            Box::new(crate::core::reducer::IoReducer),
            Box::new(crate::core::reducer::JobReducer),
            Box::new(crate::core::reducer::TimeoutReducer),
            Box::new(crate::core::reducer::LogReducer),
            Box::new(crate::core::reducer::AddonReducer),
        ];

        for reducer in reducers.iter() {
            assert!(
                !reducer.handles().is_empty(),
                "Reducer must handle at least one action"
            );
        }

        let mut routing: std::collections::HashMap<crate::core::ActionKind, Vec<usize>> =
            std::collections::HashMap::new();
        for (idx, reducer) in reducers.iter().enumerate() {
            for kind in reducer.handles() {
                routing.entry(*kind).or_default().push(idx);
            }
        }

        Self {
            dispatcher: Dispatcher::new(),
            reducers,
            routing,
        }
    }
}
