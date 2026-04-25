// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/

use crate::core::{Event, Priority, RoutedAction};
use std::collections::VecDeque;

pub const MAX_ACTIONS_PER_TICK: usize = 2048;
pub const MAX_QUEUE: usize = 4096;
const MAX_PER_ACTION_KIND: usize = 1_000;

pub struct Scheduler {
    critical_queue: VecDeque<RoutedAction>,
    normal_queue: VecDeque<RoutedAction>,
    background_queue: VecDeque<RoutedAction>,
    pub total_len: usize,
    per_kind_counts: [usize; 40], // Exactly matches ActionKind count
    step_budget: usize,
    steps_executed: usize,
    rr_index: usize,
    rr_count: usize,
}

impl Scheduler {
    pub fn new(step_budget: usize) -> Self {
        Self {
            critical_queue: VecDeque::with_capacity(512),
            normal_queue: VecDeque::with_capacity(1024),
            background_queue: VecDeque::with_capacity(2048),
            total_len: 0,
            per_kind_counts: [0; 40],
            step_budget,
            steps_executed: 0,
            rr_index: 0,
            rr_count: 0,
        }
    }

    pub fn enqueue(
        &mut self,
        action: RoutedAction,
        state: &mut crate::core::ExecutionState,
    ) -> Option<Event> {
        let kind = action.action.kind();
        let kind_idx = kind as usize;

        if kind_idx >= self.per_kind_counts.len() {
            state.metrics.dropped_actions += 1;
            return Some(crate::core::Event::DroppedAction { kind });
        }

        let count = self.per_kind_counts[kind_idx];
        if count >= MAX_PER_ACTION_KIND {
            state.metrics.dropped_actions += 1;
            return Some(crate::core::Event::DroppedAction { kind });
        }

        let mut dropped_event = None;

        if self.total_len >= MAX_QUEUE {
            let action_prio = action.action.priority();
            let mut evicted = None;

            if !self.background_queue.is_empty() && action_prio >= Priority::Background {
                evicted = self.background_queue.pop_front();
            } else if !self.normal_queue.is_empty() && action_prio >= Priority::Normal {
                evicted = self.normal_queue.pop_front();
            } else if !self.critical_queue.is_empty() && action_prio >= Priority::Critical {
                evicted = self.critical_queue.pop_front();
            }

            if let Some(ev) = evicted {
                let ev_kind = ev.action.kind();
                self.per_kind_counts[ev_kind as usize] -= 1;
                self.total_len -= 1;
                state.metrics.dropped_actions += 1;
                dropped_event = Some(crate::core::Event::DroppedAction { kind: ev_kind });
            } else {
                state.metrics.dropped_actions += 1;
                return Some(crate::core::Event::DroppedAction { kind });
            }
        }

        self.per_kind_counts[kind_idx] += 1;
        match action.action.priority() {
            Priority::Critical => self.critical_queue.push_back(action),
            Priority::Normal => self.normal_queue.push_back(action),
            Priority::Background => self.background_queue.push_back(action),
        }
        self.total_len += 1;

        dropped_event
    }

    pub fn pop_next(&mut self) -> Option<RoutedAction> {
        if self.steps_executed >= self.step_budget {
            return None;
        }

        let schedule = [
            (Priority::Critical, 4),
            (Priority::Normal, 2),
            (Priority::Background, 1),
        ];

        let start_index = self.rr_index;
        let mut checked_all = false;

        loop {
            let (prio, quota) = schedule[self.rr_index];

            if self.rr_count < quota {
                let routed_opt = match prio {
                    Priority::Critical => self.critical_queue.pop_front(),
                    Priority::Normal => self.normal_queue.pop_front(),
                    Priority::Background => self.background_queue.pop_front(),
                };

                if let Some(routed) = routed_opt {
                    self.rr_count += 1;
                    self.total_len -= 1;
                    self.steps_executed += 1;
                    let kind = routed.action.kind();
                    self.per_kind_counts[kind as usize] -= 1;
                    return Some(routed);
                }
            }

            self.rr_index = (self.rr_index + 1) % schedule.len();
            self.rr_count = 0;

            if self.rr_index == start_index {
                if checked_all {
                    return None;
                }
                checked_all = true;
            }
        }
    }

    pub fn is_exhausted(&self) -> bool {
        self.steps_executed >= self.step_budget || self.total_len == 0
    }
}
