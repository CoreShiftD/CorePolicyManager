// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/

//! `low_level` is the stable, trusted substrate for the CorePolicyManager.
//!
//! ### Substrate Contract
//! - **OS Boundary**: This module owns all direct interactions with the Linux
//!   kernel, `libc`, and `procfs`.
//! - **Policy Neutral**: No daemon-level policies, preload logic, or business
//!   rules should exist here.
//! - **Mechanisms, Not Policy**: Provide the building blocks (spawn, reactor,
//!   inotify) that higher layers use to implement policy.
//! - **Stability**: Prefer additive fixes over breaking rewrites. Higher
//!   layers (runtime, core, addons) depend on this API.

pub mod inotify;
pub mod io;
pub mod reactor;
pub mod spawn;
pub mod sys;

#[cfg(test)]
mod tests;
