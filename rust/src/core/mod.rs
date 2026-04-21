use serde::{Deserialize, Serialize};
pub mod policy;
pub mod validation;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum CancelPolicy {
    None,
    Graceful,
    Kill,
}

use std::hash::{Hash, Hasher};
use std::marker::PhantomData;

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogLevel {
    Info,
    Warn,
    Error,
}

#[derive(Clone)]
pub enum LogEvent {
    Submit { id: u64 },
    Spawn { id: u64, pid: i32 },
    Cancel { id: u64 },
    ForceKill { id: u64 },
    Exit { id: u64, status: Option<i32> },
    Timeout { id: u64 },
    Error { id: u64, err: String },

    TickStart,
    Observability { queue_len: usize, actions_processed: usize, dropped: usize },
    TickEnd,
    AddonReceived,
    AddonTranslated,
    AddonDropped,
    ActionDispatched,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ControlSignal {
    GracefulStop,
    ForceKill,
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
    AppForegrounded {
        pid: i32,
    },
    PackagesChanged,
    WarmupCompleted {
        package: String,
        bytes: u64,
        duration_ms: u64,
    },
    PackageNameResolved {
        pid: i32,
        package_name: String,
    },
    PackageDirResolved {
        package_name: String,
        base_dir: String,
    },
    PackagePathsDiscovered {
        package_name: String,
        paths: Vec<String>,
    },
    TimeAdvanced(u64),

    // Failure Events
    PackageResolutionFailed {
        pid: i32,
        err: String,
    },
    PackageDirResolutionFailed {
        package_name: String,
        err: String,
    },
    PackagePathsDiscoveryFailed {
        package_name: String,
        err: String,
    },
    WarmupFailed {
        package_name: String,
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
    AppForegrounded,
    PackagesChanged,
    RequestPackageName,
    PackageNameResolved,
    ResolvePackage,
    RequestPackageDir,
    PackageDirResolved,
    DiscoverPaths,
    RequestPackagePaths,
    PackagePathsDiscovered,
    ScheduleWarmup,
    DispatchWarmupChunk,
    CompleteWarmup,
    CleanupJob,
    TrackTimeout,
    UntrackTimeout,
    UpdateTimeoutState,
    AdvanceTime,
    HandleWarmupFailure,
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
    AppForegrounded {
        pid: i32,
    },
    PackagesChanged,
}

pub fn validate_intent(intent: &Intent) -> bool {
    match intent {
        Intent::Submit { job, .. } => !job.command.is_empty() && job.command.len() < 64,
        Intent::Control { .. } => true,
        Intent::Query { .. } => true,
        Intent::AppForegrounded { pid } => *pid > 0,
        Intent::PackagesChanged => true,
    }
}

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

pub fn expand_intent(intent: Intent, now: u64) -> Vec<Action> {
    match intent {
        Intent::Submit { id, owner, job } => {
            let mut actions = vec![Action::Submit {
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
        Intent::Control { id, signal } => {
            vec![Action::Control { id, signal }]
        }
        Intent::Query { id } => {
            vec![Action::Query { id }]
        }
        Intent::AppForegrounded { pid } => {
            vec![Action::AppForegrounded { pid }]
        }
        Intent::PackagesChanged => {
            vec![Action::PackagesChanged]
        }
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

    // State Transitions
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

    // Semantic Intents
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

    // Input actions
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

    // Warmup stages
    AppForegrounded {
        pid: i32,
    },
    PackagesChanged,

    RequestPackageName {
        pid: i32,
    },
    PackageNameResolved {
        pid: i32,
        package_name: String,
    },

    ResolvePackage {
        package_name: String,
    },

    RequestPackageDir {
        package_name: String,
    },
    PackageDirResolved {
        package_name: String,
        base_dir: String,
    },

    DiscoverPaths {
        package_name: String,
        base_dir: String,
    },
    RequestPackagePaths {
        package_name: String,
        base_dir: String,
    },
    PackagePathsDiscovered {
        package_name: String,
        paths: Vec<String>,
    },

    ScheduleWarmup {
        package_name: String,
        paths: Vec<String>,
    },
    DispatchWarmupChunk {
        package_name: String,
        path: String,
    },
    CompleteWarmup {
        package_name: String,
    },
    CleanupJob {
        id: u64,
    },

    // Timeout intent tracking (pure actions)
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

    HandleWarmupFailure {
        package_name: String,
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
            Action::AppForegrounded { .. } => ActionKind::AppForegrounded,
            Action::PackagesChanged => ActionKind::PackagesChanged,
            Action::RequestPackageName { .. } => ActionKind::RequestPackageName,
            Action::PackageNameResolved { .. } => ActionKind::PackageNameResolved,
            Action::ResolvePackage { .. } => ActionKind::ResolvePackage,
            Action::RequestPackageDir { .. } => ActionKind::RequestPackageDir,
            Action::PackageDirResolved { .. } => ActionKind::PackageDirResolved,
            Action::DiscoverPaths { .. } => ActionKind::DiscoverPaths,
            Action::RequestPackagePaths { .. } => ActionKind::RequestPackagePaths,
            Action::PackagePathsDiscovered { .. } => ActionKind::PackagePathsDiscovered,
            Action::ScheduleWarmup { .. } => ActionKind::ScheduleWarmup,
            Action::DispatchWarmupChunk { .. } => ActionKind::DispatchWarmupChunk,
            Action::CompleteWarmup { .. } => ActionKind::CompleteWarmup,
            Action::CleanupJob { .. } => ActionKind::CleanupJob,
            Action::TrackTimeout { .. } => ActionKind::TrackTimeout,
            Action::UntrackTimeout { .. } => ActionKind::UntrackTimeout,
            Action::UpdateTimeoutState { .. } => ActionKind::UpdateTimeoutState,
            Action::HandleWarmupFailure { .. } => ActionKind::HandleWarmupFailure,
            Action::HandleProcessFailure { .. } => ActionKind::HandleProcessFailure,
            Action::HandleIoFailure { .. } => ActionKind::HandleIoFailure,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Priority {
    Critical,   // lifecycle, failure, control
    Normal,     // core flow, intent
    Background, // warmup, probes, logs
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
            | Action::HandleWarmupFailure { .. }
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
            Action::AppForegrounded { .. }
            | Action::PackagesChanged
            | Action::RequestPackageName { .. }
            | Action::PackageNameResolved { .. }
            | Action::ResolvePackage { .. }
            | Action::RequestPackageDir { .. }
            | Action::PackageDirResolved { .. }
            | Action::DiscoverPaths { .. }
            | Action::RequestPackagePaths { .. }
            | Action::PackagePathsDiscovered { .. }
            | Action::ScheduleWarmup { .. }
            | Action::DispatchWarmupChunk { .. }
            | Action::CompleteWarmup { .. }
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
    Warmup {
        source: String,
        paths: Vec<String>,
    },
    ResolvePackageName {
        pid: i32,
    },
    DiscoverPackageDir {
        package_name: String,
    },
    DiscoverPackagePaths {
        package_name: String,
        base_dir: String,
    },
}

pub trait Module {
    fn handle(
        &self,
        state: &dyn crate::core::state_view::StateView,
        action: &Action,
    ) -> Vec<Action>;
    fn handle_event(
        &self,
        state: &dyn crate::core::state_view::StateView,
        event: &Event,
    ) -> Vec<Action>;
}

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
pub mod warmup;

pub const CORE_OWNER: u32 = 0;
pub const WARMUP_OWNER: u32 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JobLifecycle {
    Submitted,
    Admitted,
    Running,
    Terminating,
    Killed,
    Finished,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JobIoState {
    Pending,
    Active,
    Closed,
}

#[derive(Clone)]
pub struct JobState {
    pub id: u64,
    pub owner: u32,
    pub exec: ExecSpec,
    pub policy: ExecPolicy,
    pub process: Option<ProcessHandle>,
    pub io: Option<IoHandle>,
    pub timed_out: bool,
    pub lifecycle: JobLifecycle,
    pub io_state: JobIoState,
}

#[derive(Clone)]
pub struct StoredResult {
    pub result: Result<ExecResult, ExecError>,
    pub owner: u32,
    pub created: u64,
}

#[derive(Clone)]
pub struct JobRuntime {
    pub process: Option<ProcessHandle>,
    pub io: Option<IoHandle>,
}

#[derive(Clone)]
pub struct TimeoutEntry {
    pub id: u64,
    pub state: TimeoutState,
    pub deadline: u64,
    pub kill_grace_ms: u32,
}

#[derive(Clone, PartialEq, Eq)]
pub enum TimeoutState {
    WaitingForDeadline,
    WaitingForKillGrace(u64),
}

pub struct ExecutionState {
    pub core: crate::core::core_state::CoreState,
    pub warmup: crate::core::warmup::WarmupState,
    pub timeout: crate::core::policy::TimeoutStateStore,
    pub result: crate::core::result::ResultState,
    pub clock: u64,
    pub hash: u64,
}

impl Default for ExecutionState {
    fn default() -> Self {
        Self::new()
    }
}

impl ExecutionState {
    pub fn new() -> Self {
        Self {
            core: crate::core::core_state::CoreState::new(),
            warmup: crate::core::warmup::WarmupState::new(),
            timeout: crate::core::policy::TimeoutStateStore::new(),
            result: crate::core::result::ResultState::new(),
            clock: 0,
            hash: 0,
        }
    }

    pub fn update_hash(&mut self) {
        self.hash = self.core.hash ^ self.warmup.hash ^ self.timeout.hash ^ self.result.hash;
    }
}

impl crate::core::state_view::StateView for ExecutionState {
    fn job(&self, id: u64) -> Option<crate::core::state_view::JobView> {
        let h = self.core.job_handle(id)?;
        let j = self.core.job(h);
        Some(crate::core::state_view::JobView {
            id: j.id,
            owner: j.owner,
            lifecycle: j.lifecycle,
            io_state: j.io_state,
            process: j.process,
            io: j.io,
            timed_out: j.timed_out,
        })
    }

    fn job_by_process(&self, process: ProcessHandle) -> Option<crate::core::state_view::JobView> {
        let h = self.core.job_by_process(process)?;
        let j = self.core.job(h);
        Some(crate::core::state_view::JobView {
            id: j.id,
            owner: j.owner,
            lifecycle: j.lifecycle,
            io_state: j.io_state,
            process: j.process,
            io: j.io,
            timed_out: j.timed_out,
        })
    }

    fn job_by_io(&self, io: IoHandle) -> Option<crate::core::state_view::JobView> {
        let h = self.core.job_by_io(io)?;
        let j = self.core.job(h);
        Some(crate::core::state_view::JobView {
            id: j.id,
            owner: j.owner,
            lifecycle: j.lifecycle,
            io_state: j.io_state,
            process: j.process,
            io: j.io,
            timed_out: j.timed_out,
        })
    }

    fn result(&self, id: u64) -> Option<crate::core::state_view::ResultView> {
        self.result
            .results
            .get(&id)
            .map(|r| crate::core::state_view::ResultView {
                result: r.result.clone(),
                owner: r.owner,
            })
    }

    fn active_jobs(&self) -> usize {
        self.result.active_jobs
    }

    fn max_jobs(&self) -> usize {
        self.result.max_jobs
    }

    fn is_warmup_in_flight(&self, package: &str) -> bool {
        self.warmup.in_flight.contains(package)
    }

    fn warmup_last_run(&self, package: &str) -> Option<u64> {
        self.warmup.dedup_cache.get(package).copied()
    }

    fn warmup_negative_cached(&self, package: &str) -> Option<u64> {
        self.warmup.negative_cache.get(package).copied()
    }

    fn warmup_base_dir(&self, package: &str) -> Option<String> {
        self.warmup
            .package_map
            .get(package)
            .map(|p| p.to_string_lossy().into_owned())
    }

    fn timeouts(&self) -> Vec<crate::core::state_view::TimeoutView> {
        self.timeout
            .timeouts
            .values()
            .map(|t| crate::core::state_view::TimeoutView {
                id: t.id,
                state: t.state.clone(),
                deadline: t.deadline,
                kill_grace_ms: t.kill_grace_ms,
            })
            .collect()
    }

    fn now(&self) -> u64 {
        self.clock
    }
}

pub struct Dispatcher {
    pub modules: Vec<Box<dyn Module>>,
}

impl Default for Dispatcher {
    fn default() -> Self {
        Self::new()
    }
}

impl Dispatcher {
    pub fn new() -> Self {
        Self {
            // ORDER IS SEMANTIC. DO NOT REORDER:
            // 1. AdmissionControl
            // 2. Lifecycle
            // 3. Process
            // 4. IO
            // 5. Result
            // 6. Timeout
            // 7. Warmup
            modules: vec![
                Box::new(crate::core::policy::AdmissionControlModule),
                Box::new(crate::core::lifecycle::LifecycleModule),
                Box::new(crate::core::process::ProcessModule),
                Box::new(crate::core::io::IoModule),
                Box::new(crate::core::result::ResultModule),
                Box::new(crate::core::policy::TimeoutPolicyModule::new()),
                Box::new(crate::core::warmup::WarmupModule),
            ],
        }
    }

    pub fn dispatch(
        &self,
        state: &dyn crate::core::state_view::StateView,
        action: &Action,
    ) -> Vec<Action> {
        let mut actions = Vec::new();
        for module in &self.modules {
            actions.extend(module.handle(state, action));
        }
        actions
    }

    pub fn dispatch_event(
        &self,
        state: &dyn crate::core::state_view::StateView,
        event: &Event,
    ) -> Vec<Action> {
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
                TimeoutState::WaitingForDeadline => entry.deadline,
                TimeoutState::WaitingForKillGrace(d) => d,
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
    pub routing: std::collections::HashMap<ActionKind, Vec<usize>>,
}

impl Core {
    pub fn new() -> Self {
        let reducers: Vec<Box<dyn crate::core::reducer::Reducer>> = vec![
            Box::new(crate::core::reducer::TimeReducer),
            Box::new(crate::core::reducer::ResultReducer),
            Box::new(crate::core::reducer::IoReducer),
            Box::new(crate::core::reducer::JobReducer),
            Box::new(crate::core::reducer::TimeoutReducer),
            Box::new(crate::core::reducer::WarmupReducer),
            Box::new(crate::core::reducer::LogReducer),
        ];

        for reducer in reducers.iter() {
            assert!(
                !reducer.handles().is_empty(),
                "Reducer must handle at least one action"
            );
        }

        let mut routing: std::collections::HashMap<ActionKind, Vec<usize>> =
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
