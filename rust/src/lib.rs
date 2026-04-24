// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/

#![allow(non_snake_case)]

#[macro_use]
pub mod low_level;

/// `mid_level` handles IPC and other boundary protocols between the runtime
/// and external clients.
pub mod mid_level;

pub mod arena;
/// `core` is the pure state machine and reducer/scheduler layer.
pub mod core;
/// `high_level` contains addon semantics and policy-level behaviors.
pub mod high_level;
pub mod paths;
/// `runtime` owns side effects, Android/system services, logging, and process
/// execution.
pub mod runtime;

#[cfg(test)]
mod tests;

#[derive(Debug)]
pub enum RuntimeLimit {
    StepBudgetExceeded,
    QueueOverflow,
    ActionRepetitionExceeded,
    CauseDepthExceeded,
}

pub struct DaemonConfig {
    pub enable_warmup: bool,
    pub record_path: Option<String>,
}

pub struct TraceStore {
    pub parents: std::collections::HashMap<crate::core::CauseId, Option<crate::core::CauseId>>,
    pub order: std::collections::VecDeque<crate::core::CauseId>,
}

impl Default for TraceStore {
    fn default() -> Self {
        Self::new()
    }
}

impl TraceStore {
    pub fn new() -> Self {
        Self {
            parents: std::collections::HashMap::new(),
            order: std::collections::VecDeque::new(),
        }
    }

    pub fn insert(&mut self, id: crate::core::CauseId, parent: Option<crate::core::CauseId>) {
        self.parents.insert(id, parent);
        self.order.push_back(id);
        if self.order.len() > 10_000
            && let Some(old_id) = self.order.pop_front()
        {
            self.parents.remove(&old_id);
        }
    }
}

use std::sync::atomic::{AtomicBool, Ordering};
static RUNNING: AtomicBool = AtomicBool::new(true);

extern "C" fn handle_signal(_sig: libc::c_int) {
    RUNNING.store(false, Ordering::SeqCst);
}

const TICK_MS: u64 = 16;

#[inline]
fn compute_reactor_timeout_ms(policy_timeout_ms: i32, elapsed_ms: u64) -> i32 {
    let tick_timeout_ms = if elapsed_ms >= TICK_MS {
        0
    } else {
        (TICK_MS - elapsed_ms) as i32
    };

    match policy_timeout_ms {
        -1 => tick_timeout_ms,
        0.. => policy_timeout_ms.min(tick_timeout_ms),
        _ => tick_timeout_ms,
    }
}

pub fn run_daemon(config: DaemonConfig) -> Result<(), crate::low_level::spawn::SysError> {
    const MAX_ACTIONS_PER_TICK: usize = 10_000;

    RUNNING.store(true, Ordering::SeqCst);

    // Ensure runtime directories exist
    if let Err(e) = paths::ensure_dirs() {
        return Err(crate::low_level::spawn::SysError::sys(
            e.raw_os_error().unwrap_or(0),
            "ensure_dirs",
        ));
    }

    // PID file management
    let pid = std::process::id();

    // Register signal handlers
    unsafe {
        libc::signal(
            libc::SIGTERM,
            handle_signal as *const () as libc::sighandler_t,
        );
        libc::signal(
            libc::SIGINT,
            handle_signal as *const () as libc::sighandler_t,
        );
    }

    use crate::core::{Core, ExecutionState};
    use crate::high_level::capability::{CapabilityRegistry, CapabilityToken};
    use crate::low_level::reactor::{Fd, Reactor, Token};
    use crate::mid_level::ipc::IpcModule;

    let mut reactor = Reactor::new()?;
    let ipc_fd = Fd::new(
        unsafe {
            libc::socket(
                libc::AF_UNIX,
                libc::SOCK_STREAM | libc::SOCK_CLOEXEC | libc::SOCK_NONBLOCK,
                0,
            )
        },
        "ipc",
    )?;

    let socket_path = paths::SOCKET_PATH;
    let _ = std::fs::remove_file(socket_path);

    let mut addr: libc::sockaddr_un = unsafe { std::mem::zeroed() };
    addr.sun_family = libc::AF_UNIX as u16;
    let path_bytes = socket_path.as_bytes();
    if path_bytes.len() >= addr.sun_path.len() {
        return Err(crate::low_level::spawn::SysError::sys(
            libc::ENAMETOOLONG,
            "socket_path too long",
        ));
    }
    for (i, &b) in path_bytes.iter().enumerate() {
        addr.sun_path[i] = b as _;
    }

    let ret = unsafe {
        libc::bind(
            ipc_fd.as_raw_fd(),
            &addr as *const libc::sockaddr_un as *const libc::sockaddr,
            std::mem::size_of::<libc::sockaddr_un>() as libc::socklen_t,
        )
    };
    if ret < 0 {
        return Err(crate::low_level::spawn::SysError::sys(
            std::io::Error::last_os_error().raw_os_error().unwrap_or(0),
            "bind(AF_UNIX)",
        ));
    }

    let ret = unsafe { libc::listen(ipc_fd.as_raw_fd(), 128) };
    if ret < 0 {
        let _ = std::fs::remove_file(socket_path);
        return Err(crate::low_level::spawn::SysError::sys(
            std::io::Error::last_os_error().raw_os_error().unwrap_or(0),
            "listen",
        ));
    }

    let ipc_token = Token(1);
    use std::os::unix::io::AsRawFd;
    reactor.add_with_token(ipc_fd.as_raw_fd(), ipc_token, true, false)?;

    let mut state = ExecutionState::new();
    let core = Core::new();
    use crate::high_level::addon::{Addon, AddonSpec, EchoAddon, NoOpAddon};
    use crate::high_level::addons::preload::PreloadAddon;

    let mut addons: Vec<(Box<dyn Addon>, AddonSpec, u32)> = vec![
        (
            Box::new(NoOpAddon),
            AddonSpec {
                id: 100,
                capability: CapabilityToken::empty(),
                max_actions_per_tick: 50,
            },
            0, // Initial error count
        ),
        (
            Box::new(EchoAddon),
            AddonSpec {
                id: 101,
                capability: CapabilityToken::empty(),
                max_actions_per_tick: 50,
            },
            0,
        ),
    ];

    if config.enable_warmup {
        addons.push((
            Box::new(PreloadAddon::new(
                crate::high_level::addons::preload::PreloadConfig::default(),
            )),
            AddonSpec {
                id: 102,
                capability: CapabilityToken::allow_all(),
                max_actions_per_tick: 100,
            },
            0,
        ));
    }

    let mut effect_executor = crate::runtime::EffectExecutor::new(reactor);

    if let Err(e) = std::fs::write(paths::PID_PATH, pid.to_string()) {
        crate::runtime::log_runtime_event(
            crate::core::CORE_OWNER,
            crate::core::LogLevel::Warn,
            crate::core::LogEvent::Generic(format!("failed to write pid file: {}", e)),
        );
    }

    let _ = effect_executor.apply(crate::core::Effect::Log {
        owner: crate::core::CORE_OWNER,
        level: crate::core::LogLevel::Info,
        event: crate::core::LogEvent::Generic(
            "daemon start version=0.1.0 git=a472b4f log_schema=structured_v2".to_string(),
        ),
    });
    let _ = effect_executor.apply(crate::core::Effect::Log {
        owner: crate::core::CORE_OWNER,
        level: crate::core::LogLevel::Info,
        event: crate::core::LogEvent::Generic(format!("socket bound path={}", socket_path)),
    });
    let _ = effect_executor.apply(crate::core::Effect::Log {
        owner: crate::core::CORE_OWNER,
        level: crate::core::LogLevel::Info,
        event: crate::core::LogEvent::Generic("ipc listener ready".to_string()),
    });

    // Load log level from system property if possible (placeholder for Android)
    // effect_executor.log_router.verbosity = LogLevel::Info;

    let mut capabilities = CapabilityRegistry::new();
    capabilities.insert(0, CapabilityToken::allow_all()); // System / IPC root

    // Capability assignment for addons
    for (_, spec, _) in &addons {
        capabilities.insert(spec.id, spec.capability);
    }

    let mut ipc = IpcModule::new(ipc_fd, ipc_token);

    let mut inotify_fd_opt = None;
    if config.enable_warmup
        && let Ok(fd_obj) = effect_executor.reactor.setup_inotify()
    {
        let inotify_fd = fd_obj.raw();

        let cgroup_path = std::ffi::CString::new("/dev/cpuset/top-app/cgroup.procs");
        let pkg_xml_path = std::ffi::CString::new("/data/system/packages.xml");
        let pkg_list_path = std::ffi::CString::new("/data/system/packages.list");

        match (cgroup_path, pkg_xml_path, pkg_list_path) {
            (Ok(cgroup_path), Ok(pkg_xml_path), Ok(pkg_list_path)) => {
                let wd_cgroup = unsafe {
                    libc::inotify_add_watch(
                        inotify_fd,
                        cgroup_path.as_ptr(),
                        libc::IN_CLOSE_WRITE | libc::IN_MODIFY,
                    )
                };

                let wd_pkg_xml = unsafe {
                    libc::inotify_add_watch(
                        inotify_fd,
                        pkg_xml_path.as_ptr(),
                        libc::IN_MODIFY | libc::IN_CREATE | libc::IN_DELETE,
                    )
                };

                let wd_pkg_list = unsafe {
                    libc::inotify_add_watch(
                        inotify_fd,
                        pkg_list_path.as_ptr(),
                        libc::IN_MODIFY | libc::IN_CREATE | libc::IN_DELETE,
                    )
                };

                inotify_fd_opt = Some((fd_obj, wd_cgroup, wd_pkg_xml, wd_pkg_list));
            }
            _ => {
                crate::runtime::log_runtime_event(
                    crate::core::CORE_OWNER,
                    crate::core::LogLevel::Warn,
                    crate::core::LogEvent::Generic(
                        "failed to build inotify watch paths".to_string(),
                    ),
                );
            }
        }
    }

    let mut next_action_id = 1u64;
    let mut trace_store = TraceStore::new();
    let mut pending_events = Vec::new();
    let mut next_events = Vec::new();

    let mut record_file = if let Some(p) = &config.record_path {
        Some(
            std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(p)
                .map_err(|e| {
                    crate::low_level::spawn::SysError::sys(
                        e.raw_os_error().unwrap_or(0),
                        "open record file",
                    )
                })?,
        )
    } else {
        None
    };

    let mut last_tick_time = std::time::Instant::now();
    let mut tick_counter = 0u64;
    let mut reactor_failure_count = 0;

    while RUNNING.load(Ordering::SeqCst) {
        let t0 = std::time::Instant::now();
        tick_counter += 1;
        let elapsed = t0.duration_since(last_tick_time).as_millis() as u64;

        let ticks = elapsed / TICK_MS;

        // Update metrics
        state.metrics.active_clients = ipc.clients.len() as u32;

        let mut sys_events = Vec::new();
        for _ in 0..ticks {
            sys_events.push(crate::core::Event::TimeAdvanced(TICK_MS));
        }

        if ticks > 0 {
            last_tick_time += std::time::Duration::from_millis(ticks * TICK_MS);
        }

        let mut reactor_events = Vec::new();
        let timeout =
            compute_reactor_timeout_ms(core.dispatcher.compute_timeout_ms(&state), elapsed);

        let reactor_res = effect_executor.process_reactor_events(&mut reactor_events, timeout);
        match reactor_res {
            Ok(evs) => {
                sys_events.extend(evs);
                reactor_failure_count = 0;
            }
            Err(e) => {
                reactor_failure_count += 1;
                let _ = effect_executor.apply(crate::core::Effect::Log {
                    owner: crate::core::CORE_OWNER,
                    level: crate::core::LogLevel::Error,
                    event: crate::core::LogEvent::Error {
                        id: 0,
                        err: format!("reactor wait failed: {}", e),
                    },
                });

                if reactor_failure_count >= 10 {
                    match Reactor::new() {
                        Ok(new_reactor) => {
                            effect_executor.reactor = new_reactor;
                            let _ = effect_executor.reactor.add_with_token(
                                ipc.fd.as_raw_fd(),
                                ipc_token,
                                true,
                                false,
                            );
                            ipc.clients.clear();
                            ipc.client_tokens.clear();
                            reactor_failure_count = 0;
                            state.metrics.restart_count += 1;
                        }
                        Err(_) => {
                            std::thread::sleep(std::time::Duration::from_millis(100));
                        }
                    }
                }
            }
        }

        sys_events.append(&mut pending_events);

        let mut scheduler = crate::core::scheduler::Scheduler::new(MAX_ACTIONS_PER_TICK);
        let queue_before = scheduler.total_len;
        state.metrics.queue_depth = queue_before as u32;

        for conn in ipc.clients.values() {
            state.metrics.peak_read_buf_kb = state
                .metrics
                .peak_read_buf_kb
                .max((conn.read_buf.len() / 1024) as u32);
            state.metrics.peak_write_buf_kb = state
                .metrics
                .peak_write_buf_kb
                .max((conn.write_buf.len() / 1024) as u32);
        }

        // Phase 1: Collect
        let mut collected_actions: Vec<(crate::core::Action, crate::core::ActionMeta)> = Vec::new();

        for rev in reactor_events {
            let ipc_msgs = ipc.handle_event(&mut effect_executor.reactor, &rev);
            let mut ipc_intents = Vec::new();
            for msg in ipc_msgs {
                let uid = msg.uid;
                let cmd = msg.command;
                let client_id = msg.client_id;

                let intent_opt = match cmd {
                    crate::high_level::api::Command::Cmd { .. }
                    | crate::high_level::api::Command::Dumpsys { .. } => {
                        let (exec, _policy) = cmd.map_to_exec();
                        let id = next_action_id;
                        next_action_id += 1;
                        Some(crate::core::Intent::Submit {
                            id,
                            owner: 0,
                            job: crate::core::JobRequest { command: exec.argv },
                        })
                    }
                    crate::high_level::api::Command::GetResult { id } => {
                        Some(crate::core::Intent::Query { id })
                    }
                    crate::high_level::api::Command::Cancel { id } => {
                        Some(crate::core::Intent::Control {
                            id,
                            signal: crate::core::ControlSignal::GracefulStop,
                        })
                    }
                };

                if let Some(mut intent) = intent_opt {
                    let cause = crate::core::CauseId(next_action_id);
                    next_action_id += 1;
                    trace_store.insert(cause, None);

                    if let crate::core::Intent::Submit { ref mut id, .. } = intent {
                        *id = cause.0;
                    }

                    ipc_intents.push(crate::high_level::identity::Request {
                        principal: crate::high_level::identity::Principal::new_user(uid),
                        client_id: Some(client_id),
                        cause,
                        intent,
                    });
                }
            }

            let mut addon_reqs = Vec::new();
            for (addon, spec, error_count) in &mut addons {
                if *error_count >= 10 {
                    continue;
                }
                let reqs = addon.on_reactor_event(&state, &rev);
                let count = std::cmp::min(reqs.len(), spec.max_actions_per_tick as usize);
                for mut req in reqs.into_iter().take(count) {
                    let cause = crate::core::CauseId(next_action_id);
                    next_action_id += 1;
                    trace_store.insert(cause, None);

                    if let crate::core::Intent::Submit { ref mut id, .. } = req.intent {
                        *id = cause.0;
                    }

                    req.cause = cause;
                    req.principal = crate::high_level::identity::Principal::Addon(spec.id);
                    req.client_id = None;
                    addon_reqs.push(req);
                }
            }

            let all_reqs = ipc_intents.into_iter().chain(addon_reqs);

            for req in all_reqs {
                if crate::core::validation::validate_request(&req, &state).is_ok() {
                    let actions = crate::core::expand_intent(req.intent.clone(), state.clock);
                    let mut allowed = true;
                    for action in &actions {
                        if !capabilities.allows(&req.principal, action.kind()) {
                            allowed = false;
                            break;
                        }
                    }

                    if allowed {
                        if let Some(f) = &mut record_file {
                            let _ = bincode::serialize_into(
                                f,
                                &crate::core::replay::ReplayInput::Intent(
                                    req.principal.clone(),
                                    req.intent.clone(),
                                ),
                            );
                        }
                        for action in actions {
                            collected_actions.push((
                                action,
                                crate::core::ActionMeta {
                                    id: req.cause,
                                    parent: Some(req.cause),
                                    source: req.principal.clone(),
                                    reply_to: req.client_id,
                                },
                            ));
                        }
                    }
                }
            }

            if let Some((inotify_fd_obj, wd_cgroup, wd_pkg_xml, wd_pkg_list)) = &inotify_fd_opt
                && Some(rev.token) == effect_executor.reactor.inotify_token
                && rev.readable
            {
                let mut fds = libc::pollfd {
                    fd: inotify_fd_obj.raw(),
                    events: libc::POLLIN,
                    revents: 0,
                };

                let ret = unsafe { libc::poll(&mut fds, 1, 0) };
                if ret > 0 {
                    let mut len: libc::c_int = 0;
                    if unsafe { libc::ioctl(inotify_fd_obj.raw(), libc::FIONREAD, &mut len) } >= 0
                        && len > 0
                    {
                        let mut buf = vec![0u8; len as usize];
                        let n = unsafe {
                            libc::read(
                                inotify_fd_obj.raw(),
                                buf.as_mut_ptr() as *mut libc::c_void,
                                len as usize,
                            )
                        };

                        if n > 0 {
                            let mut offset = 0;
                            let mut cgroup_changed = false;
                            let mut packages_changed = false;
                            let base = std::mem::size_of::<libc::inotify_event>();

                            while offset + base <= n as usize {
                                let event = unsafe {
                                    &*(buf.as_ptr().add(offset) as *const libc::inotify_event)
                                };
                                let size = base + event.len as usize;

                                if offset + size > n as usize {
                                    break;
                                }

                                if event.wd == *wd_pkg_xml || event.wd == *wd_pkg_list {
                                    packages_changed = true;
                                } else if event.wd == *wd_cgroup {
                                    cgroup_changed = true;
                                }

                                offset += size;
                            }

                            if packages_changed {
                                sys_events.push(crate::core::Event::PackagesChanged);
                            }

                            if cgroup_changed
                                && let Ok(cgroup_content) =
                                    std::fs::read_to_string("/dev/cpuset/top-app/cgroup.procs")
                            {
                                for pid_str in cgroup_content.split_whitespace() {
                                    if let Ok(pid) = pid_str.parse::<i32>() {
                                        sys_events
                                            .push(crate::core::Event::ForegroundChanged { pid });
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        sys_events.push(crate::core::Event::Tick);

        while !sys_events.is_empty() || !collected_actions.is_empty() {
            if !sys_events.is_empty() {
                let current_events = std::mem::take(&mut sys_events);
                for ev in current_events {
                    if let Some(f) = &mut record_file {
                        let _ = bincode::serialize_into(
                            f,
                            &crate::core::replay::ReplayInput::Event(ev.clone()),
                        );
                    }

                    // Feed core events to addons
                    for (addon, spec, error_count) in &mut addons {
                        if *error_count >= 10 {
                            continue;
                        }
                        let reqs = addon.on_core_event(&state, &ev);
                        for mut req in reqs {
                            let cause = crate::core::CauseId(next_action_id);
                            next_action_id += 1;
                            trace_store.insert(cause, None);
                            if let crate::core::Intent::Submit { ref mut id, .. } = req.intent {
                                *id = cause.0;
                            }
                            req.cause = cause;
                            req.principal = crate::high_level::identity::Principal::Addon(spec.id);

                            if crate::core::validation::validate_request(&req, &state).is_ok() {
                                let actions = crate::core::expand_intent(req.intent, state.clock);
                                for action in actions {
                                    if capabilities.allows(&req.principal, action.kind()) {
                                        collected_actions.push((
                                            action,
                                            crate::core::ActionMeta {
                                                id: req.cause,
                                                parent: Some(req.cause),
                                                source: req.principal.clone(),
                                                reply_to: None,
                                            },
                                        ));
                                    }
                                }
                            }
                        }
                    }

                    let sys_actions = core.dispatcher.dispatch_event(&state, &ev);
                    for action in sys_actions {
                        let cause = crate::core::CauseId(next_action_id);
                        next_action_id += 1;
                        trace_store.insert(cause, None);
                        collected_actions.push((
                            action,
                            crate::core::ActionMeta {
                                id: cause,
                                parent: None,
                                source: crate::high_level::identity::Principal::System,
                                reply_to: None,
                            },
                        ));
                    }
                }
            }

            if !collected_actions.is_empty() {
                let current_actions = std::mem::take(&mut collected_actions);
                for (action, meta) in current_actions {
                    if !capabilities.allows(&meta.source, action.kind()) {
                        continue;
                    }

                    let next_id = crate::core::CauseId(next_action_id);
                    trace_store.insert(next_id, Some(meta.id));

                    let new_meta = crate::core::ActionMeta {
                        id: next_id,
                        parent: Some(meta.id),
                        source: meta.source.clone(),
                        reply_to: meta.reply_to,
                    };
                    if let Some(ev) = scheduler.enqueue(
                        crate::core::RoutedAction {
                            action,
                            meta: new_meta,
                        },
                        &mut state,
                    ) {
                        sys_events.push(ev);
                    }
                    next_action_id += 1;
                }
            }
        }

        if tick_counter.is_multiple_of(64) {
            // Check log verbosity trigger files
            let new_verbosity = if std::path::Path::new(paths::LOG_TRACE_PATH).exists() {
                crate::core::LogLevel::Trace
            } else if std::path::Path::new(paths::LOG_DEBUG_PATH).exists() {
                crate::core::LogLevel::Debug
            } else {
                crate::core::LogLevel::Info
            };

            if new_verbosity != effect_executor.log_router.verbosity {
                let _ = effect_executor.apply(crate::core::Effect::Log {
                    owner: crate::core::CORE_OWNER,
                    level: crate::core::LogLevel::Info,
                    event: crate::core::LogEvent::Generic(format!(
                        "log verbosity changed level={:?}",
                        new_verbosity
                    )),
                });
                effect_executor.log_router.verbosity = new_verbosity;
            }

            if let Err(err) = crate::core::verify::verify_global(&state) {
                let _ = effect_executor.apply(crate::core::Effect::Log {
                    owner: crate::core::CORE_OWNER,
                    level: crate::core::LogLevel::Error,
                    event: crate::core::LogEvent::Generic(format!(
                        "state verification failed err={}",
                        err
                    )),
                });
            }
        }

        // Phase 4: Resolve
        let mut generated_effects = Vec::with_capacity(16);
        let mut action_effects = Vec::with_capacity(16);
        let mut tick_actions_processed = 0;
        let mut tick_dropped_actions = 0;
        let mut per_source_count: std::collections::HashMap<
            crate::high_level::identity::Principal,
            usize,
        > = std::collections::HashMap::new();

        loop {
            let mut made_progress = false;

            while let Some(routed) = scheduler.pop_next() {
                let count = per_source_count
                    .entry(routed.meta.source.clone())
                    .or_insert(0);
                if *count >= 256 {
                    tick_dropped_actions += 1;
                    continue;
                }
                *count += 1;

                ipc.intercept_action(&routed.action, routed.meta.reply_to);
                tick_actions_processed += 1;

                if let crate::core::Action::HandleAddonFailure { addon_id, .. } = routed.action {
                    for (_, spec, error_count) in &mut addons {
                        if spec.id == addon_id {
                            *error_count += 1;
                        }
                    }
                }

                // Structured Action Log
                let payload_len = match &routed.action {
                    crate::core::Action::SystemRequest { payload, .. } => payload.len(),
                    crate::core::Action::AddonTask { payload, .. } => payload.len(),
                    _ => 0,
                };

                let log_id = match &routed.action {
                    crate::core::Action::Submit { id, .. } => Some(*id),
                    crate::core::Action::SystemRequest { request_id, .. } => Some(*request_id),
                    _ => None,
                };

                let log_addon = match &routed.action {
                    crate::core::Action::AddonTask { addon_id, .. } => Some(*addon_id),
                    crate::core::Action::AddonLog { addon_id, .. } => Some(*addon_id),
                    _ => None,
                };

                let log_key = match &routed.action {
                    crate::core::Action::AddonTask { key, .. } => Some(key.clone()),
                    _ => None,
                };

                let log_svc = match &routed.action {
                    crate::core::Action::SystemRequest { kind, .. } => Some(*kind),
                    _ => None,
                };

                let _ = effect_executor.apply(crate::core::Effect::Log {
                    owner: crate::core::CORE_OWNER,
                    level: crate::core::LogLevel::Trace,
                    event: crate::core::LogEvent::ActionDispatch {
                        kind: routed.action.kind(),
                        id: log_id,
                        addon_id: log_addon,
                        key: log_key,
                        service: log_svc,
                        payload_len,
                    },
                });

                action_effects.clear();

                if let Some(indices) = core.routing.get(&routed.action.kind()) {
                    for &idx in indices {
                        let reducer = &core.reducers[idx];
                        let mut ctx = crate::core::reducer::ReducerCtx {
                            core: &mut state.core,
                            timeout: &mut state.timeout,
                            result: &mut state.result,
                            clock: &mut state.clock,
                        };
                        reducer.apply(&mut ctx, &routed.action, &mut action_effects);
                    }
                }

                for effect in action_effects.drain(..) {
                    generated_effects.push(effect);
                }
                state.update_hash();

                let new_actions = core.dispatcher.dispatch(&state, &routed.action);
                for action in new_actions {
                    let next_id = crate::core::CauseId(next_action_id);
                    trace_store.insert(next_id, Some(routed.meta.id));
                    if let Some(ev) = scheduler.enqueue(
                        crate::core::RoutedAction {
                            action,
                            meta: crate::core::ActionMeta {
                                id: next_id,
                                parent: Some(routed.meta.id),
                                source: routed.meta.source.clone(),
                                reply_to: routed.meta.reply_to,
                            },
                        },
                        &mut state,
                    ) {
                        sys_events.push(ev);
                    }
                    next_action_id += 1;
                }
            }

            if !sys_events.is_empty() || !collected_actions.is_empty() {
                made_progress = true;
                while !sys_events.is_empty() || !collected_actions.is_empty() {
                    if !sys_events.is_empty() {
                        let current_events = std::mem::take(&mut sys_events);
                        for ev in current_events {
                            if let Some(f) = &mut record_file {
                                let _ = bincode::serialize_into(
                                    f,
                                    &crate::core::replay::ReplayInput::Event(ev.clone()),
                                );
                            }
                            let sys_actions = core.dispatcher.dispatch_event(&state, &ev);
                            for action in sys_actions {
                                let cause = crate::core::CauseId(next_action_id);
                                next_action_id += 1;
                                trace_store.insert(cause, None);
                                collected_actions.push((
                                    action,
                                    crate::core::ActionMeta {
                                        id: cause,
                                        parent: None,
                                        source: crate::high_level::identity::Principal::System,
                                        reply_to: None,
                                    },
                                ));
                            }
                        }
                    }

                    if !collected_actions.is_empty() {
                        let current_actions = std::mem::take(&mut collected_actions);
                        for (action, meta) in current_actions {
                            if !capabilities.allows(&meta.source, action.kind()) {
                                continue;
                            }
                            let next_id = crate::core::CauseId(next_action_id);
                            next_action_id += 1;
                            trace_store.insert(next_id, Some(meta.id));

                            if let Some(ev) = scheduler.enqueue(
                                crate::core::RoutedAction {
                                    action,
                                    meta: crate::core::ActionMeta {
                                        id: next_id,
                                        parent: Some(meta.id),
                                        source: meta.source.clone(),
                                        reply_to: meta.reply_to,
                                    },
                                },
                                &mut state,
                            ) {
                                sys_events.push(ev);
                            }
                        }
                    }
                }
            }

            if !made_progress {
                break;
            }
        }

        if tick_counter.is_multiple_of(640) {
            let m = &state.metrics;
            let _ = effect_executor.apply(crate::core::Effect::Log {
                owner: crate::core::CORE_OWNER,
                level: crate::core::LogLevel::Info,
                event: crate::core::LogEvent::Generic(format!("METRICS: clients={} dropped={} queue={} avg_tick_us={} peak_r_kb={} peak_w_kb={}",
                        m.active_clients, m.dropped_actions, m.queue_depth, m.avg_tick_duration_us, m.peak_read_buf_kb, m.peak_write_buf_kb)),
            });
        }

        for effect in generated_effects {
            let events = effect_executor.apply(effect);
            next_events.extend(events);
        }

        std::mem::swap(&mut pending_events, &mut next_events);
        next_events.clear();

        let tick_duration_us = t0.elapsed().as_micros() as u64;
        state.metrics.avg_tick_duration_us =
            (state.metrics.avg_tick_duration_us * 7 + tick_duration_us as u32) / 8;

        let _ = effect_executor.apply(crate::core::Effect::Log {
            owner: crate::core::CORE_OWNER,
            level: crate::core::LogLevel::Debug,
            event: crate::core::LogEvent::TickSummary {
                processed: tick_actions_processed,
                dropped: tick_dropped_actions,
                queue_before,
                queue_after: scheduler.total_len,
                elapsed_us: tick_duration_us,
            },
        });

        if let Some(f) = &mut record_file {
            let stats = crate::core::replay::TickStats {
                hash: state.hash,
                actions_processed: tick_actions_processed,
                dropped_actions: tick_dropped_actions,
            };
            let _ = bincode::serialize_into(f, &crate::core::replay::ReplayInput::TickEnd(stats));
        }
    }

    crate::runtime::log_runtime_event(
        crate::core::CORE_OWNER,
        crate::core::LogLevel::Info,
        crate::core::LogEvent::Generic("daemon shutting down".to_string()),
    );
    let _ = std::fs::remove_file(socket_path);
    let _ = std::fs::remove_file(paths::PID_PATH);
    Ok(())
}

pub fn run_replay(path: &str) -> Result<u64, crate::low_level::spawn::SysError> {
    use crate::core::replay::ReplayInput;
    use crate::core::{Core, ExecutionState};
    use std::fs::File;
    use std::io::BufReader;

    let mut state = ExecutionState::new();
    let core = Core::new();
    let mut scheduler = crate::core::scheduler::Scheduler::new(10_000);
    let mut trace_store = TraceStore::new();
    let mut next_action_id = 1u64;

    let file = File::open(path).map_err(|e| {
        crate::low_level::spawn::SysError::sys(e.raw_os_error().unwrap_or(0), "open replay file")
    })?;
    let mut reader = BufReader::new(file);

    let mut inputs = Vec::new();
    while let Ok(input) = bincode::deserialize_from::<_, ReplayInput>(&mut reader) {
        inputs.push(input);
    }

    let mut tick_idx = 0;
    let mut current_input_idx = 0;

    let mut action_effects = Vec::with_capacity(16);

    while current_input_idx < inputs.len() {
        let mut tick_events = Vec::new();
        let mut tick_intents: Vec<crate::high_level::identity::Request> = Vec::new();
        let mut expected_hash = None;

        while current_input_idx < inputs.len() {
            match &inputs[current_input_idx] {
                ReplayInput::Time(dur) => {
                    let elapsed = dur.as_millis() as u64;
                    let ticks = elapsed / 16;
                    for _ in 0..ticks {
                        tick_events.push(crate::core::Event::TimeAdvanced(16));
                    }
                }
                ReplayInput::TickHash(h) => {
                    expected_hash = Some(crate::core::replay::TickStats {
                        hash: *h,
                        actions_processed: 0,
                        dropped_actions: 0,
                    });
                    current_input_idx += 1;
                    break;
                }
                ReplayInput::TickEnd(stats) => {
                    expected_hash = Some(stats.clone());
                    current_input_idx += 1;
                    break;
                }
                ReplayInput::Event(e) => {
                    tick_events.push(e.clone());
                }
                ReplayInput::LegacyIntent(i) => {
                    let cause = crate::core::CauseId(next_action_id);
                    next_action_id += 1;
                    trace_store.insert(cause, None);

                    let mut cloned_intent = i.clone();
                    if let crate::core::Intent::Submit { ref mut id, .. } = cloned_intent {
                        *id = cause.0;
                    }

                    tick_intents.push(crate::high_level::identity::Request {
                        principal: crate::high_level::identity::Principal::System,
                        client_id: None,
                        cause,
                        intent: cloned_intent,
                    });
                }
                ReplayInput::Intent(p, i) => {
                    let cause = crate::core::CauseId(next_action_id);
                    next_action_id += 1;
                    trace_store.insert(cause, None);

                    let mut cloned_intent = i.clone();
                    if let crate::core::Intent::Submit { ref mut id, .. } = cloned_intent {
                        *id = cause.0;
                    }

                    tick_intents.push(crate::high_level::identity::Request {
                        principal: p.clone(),
                        client_id: None,
                        cause,
                        intent: cloned_intent,
                    });
                }
            }
            current_input_idx += 1;
        }

        for req in tick_intents {
            let actions = crate::core::expand_intent(req.intent, state.clock);
            for action in actions {
                let next_id = crate::core::CauseId(next_action_id);
                trace_store.insert(next_id, None);
                if let Some(ev) = scheduler.enqueue(
                    crate::core::RoutedAction {
                        action,
                        meta: crate::core::ActionMeta {
                            id: next_id,
                            parent: None,
                            source: req.principal.clone(),
                            reply_to: req.client_id,
                        },
                    },
                    &mut state,
                ) {
                    tick_events.push(ev);
                }
                next_action_id += 1;
            }
        }

        let mut collected_actions: Vec<crate::core::Action> = Vec::new();
        while !tick_events.is_empty() || !collected_actions.is_empty() {
            if !tick_events.is_empty() {
                let current_events = std::mem::take(&mut tick_events);
                for event in current_events {
                    let actions = core.dispatcher.dispatch_event(&state, &event);
                    for action in actions {
                        collected_actions.push(action);
                    }
                }
            }

            if !collected_actions.is_empty() {
                let current_actions = std::mem::take(&mut collected_actions);
                for action in current_actions {
                    let next_id = crate::core::CauseId(next_action_id);
                    trace_store.insert(next_id, None);
                    if let Some(ev) = scheduler.enqueue(
                        crate::core::RoutedAction {
                            action,
                            meta: crate::core::ActionMeta {
                                id: next_id,
                                parent: None,
                                source: crate::high_level::identity::Principal::System,
                                reply_to: None,
                            },
                        },
                        &mut state,
                    ) {
                        tick_events.push(ev);
                    }
                    next_action_id += 1;
                }
            }
        }

        let mut tick_actions_processed = 0;
        let mut tick_dropped_actions = 0;
        let mut per_source_count: std::collections::HashMap<
            crate::high_level::identity::Principal,
            usize,
        > = std::collections::HashMap::new();

        loop {
            let mut made_progress = false;

            while let Some(routed) = scheduler.pop_next() {
                let count = per_source_count
                    .entry(routed.meta.source.clone())
                    .or_insert(0);
                if *count >= 256 {
                    tick_dropped_actions += 1;
                    continue;
                }
                *count += 1;
                tick_actions_processed += 1;

                action_effects.clear();

                if let Some(indices) = core.routing.get(&routed.action.kind()) {
                    for &idx in indices {
                        let reducer = &core.reducers[idx];
                        let mut ctx = crate::core::reducer::ReducerCtx {
                            core: &mut state.core,
                            timeout: &mut state.timeout,
                            result: &mut state.result,
                            clock: &mut state.clock,
                        };
                        reducer.apply(&mut ctx, &routed.action, &mut action_effects);
                    }
                }
                state.update_hash();

                let new_actions = core.dispatcher.dispatch(&state, &routed.action);
                for action in new_actions {
                    let next_id = crate::core::CauseId(next_action_id);
                    trace_store.insert(next_id, Some(routed.meta.id));
                    if let Some(ev) = scheduler.enqueue(
                        crate::core::RoutedAction {
                            action,
                            meta: crate::core::ActionMeta {
                                id: next_id,
                                parent: Some(routed.meta.id),
                                source: routed.meta.source.clone(),
                                reply_to: routed.meta.reply_to,
                            },
                        },
                        &mut state,
                    ) {
                        tick_events.push(ev);
                    }
                    next_action_id += 1;
                }
            }

            if !tick_events.is_empty() || !collected_actions.is_empty() {
                made_progress = true;
                while !tick_events.is_empty() || !collected_actions.is_empty() {
                    if !tick_events.is_empty() {
                        let current_events = std::mem::take(&mut tick_events);
                        for event in current_events {
                            let actions = core.dispatcher.dispatch_event(&state, &event);
                            for action in actions {
                                collected_actions.push(action);
                            }
                        }
                    }

                    if !collected_actions.is_empty() {
                        let current_actions = std::mem::take(&mut collected_actions);
                        for action in current_actions {
                            let next_id = crate::core::CauseId(next_action_id);
                            trace_store.insert(next_id, None);
                            if let Some(ev) = scheduler.enqueue(
                                crate::core::RoutedAction {
                                    action,
                                    meta: crate::core::ActionMeta {
                                        id: next_id,
                                        parent: None,
                                        source: crate::high_level::identity::Principal::System,
                                        reply_to: None,
                                    },
                                },
                                &mut state,
                            ) {
                                tick_events.push(ev);
                            }
                            next_action_id += 1;
                        }
                    }
                }
            }

            if !made_progress {
                break;
            }
        }

        if let Some(expected) = expected_hash {
            let actual = state.hash;
            assert_eq!(
                actual, expected.hash,
                "Determinism violation: state hash diverged at tick {}",
                tick_idx
            );
            if expected.actions_processed > 0 || expected.dropped_actions > 0 {
                assert_eq!(
                    tick_actions_processed, expected.actions_processed,
                    "Determinism violation: actions processed diverged at tick {}",
                    tick_idx
                );
                assert_eq!(
                    tick_dropped_actions, expected.dropped_actions,
                    "Determinism violation: dropped actions diverged at tick {}",
                    tick_idx
                );
            }
            tick_idx += 1;
        }
    }

    crate::runtime::log_runtime_event(
        crate::core::CORE_OWNER,
        crate::core::LogLevel::Info,
        crate::core::LogEvent::Generic("replay finished deterministically".to_string()),
    );
    Ok(state.hash)
}
