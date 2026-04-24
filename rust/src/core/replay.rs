// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/

use crate::core::{Event, Intent};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone)]
pub struct TickStats {
    pub hash: u64,
    pub actions_processed: usize,
    pub dropped_actions: usize,
}

#[derive(Serialize, Deserialize, Clone)]
pub enum ReplayInput {
    Event(Event),
    LegacyIntent(Intent), // Legacy support for intent without principal
    Intent(crate::high_level::identity::Principal, Intent),
    TickHash(u64), // Legacy
    TickEnd(TickStats),
    Time(std::time::Duration), // We need to record time because if we don't, time-dependent logic (timeouts, etc) will diverge during replay.
}

thread_local! {
    pub static EPOCH: std::time::Instant = std::time::Instant::now();
}
pub fn hash_str(s: &str) -> u64 {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut h = DefaultHasher::new();
    s.hash(&mut h);
    h.finish()
}
