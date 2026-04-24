use crate::core::ActionKind;
use crate::core::Effect;
use crate::core::{Action, JobIoState, JobLifecycle};

pub struct ReducerCtx<'a> {
    pub core: &'a mut crate::core::core_state::CoreState,
    pub timeout: &'a mut crate::core::policy::TimeoutStateStore,
    pub result: &'a mut crate::core::result::ResultState,
    pub clock: &'a mut u64,
}

pub trait Reducer {
    fn handles(&self) -> &'static [ActionKind];
    fn apply(&self, ctx: &mut ReducerCtx, action: &Action, _effects: &mut Vec<Effect>);
}

pub struct JobReducer;

impl Reducer for JobReducer {
    fn handles(&self) -> &'static [ActionKind] {
        use std::sync::OnceLock;
        static HANDLES: OnceLock<Vec<ActionKind>> = OnceLock::new();
        HANDLES
            .get_or_init(|| {
                vec![
                    ActionKind::Submit,
                    ActionKind::SetLifecycle,
                    ActionKind::Finished,
                    ActionKind::Rejected,
                    ActionKind::CleanupJob,
                    ActionKind::TimeoutReached,
                    ActionKind::AssignProcess,
                    ActionKind::StartProcess,
                    ActionKind::PollProcess,
                    ActionKind::SignalProcess,
                    ActionKind::HandleProcessFailure,
                    ActionKind::SetJobIoState,
                    ActionKind::AssignIo,
                    ActionKind::HandleIoFailure,
                ]
            })
            .as_slice()
    }

    fn apply(&self, ctx: &mut ReducerCtx, action: &Action, effects: &mut Vec<Effect>) {
        let core = &mut ctx.core;
        match action {
            Action::Submit { id, owner, job: job_req } => {
                let policy = if job_req.command.first().map(|s| s.as_str()) == Some("dumpsys") {
                    crate::core::ExecPolicy { timeout_ms: Some(3000), kill_grace_ms: 500, cancel: crate::core::CancelPolicy::Kill }
                } else {
                    crate::core::ExecPolicy { timeout_ms: Some(1000), kill_grace_ms: 300, cancel: crate::core::CancelPolicy::Kill }
                };
                let exec = crate::core::ExecSpec {
                    argv: job_req.command.clone(), stdin: None, capture_stdout: true, capture_stderr: true, max_output: 1024 * 1024,
                };

                core.insert_job(*id, *owner, exec, policy);
            }
            Action::SetLifecycle { id, state: lifecycle_state } => {
                if let Some(h) = core.job_handle(*id) {
                    let old_job = core.job(h).clone();
                    if old_job.lifecycle != *lifecycle_state {
                        core.hash ^= crate::core::core_state::mix(*id, crate::core::core_state::hash_job(&old_job));
                        core.job_mut(h).lifecycle = *lifecycle_state;
                        core.hash ^= crate::core::core_state::mix(*id, crate::core::core_state::hash_job(core.job(h)));
                    }
                }
            }
            Action::Finished { id, .. } | Action::Rejected { id, .. } => {
                // ONLY semantic transition, NO removals here
                if let Some(h) = core.job_handle(*id) {
                    let old_job = core.job(h).clone();
                    if old_job.lifecycle != JobLifecycle::Finished {
                        core.hash ^= crate::core::core_state::mix(*id, crate::core::core_state::hash_job(&old_job));
                        core.job_mut(h).lifecycle = JobLifecycle::Finished;
                        core.hash ^= crate::core::core_state::mix(*id, crate::core::core_state::hash_job(core.job(h)));
                    }
                }
            }
            Action::CleanupJob { id } => {
                core.remove_job(*id);
            }
            Action::AssignProcess { id, process } => {
                if let Some(h) = core.job_handle(*id) {
                    let old_job = core.job(h).clone();
                    if old_job.process != Some(*process) {
                        core.hash ^= crate::core::core_state::mix(*id, crate::core::core_state::hash_job(&old_job));
                        core.job_mut(h).process = Some(*process);
                        core.hash ^= crate::core::core_state::mix(*id, crate::core::core_state::hash_job(core.job(h)));
                    }
                    if let Some(old) = core.runtime(h).process {
                        core.remove_process_index(old);
                    }
                    core.runtime_mut(h).process = Some(*process);
                    core.insert_process_index(*process, h);
                }
            }
            Action::AssignIo { id, io } => {
                if let Some(h) = core.job_handle(*id) {
                    let old_job = core.job(h).clone();
                    if old_job.io != Some(*io) {
                        core.hash ^= crate::core::core_state::mix(*id, crate::core::core_state::hash_job(&old_job));
                        core.job_mut(h).io = Some(*io);
                        core.hash ^= crate::core::core_state::mix(*id, crate::core::core_state::hash_job(core.job(h)));
                    }
                    if let Some(old) = core.runtime(h).io {
                        core.remove_io_index(old);
                    }
                    core.runtime_mut(h).io = Some(*io);
                    core.insert_io_index(*io, h);
                }
            }
            Action::SetJobIoState { id, state } => {
                if let Some(h) = core.job_handle(*id) {
                    let old_job = core.job(h).clone();
                    if old_job.io_state != *state {
                        core.hash ^= crate::core::core_state::mix(*id, crate::core::core_state::hash_job(&old_job));
                        core.job_mut(h).io_state = *state;
                        core.hash ^= crate::core::core_state::mix(*id, crate::core::core_state::hash_job(core.job(h)));
                    }
                }
            }
            Action::TimeoutReached { id } => {
                if let Some(h) = core.job_handle(*id) {
                    let old_job = core.job(h).clone();
                    if !old_job.timed_out {
                        core.hash ^= crate::core::core_state::mix(*id, crate::core::core_state::hash_job(&old_job));
                        core.job_mut(h).timed_out = true;
                        core.hash ^= crate::core::core_state::mix(*id, crate::core::core_state::hash_job(core.job(h)));
                    }
                }
            }
            Action::PollProcess { process } => {
                effects.push(crate::core::Effect::PollProcess { process: *process });
            }
            Action::StartProcess { id } => {
                if let Some(h) = core.job_handle(*id) {
                    let job = core.job(h);
                    effects.push(crate::core::Effect::StartProcess { id: *id, exec: job.exec.clone(), policy: job.policy.clone() });
                }
            }
            Action::SignalProcess { process, signal } => {
                effects.push(crate::core::Effect::KillProcess { process: *process, signal: *signal });
            }
            Action::HandleProcessFailure { process, err } => {
                if let Some(h) = core.job_by_process(*process) {
                    let id = core.job(h).id;
                    effects.push(crate::core::Effect::Log {
                        owner: core.job(h).owner,
                        level: crate::core::LogLevel::Error,
                        event: crate::core::LogEvent::Error { id, err: err.clone() },
                    });

                    let old_job = core.job(h).clone();
                    if old_job.process.is_some() {
                        core.hash ^= crate::core::core_state::mix(id, crate::core::core_state::hash_job(&old_job));
                        core.job_mut(h).process = None;
                        core.hash ^= crate::core::core_state::mix(id, crate::core::core_state::hash_job(core.job(h)));
                    }
                }
            }
            Action::HandleIoFailure { io, reason } => {
                if let Some(h) = core.job_by_io(*io) {
                    let id = core.job(h).id;
                    effects.push(crate::core::Effect::Log {
                        owner: core.job(h).owner,
                        level: crate::core::LogLevel::Error,
                        event: crate::core::LogEvent::Error { id, err: reason.clone() },
                    });

                    let old_job = core.job(h).clone();
                    if old_job.io.is_some() || old_job.io_state != JobIoState::Closed {
                        core.hash ^= crate::core::core_state::mix(id, crate::core::core_state::hash_job(&old_job));
                        let job_mut = core.job_mut(h);
                        job_mut.io = None;
                        job_mut.io_state = JobIoState::Closed;
                        core.hash ^= crate::core::core_state::mix(id, crate::core::core_state::hash_job(core.job(h)));
                    }
                }
            }
            _ => {}
        }
    }
}

pub struct IoReducer;

impl Reducer for IoReducer {
    fn handles(&self) -> &'static [ActionKind] {
        use std::sync::OnceLock;
        static HANDLES: OnceLock<Vec<ActionKind>> = OnceLock::new();
        HANDLES
            .get_or_init(|| {
                vec![
                    ActionKind::RegisterInterest,
                    ActionKind::RemoveInterest,
                    ActionKind::PerformIo,
                ]
            })
            .as_slice()
    }

    fn apply(&self, _ctx: &mut ReducerCtx, action: &Action, effects: &mut Vec<Effect>) {
        match action {
            Action::RegisterInterest { io, stream } => {
                effects.push(crate::core::Effect::WatchStream {
                    io: *io,
                    stream: stream.clone(),
                });
            }
            Action::RemoveInterest { io, stream } => {
                effects.push(crate::core::Effect::UnwatchStream { io: *io, stream: stream.clone() });
            }
            Action::PerformIo { io } => {
                effects.push(crate::core::Effect::PerformIo { io: *io });
            }
            _ => {}
        }
    }
}
pub struct TimeoutReducer;

impl Reducer for TimeoutReducer {
    fn handles(&self) -> &'static [ActionKind] {
        use std::sync::OnceLock;
        static HANDLES: OnceLock<Vec<ActionKind>> = OnceLock::new();
        HANDLES
            .get_or_init(|| {
                vec![
                    ActionKind::TrackTimeout,
                    ActionKind::UntrackTimeout,
                    ActionKind::UpdateTimeoutState,
                ]
            })
            .as_slice()
    }

    fn apply(&self, ctx: &mut ReducerCtx, action: &Action, _effects: &mut Vec<Effect>) {
        let ts = &mut ctx.timeout;
        match action {
            Action::TrackTimeout {
                id,
                deadline,
                kill_grace_ms,
            } => {
                if !ts.timeouts.contains_key(id) {
                    let entry = crate::core::TimeoutEntry {
                        id: *id,
                        state: crate::core::TimeoutState::WaitingForDeadline,
                        deadline: *deadline,
                        kill_grace_ms: *kill_grace_ms,
                    };
                    let st_hash = match entry.state {
                        crate::core::TimeoutState::WaitingForDeadline => 0,
                        crate::core::TimeoutState::WaitingForKillGrace(_) => 1,
                    };
                    ts.timeouts.insert(*id, entry);
                    ts.hash ^= id.wrapping_mul(0x5BD1E995);
                    ts.hash ^= id.wrapping_mul(0x5BD1E995).wrapping_add(st_hash);
                }
            }
            Action::UntrackTimeout { id } => {
                if let Some(entry) = ts.timeouts.remove(id) {
                    let st_hash = match entry.state {
                        crate::core::TimeoutState::WaitingForDeadline => 0,
                        crate::core::TimeoutState::WaitingForKillGrace(_) => 1,
                    };
                    ts.hash ^= id.wrapping_mul(0x5BD1E995);
                    ts.hash ^= id.wrapping_mul(0x5BD1E995).wrapping_add(st_hash);
                }
            }
            Action::UpdateTimeoutState {
                id,
                state: new_state,
            } => {
                if let Some(entry) = ts.timeouts.get_mut(id) {
                    if entry.state != *new_state {
                        let old_hash = match entry.state {
                            crate::core::TimeoutState::WaitingForDeadline => 0,
                            crate::core::TimeoutState::WaitingForKillGrace(_) => 1,
                        };
                        let new_hash = match new_state {
                            crate::core::TimeoutState::WaitingForDeadline => 0,
                            crate::core::TimeoutState::WaitingForKillGrace(_) => 1,
                        };
                        ts.hash ^= id.wrapping_mul(0x5BD1E995).wrapping_add(old_hash);
                        entry.state = new_state.clone();
                        ts.hash ^= id.wrapping_mul(0x5BD1E995).wrapping_add(new_hash);
                    }
                }
            }
            _ => {}
        }
    }
}

pub struct AddonReducer;

impl Reducer for AddonReducer {
    fn handles(&self) -> &'static [ActionKind] {
        use std::sync::OnceLock;
        static HANDLES: OnceLock<Vec<ActionKind>> = OnceLock::new();
        HANDLES
            .get_or_init(|| {
                vec![
                    ActionKind::AddonTask,
                    ActionKind::AddonEvent,
                    ActionKind::SystemRequest,
                    ActionKind::HandleAddonFailure,
                    ActionKind::HandleSystemFailure,
                ]
            })
            .as_slice()
    }

    fn apply(&self, _ctx: &mut ReducerCtx, action: &Action, effects: &mut Vec<Effect>) {
        match action {
            Action::AddonTask { addon_id, key, payload } => {
                effects.push(Effect::AddonTask { addon_id: *addon_id, key: key.clone(), payload: payload.clone() });
            }
            Action::AddonLog { addon_id, level, msg } => {
                effects.push(Effect::AddonLog { addon_id: *addon_id, level: *level, msg: msg.clone() });
            }
            Action::AddonEvent { addon_id: _, key: _ } => {
                // Generic notify, no effect yet
            }
            Action::EmitLog { owner, level, event } => {
                effects.push(Effect::Log { owner: *owner, level: *level, event: event.clone() });
            }
            Action::SystemRequest { request_id, kind, payload } => {
                effects.push(Effect::SystemRequest { request_id: *request_id, kind: *kind, payload: payload.clone() });
            }
            Action::HandleAddonFailure { addon_id, key, err } => {
                 effects.push(Effect::Log {
                    owner: *addon_id,
                    level: crate::core::LogLevel::Error,
                    event: crate::core::LogEvent::Error {
                        id: 0,
                        err: format!("Addon {} key {} failed: {}", addon_id, key, err),
                    },
                });
            }
            Action::HandleSystemFailure { request_id, kind, err } => {
                 effects.push(Effect::Log {
                    owner: crate::core::CORE_OWNER,
                    level: crate::core::LogLevel::Error,
                    event: crate::core::LogEvent::Error {
                        id: *request_id,
                        err: format!("SystemRequest {:?} failed: {}", kind, err),
                    },
                });
            }
            _ => {}
        }
    }
}

pub struct ResultReducer;

impl Reducer for ResultReducer {
    fn handles(&self) -> &'static [ActionKind] {
        use std::sync::OnceLock;
        static HANDLES: OnceLock<Vec<ActionKind>> = OnceLock::new();
        HANDLES
            .get_or_init(|| {
                vec![
                    ActionKind::Admitted,
                    ActionKind::Finished,
                    ActionKind::Rejected,
                ]
            })
            .as_slice()
    }

    fn apply(&self, ctx: &mut ReducerCtx, action: &Action, _effects: &mut Vec<Effect>) {
        let clock = *ctx.clock;
        match action {
            Action::Admitted { id: _ } => {
                let rs = &mut ctx.result;
                rs.hash ^= rs.active_jobs as u64;
                rs.active_jobs += 1;
                rs.hash ^= rs.active_jobs as u64;
            }
            Action::Finished {
                id,
                result,
                owner,
                was_submitted,
            } => {
                let owner = *owner;
                let was_submitted = *was_submitted;
                let rs = &mut ctx.result;
                if !was_submitted && rs.active_jobs > 0 {
                    rs.hash ^= rs.active_jobs as u64;
                    rs.active_jobs -= 1;
                    rs.hash ^= rs.active_jobs as u64;
                }
                if let Some(old) = rs.results.insert(
                    *id,
                    crate::core::StoredResult {
                        result: result.clone(),
                        owner,
                        created: clock,
                    },
                ) {
                    rs.hash ^= id.wrapping_mul(0x1234567).wrapping_add(old.owner as u64);
                }
                rs.result_order.push_back(*id);
                rs.hash ^= id.wrapping_mul(0x1234567).wrapping_add(owner as u64);
                if rs.result_order.len() > 100 {
                    if let Some(old_id) = rs.result_order.pop_front() {
                        if let Some(removed) = rs.results.remove(&old_id) {
                            rs.hash ^= old_id
                                .wrapping_mul(0x1234567)
                                .wrapping_add(removed.owner as u64);
                        }
                    }
                }
            }
            Action::Rejected {
                id,
                owner,
                was_submitted,
            } => {
                let owner = *owner;
                let was_submitted = *was_submitted;
                let rs = &mut ctx.result;
                if !was_submitted && rs.active_jobs > 0 {
                    rs.hash ^= rs.active_jobs as u64;
                    rs.active_jobs -= 1;
                    rs.hash ^= rs.active_jobs as u64;
                }
                if let Some(old) = rs.results.insert(
                    *id,
                    crate::core::StoredResult {
                        result: Err(crate::core::ExecError::Internal("Rejected".to_string())),
                        owner,
                        created: clock,
                    },
                ) {
                    rs.hash ^= id.wrapping_mul(0x1234567).wrapping_add(old.owner as u64);
                }
                rs.result_order.push_back(*id);
                rs.hash ^= id.wrapping_mul(0x1234567).wrapping_add(owner as u64);
                if rs.result_order.len() > 100 {
                    if let Some(old_id) = rs.result_order.pop_front() {
                        if let Some(removed) = rs.results.remove(&old_id) {
                            rs.hash ^= old_id
                                .wrapping_mul(0x1234567)
                                .wrapping_add(removed.owner as u64);
                        }
                    }
                }
            }
            _ => {}
        }
    }
}
pub struct LogReducer;

impl Reducer for LogReducer {
    fn handles(&self) -> &'static [ActionKind] {
        use std::sync::OnceLock;
        static HANDLES: OnceLock<Vec<ActionKind>> = OnceLock::new();
        HANDLES.get_or_init(|| vec![ActionKind::EmitLog]).as_slice()
    }

    fn apply(&self, _ctx: &mut ReducerCtx, action: &Action, effects: &mut Vec<Effect>) {
        if let Action::EmitLog {
            owner,
            level,
            event,
        } = action
        {
            effects.push(Effect::Log {
                owner: *owner,
                level: *level,
                event: event.clone(),
            });
        }
    }
}
pub struct TimeReducer;

impl Reducer for TimeReducer {
    fn handles(&self) -> &'static [ActionKind] {
        use std::sync::OnceLock;
        static HANDLES: OnceLock<Vec<ActionKind>> = OnceLock::new();
        HANDLES
            .get_or_init(|| vec![ActionKind::AdvanceTime])
            .as_slice()
    }

    fn apply(&self, ctx: &mut ReducerCtx, action: &Action, _effects: &mut Vec<Effect>) {
        if let Action::AdvanceTime { delta } = action {
            *ctx.clock += delta;
        }
    }
}
