// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/

//! `core` is the pure state machine layer: events, actions, reducers, and state.

pub mod engine;
pub mod policy;
pub mod state;
pub mod types;
pub mod validation;

pub mod core_state;
pub mod io;
pub mod lifecycle;
pub mod process;
pub mod reducer;
pub mod replay;
pub mod result;
pub mod scheduler;
pub mod state_view;
pub mod verify;

pub use self::engine::{Core, Dispatcher};
pub use self::state::{
    ExecutionState, JobIoState, JobLifecycle, JobRuntime, JobState, Metrics, StoredResult,
    TimeoutEntry, TimeoutState,
};
pub use self::types::{
    Action, ActionKind, ActionList, ActionMeta, CORE_OWNER, CancelPolicy, CauseId, ControlSignal,
    Effect, Event, ExecError, ExecOutcome, ExecPolicy, ExecResult, ExecSpec, Handle, Intent,
    IoHandle, IoStream, IoTag, JobHandle, JobRequest, JobTag, LogEvent, LogLevel, Module, Priority,
    ProcessHandle, ProcessTag, RoutedAction, SystemService, WARMUP_OWNER, expand_intent,
    validate_intent,
};
