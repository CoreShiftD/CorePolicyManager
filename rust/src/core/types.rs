// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/

use serde::{Deserialize, Serialize};
use smallvec::{SmallVec, smallvec};
use std::hash::{Hash, Hasher};
use std::marker::PhantomData;

use crate::core::{JobIoState, JobLifecycle, TimeoutState};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum CancelPolicy {
    None,
    Graceful,
    Kill,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ProcessTag;

#[derive(Debug, Serialize, Deserialize)]
pub struct IoTag;

#[derive(Debug, Serialize, Deserialize)]
pub struct JobTag;
pub type JobHandle = Handle<JobTag>;

#[derive(Debug, Serialize, Deserialize)]
pub struct Handle<T> {
    pub index: u32,
    pub generation: u32,
    #[serde(skip, default)]
    pub _marker: PhantomData<T>,
}

impl<T> Clone for Handle<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> Copy for Handle<T> {}

impl<T> PartialEq for Handle<T> {
    fn eq(&self, other: &Self) -> bool {
        self.index == other.index && self.generation == other.generation
    }
}

impl<T> Eq for Handle<T> {}

impl<T> Hash for Handle<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.index.hash(state);
        self.generation.hash(state);
    }
}

pub type ProcessHandle = Handle<ProcessTag>;
pub type IoHandle = Handle<IoTag>;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecSpec {
    pub argv: Vec<String>,
    pub stdin: Option<Vec<u8>>,
    pub capture_stdout: bool,
    pub capture_stderr: bool,
    pub max_output: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecPolicy {
    pub timeout_ms: Option<u32>,
    pub kill_grace_ms: u32,
    pub cancel: CancelPolicy,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ExecResult {
    pub status: Option<i32>,
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
    pub timed_out: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum ExecError {
    SpawnFailed,
    RuntimeError,
    Internal(String),
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ExecOutcome {
    pub id: u64,
    pub result: Result<ExecResult, ExecError>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LogLevel {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
}

#[derive(Clone, Serialize, Deserialize)]
pub enum LogEvent {
    TickSummary {
        processed: usize,
        dropped: usize,
        queue_before: usize,
        queue_after: usize,
        elapsed_us: u64,
    },
    ActionDispatch {
        kind: ActionKind,
        id: Option<u64>,
        addon_id: Option<u32>,
        key: Option<String>,
        service: Option<SystemService>,
        payload_len: usize,
    },
    PreloadForeground {
        pid: i32,
        package: String,
    },
    PreloadSkip {
        package: String,
        reason: String,
        remaining_ms: Option<u64>,
    },
    PreloadStart {
        package: String,
        paths: usize,
    },
    PreloadDone {
        package: String,
        paths: usize,
        bytes: u64,
        duration_ms: u64,
    },
    PreloadFail {
        package: String,
        reason: String,
        backoff_ms: u64,
    },
    Generic(String),
    Error {
        id: u64,
        err: String,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ControlSignal {
    GracefulStop,
    ForceKill,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SystemService {
    ResolveIdentity,
    ResolveDirectory,
    DiscoverPaths,
}

#[derive(Clone, Serialize, Deserialize)]
pub enum Event {
    Tick,
    ProcessStarted {
        id: u64,
        process: ProcessHandle,
        io: IoHandle,
    },
    ProcessSpawnFailed {
        id: u64,
        err: String,
    },
    ProcessExited {
        process: ProcessHandle,
        status: Option<i32>,
    },
    IoReady {
        io: IoHandle,
        stream: IoStream,
        readable: bool,
        writable: bool,
        error: bool,
    },
    IoClosed {
        io: IoHandle,
    },
    ForegroundChanged {
        pid: i32,
    },
    PackagesChanged,
    AddonCompleted {
        addon_id: u32,
        key: String,
        payload: Vec<u8>,
    },
    SystemResponse {
        request_id: u64,
        kind: SystemService,
        payload: Vec<u8>,
    },
    SystemFailure {
        request_id: u64,
        kind: SystemService,
        err: String,
    },
    TimeAdvanced(u64),
    AddonFailed {
        addon_id: u32,
        key: String,
        err: String,
    },
    WatchStreamFailed {
        io: IoHandle,
        err: String,
    },
    DroppedAction {
        kind: ActionKind,
    },
    KillProcessFailed {
        process: ProcessHandle,
        err: String,
    },
    ReactorError {
        err: String,
    },
    IoFailed {
        io: IoHandle,
        reason: String,
    },
}

#[derive(Hash, Eq, PartialEq, Clone, Copy, Debug, Serialize, Deserialize)]
#[repr(u8)]
pub enum ActionKind {
    Submit = 0,
    Admitted,
    Rejected,
    Started,
    Controlled,
    Finished,
    QueryResult,
    SetJobIoState,
    AssignProcess,
    AssignIo,
    SetLifecycle,
    StartProcess,
    SignalProcess,
    PollProcess,
    PerformIo,
    RegisterInterest,
    RemoveInterest,
    EmitLog,
    Control,
    Query,
    TimeoutReached,
    KillDeadlineReached,
    ForegroundChanged,
    PackagesChanged,
    SystemRequest,
    AddonTask,
    AddonLog,
    AddonEvent,
    CleanupJob,
    TrackTimeout,
    UntrackTimeout,
    UpdateTimeoutState,
    AdvanceTime,
    HandleAddonFailure,
    HandleSystemFailure,
    HandleProcessFailure,
    HandleIoFailure,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct JobRequest {
    pub command: Vec<String>,
}

#[derive(Clone, Serialize, Deserialize)]
pub enum Intent {
    Submit {
        id: u64,
        owner: u32,
        job: JobRequest,
    },
    Control {
        id: u64,
        signal: ControlSignal,
    },
    Query {
        id: u64,
    },
    ForegroundChanged {
        pid: i32,
    },
    PackagesChanged,
    SystemRequest {
        request_id: u64,
        kind: SystemService,
        payload: Vec<u8>,
    },
    AddonTask {
        addon_id: u32,
        key: String,
        payload: Vec<u8>,
    },
    AddonLog {
        addon_id: u32,
        level: LogLevel,
        msg: String,
    },
}

pub fn validate_intent(intent: &Intent) -> bool {
    match intent {
        Intent::Submit { job, .. } => !job.command.is_empty() && job.command.len() < 64,
        Intent::Control { .. }
        | Intent::Query { .. }
        | Intent::PackagesChanged
        | Intent::SystemRequest { .. }
        | Intent::AddonTask { .. }
        | Intent::AddonLog { .. } => true,
        Intent::ForegroundChanged { pid } => *pid > 0,
    }
}

pub type ActionList = SmallVec<[Action; 4]>;

fn default_policy(cmd: &[String]) -> ExecPolicy {
    if cmd.first().map(|s| s.as_str()) == Some("dumpsys") {
        ExecPolicy {
            timeout_ms: Some(3000),
            kill_grace_ms: 500,
            cancel: CancelPolicy::Kill,
        }
    } else {
        ExecPolicy {
            timeout_ms: Some(1000),
            kill_grace_ms: 300,
            cancel: CancelPolicy::Kill,
        }
    }
}

pub fn expand_intent(intent: Intent, now: u64) -> ActionList {
    match intent {
        Intent::Submit { id, owner, job } => {
            let mut actions = smallvec![Action::Submit {
                id,
                owner,
                job: job.clone(),
            }];

            let policy = default_policy(&job.command);
            if let Some(timeout_ms) = policy.timeout_ms {
                actions.push(Action::TrackTimeout {
                    id,
                    deadline: now + (timeout_ms as u64),
                    kill_grace_ms: policy.kill_grace_ms,
                });
            }

            actions
        }
        Intent::Control { id, signal } => smallvec![Action::Control { id, signal }],
        Intent::Query { id } => smallvec![Action::Query { id }],
        Intent::ForegroundChanged { pid } => smallvec![Action::ForegroundChanged { pid }],
        Intent::PackagesChanged => smallvec![Action::PackagesChanged],
        Intent::SystemRequest {
            request_id,
            kind,
            payload,
        } => smallvec![Action::SystemRequest {
            request_id,
            kind,
            payload,
        }],
        Intent::AddonTask {
            addon_id,
            key,
            payload,
        } => smallvec![Action::AddonTask {
            addon_id,
            key,
            payload,
        }],
        Intent::AddonLog {
            addon_id,
            level,
            msg,
        } => smallvec![Action::AddonLog {
            addon_id,
            level,
            msg,
        }],
    }
}

#[derive(Clone)]
pub enum Action {
    AdvanceTime {
        delta: u64,
    },
    Submit {
        id: u64,
        owner: u32,
        job: JobRequest,
    },
    Admitted {
        id: u64,
    },
    Rejected {
        id: u64,
        owner: u32,
        was_submitted: bool,
    },
    Started {
        id: u64,
    },
    Controlled {
        id: u64,
    },
    Finished {
        id: u64,
        owner: u32,
        was_submitted: bool,
        result: Result<ExecResult, ExecError>,
    },
    QueryResult {
        id: u64,
        result: Option<ExecOutcome>,
    },
    SetJobIoState {
        id: u64,
        state: JobIoState,
    },
    AssignProcess {
        id: u64,
        process: ProcessHandle,
    },
    AssignIo {
        id: u64,
        io: IoHandle,
    },
    SetLifecycle {
        id: u64,
        state: JobLifecycle,
    },
    StartProcess {
        id: u64,
    },
    SignalProcess {
        process: ProcessHandle,
        signal: ControlSignal,
    },
    PollProcess {
        process: ProcessHandle,
    },
    PerformIo {
        io: IoHandle,
    },
    RegisterInterest {
        io: IoHandle,
        stream: IoStream,
    },
    RemoveInterest {
        io: IoHandle,
        stream: IoStream,
    },
    EmitLog {
        owner: u32,
        level: LogLevel,
        event: LogEvent,
    },
    Control {
        id: u64,
        signal: ControlSignal,
    },
    Query {
        id: u64,
    },
    TimeoutReached {
        id: u64,
    },
    KillDeadlineReached {
        id: u64,
    },
    ForegroundChanged {
        pid: i32,
    },
    PackagesChanged,
    SystemRequest {
        request_id: u64,
        kind: SystemService,
        payload: Vec<u8>,
    },
    AddonTask {
        addon_id: u32,
        key: String,
        payload: Vec<u8>,
    },
    AddonLog {
        addon_id: u32,
        level: LogLevel,
        msg: String,
    },
    AddonEvent {
        addon_id: u32,
        key: String,
    },
    CleanupJob {
        id: u64,
    },
    TrackTimeout {
        id: u64,
        deadline: u64,
        kill_grace_ms: u32,
    },
    UntrackTimeout {
        id: u64,
    },
    UpdateTimeoutState {
        id: u64,
        state: TimeoutState,
    },
    HandleSystemFailure {
        request_id: u64,
        kind: SystemService,
        err: String,
    },
    HandleAddonFailure {
        addon_id: u32,
        key: String,
        err: String,
    },
    HandleProcessFailure {
        process: ProcessHandle,
        err: String,
    },
    HandleIoFailure {
        io: IoHandle,
        reason: String,
    },
}

impl Action {
    pub fn kind(&self) -> ActionKind {
        match self {
            Action::AdvanceTime { .. } => ActionKind::AdvanceTime,
            Action::Submit { .. } => ActionKind::Submit,
            Action::Admitted { .. } => ActionKind::Admitted,
            Action::Rejected { .. } => ActionKind::Rejected,
            Action::Started { .. } => ActionKind::Started,
            Action::Controlled { .. } => ActionKind::Controlled,
            Action::Finished { .. } => ActionKind::Finished,
            Action::QueryResult { .. } => ActionKind::QueryResult,
            Action::SetJobIoState { .. } => ActionKind::SetJobIoState,
            Action::AssignProcess { .. } => ActionKind::AssignProcess,
            Action::AssignIo { .. } => ActionKind::AssignIo,
            Action::SetLifecycle { .. } => ActionKind::SetLifecycle,
            Action::StartProcess { .. } => ActionKind::StartProcess,
            Action::SignalProcess { .. } => ActionKind::SignalProcess,
            Action::PollProcess { .. } => ActionKind::PollProcess,
            Action::PerformIo { .. } => ActionKind::PerformIo,
            Action::RegisterInterest { .. } => ActionKind::RegisterInterest,
            Action::RemoveInterest { .. } => ActionKind::RemoveInterest,
            Action::EmitLog { .. } => ActionKind::EmitLog,
            Action::Control { .. } => ActionKind::Control,
            Action::Query { .. } => ActionKind::Query,
            Action::TimeoutReached { .. } => ActionKind::TimeoutReached,
            Action::KillDeadlineReached { .. } => ActionKind::KillDeadlineReached,
            Action::ForegroundChanged { .. } => ActionKind::ForegroundChanged,
            Action::PackagesChanged => ActionKind::PackagesChanged,
            Action::SystemRequest { .. } => ActionKind::SystemRequest,
            Action::AddonTask { .. } => ActionKind::AddonTask,
            Action::AddonLog { .. } => ActionKind::AddonLog,
            Action::AddonEvent { .. } => ActionKind::AddonEvent,
            Action::CleanupJob { .. } => ActionKind::CleanupJob,
            Action::TrackTimeout { .. } => ActionKind::TrackTimeout,
            Action::UntrackTimeout { .. } => ActionKind::UntrackTimeout,
            Action::UpdateTimeoutState { .. } => ActionKind::UpdateTimeoutState,
            Action::HandleSystemFailure { .. } => ActionKind::HandleSystemFailure,
            Action::HandleAddonFailure { .. } => ActionKind::HandleAddonFailure,
            Action::HandleProcessFailure { .. } => ActionKind::HandleProcessFailure,
            Action::HandleIoFailure { .. } => ActionKind::HandleIoFailure,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Priority {
    Critical,
    Normal,
    Background,
}

impl Action {
    pub fn priority(&self) -> Priority {
        match self {
            Action::Control { .. }
            | Action::SignalProcess { .. }
            | Action::KillDeadlineReached { .. }
            | Action::TimeoutReached { .. }
            | Action::Admitted { .. }
            | Action::Rejected { .. }
            | Action::Started { .. }
            | Action::Controlled { .. }
            | Action::Finished { .. }
            | Action::SetJobIoState { .. }
            | Action::AssignProcess { .. }
            | Action::AssignIo { .. }
            | Action::SetLifecycle { .. }
            | Action::UpdateTimeoutState { .. }
            | Action::TrackTimeout { .. }
            | Action::UntrackTimeout { .. }
            | Action::AdvanceTime { .. }
            | Action::HandleSystemFailure { .. }
            | Action::HandleAddonFailure { .. }
            | Action::HandleProcessFailure { .. }
            | Action::HandleIoFailure { .. } => Priority::Critical,
            Action::Submit { .. }
            | Action::StartProcess { .. }
            | Action::PollProcess { .. }
            | Action::PerformIo { .. }
            | Action::RegisterInterest { .. }
            | Action::RemoveInterest { .. }
            | Action::Query { .. }
            | Action::QueryResult { .. }
            | Action::CleanupJob { .. } => Priority::Normal,
            Action::ForegroundChanged { .. }
            | Action::PackagesChanged
            | Action::SystemRequest { .. }
            | Action::AddonTask { .. }
            | Action::AddonLog { .. }
            | Action::AddonEvent { .. }
            | Action::EmitLog { .. } => Priority::Background,
        }
    }
}

#[derive(Clone, Copy, Debug, Hash, Eq, PartialEq)]
pub struct CauseId(pub u64);

#[derive(Clone)]
pub struct ActionMeta {
    pub id: CauseId,
    pub parent: Option<CauseId>,
    pub source: crate::high_level::identity::Principal,
    pub reply_to: Option<u32>,
}

#[derive(Clone)]
pub struct RoutedAction {
    pub action: Action,
    pub meta: ActionMeta,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum IoStream {
    Stdout,
    Stderr,
    Stdin,
}

pub enum Effect {
    Log {
        owner: u32,
        level: LogLevel,
        event: LogEvent,
    },
    WatchStream {
        io: IoHandle,
        stream: IoStream,
    },
    UnwatchStream {
        io: IoHandle,
        stream: IoStream,
    },
    StartProcess {
        id: u64,
        exec: ExecSpec,
        policy: ExecPolicy,
    },
    KillProcess {
        process: ProcessHandle,
        signal: ControlSignal,
    },
    PollProcess {
        process: ProcessHandle,
    },
    PerformIo {
        io: IoHandle,
    },
    AddonTask {
        addon_id: u32,
        key: String,
        payload: Vec<u8>,
    },
    AddonLog {
        addon_id: u32,
        level: LogLevel,
        msg: String,
    },
    SystemRequest {
        request_id: u64,
        kind: SystemService,
        payload: Vec<u8>,
    },
}

pub trait Module {
    fn handle(&self, state: &dyn crate::core::state_view::StateView, action: &Action)
    -> ActionList;
    fn handle_event(
        &self,
        state: &dyn crate::core::state_view::StateView,
        event: &Event,
    ) -> ActionList;
}

pub const CORE_OWNER: u32 = 0;
pub const WARMUP_OWNER: u32 = 1;
