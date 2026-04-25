// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/

//! # Low Level Infrastructure
//!
//! This module and its submodules (inotify, io, reactor, spawn, sys) form a
//! **stable platform layer** for the CoreShift daemon.
//!
//! ## Mandate
//! - `low_level` is a stable substrate.
//! - Prefer changes in `runtime` or `high_level` layers first.
//! - Only modify `low_level` for:
//!   - Correctness bugs.
//!   - Android compatibility issues.
//!   - Safety issues (memory safety, undefined behavior).
//!   - Measurable performance bottlenecks.
//!   - Missing OS primitives strictly required by higher layers.
//!
//! ## Architecture
//! - Dependency direction: `runtime` -> `low_level`.
//! - Avoid circular dependencies.
//! - Wrap awkward APIs in `runtime` adapters instead of editing internals.

pub mod inotify;
pub mod io;
pub mod reactor;
pub mod spawn;
pub mod sys;
