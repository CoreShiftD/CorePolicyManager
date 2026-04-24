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
    per_kind_counts: [usize; 43], // Based on ActionKind count
    step_budget: usize,
    steps_executed: usize,
    rr_index: usize,
    rr_count: usize,
}

impl Scheduler {
    pub fn new(step_budget: usize) -> Self {
        Self {
            critical_queue: VecDeque::with_capacity(MAX_QUEUE),
            normal_queue: VecDeque::with_capacity(MAX_QUEUE),
            background_queue: VecDeque::with_capacity(MAX_QUEUE),
            total_len: 0,
            per_kind_counts: [0; 43],
            step_budget,
            steps_executed: 0,
            rr_index: 0,
            rr_count: 0,
        }
    }

    pub fn enqueue(&mut self, action: RoutedAction) -> Option<Event> {
        let kind = action.action.kind();
        let kind_idx = kind as usize;
        let count = self.per_kind_counts[kind_idx];
        if count >= MAX_PER_ACTION_KIND {
            return Some(crate::core::Event::DroppedAction { kind });
        }

        let mut dropped_event = None;

        if self.total_len >= MAX_QUEUE {
            // Drop lowest priority, oldest (FIFO drop -> pop_front)
            let mut evicted = None;
            let action_prio = action.action.priority();

            if !self.background_queue.is_empty() {
                if action_prio <= Priority::Background {
                    evicted = self.background_queue.pop_front();
                }
            } else if !self.normal_queue.is_empty() {
                if action_prio <= Priority::Normal {
                    evicted = self.normal_queue.pop_front();
                }
            } else if !self.critical_queue.is_empty() {
                if action_prio <= Priority::Critical {
                    evicted = self.critical_queue.pop_front();
                }
            }

            if let Some(ev) = evicted {
                let ev_kind = ev.action.kind();
                self.per_kind_counts[ev_kind as usize] -= 1;
                self.total_len -= 1;
                dropped_event = Some(crate::core::Event::DroppedAction { kind: ev_kind });
            } else {
                // Cannot evict, so drop the incoming action
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

    pub fn next(&mut self) -> Option<RoutedAction> {
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
                } else {
                    self.rr_index = (self.rr_index + 1) % schedule.len();
                    self.rr_count = 0;
                }
            } else {
                self.rr_index = (self.rr_index + 1) % schedule.len();
                self.rr_count = 0;
            }

            if self.rr_index == start_index && self.rr_count == 0 {
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
