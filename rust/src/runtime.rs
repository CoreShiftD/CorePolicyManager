use crate::arena::Arena;
use crate::core::{ControlSignal, Effect, Event, IoHandle, LogEvent, LogLevel, SystemService};
use crate::low_level::io::DrainState;
use crate::low_level::reactor::{Event as ReactorEvent, Reactor};
use crate::low_level::spawn::{Process, SpawnBackend, SpawnOptions, SysError, spawn_start};
use crate::low_level::sys::{ExecContext, ProcessGroup};
use std::collections::HashMap;

#[cfg(target_os = "linux")]
unsafe extern "C" {
    fn readahead(fd: libc::c_int, offset: libc::off64_t, count: libc::size_t) -> libc::ssize_t;
}

#[inline(always)]
unsafe fn do_readahead(fd: libc::c_int) {
    if fd < 0 {
        return;
    }

    #[cfg(target_os = "android")]
    unsafe {
        libc::syscall(libc::SYS_readahead, fd, 0, 0);
    }

    #[cfg(all(target_os = "linux", not(target_os = "android")))]
    unsafe {
        readahead(fd, 0, 0);
    }
}

pub struct FileSink {
    file: std::fs::File,
}

impl FileSink {
    pub fn new(path: &str) -> Self {
        use std::fs::OpenOptions;
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .unwrap_or_else(|_| std::fs::File::create("/dev/null").unwrap());
        Self { file }
    }

    pub fn write(&mut self, level: LogLevel, msg: String) {
        use std::io::Write;
        let _ = writeln!(self.file, "[{:?}] {}", level, msg);
    }
}

pub struct LogRouter {
    sinks: HashMap<u32, FileSink>,
    default: FileSink,
    pub verbosity: LogLevel,
}

impl Default for LogRouter {
    fn default() -> Self {
        Self::new()
    }
}

impl LogRouter {
    pub fn new() -> Self {
        Self {
            sinks: HashMap::new(),
            default: FileSink::new(crate::paths::CORE_LOG_PATH),
            verbosity: LogLevel::Info,
        }
    }

    fn get_or_create(&mut self, owner: u32) -> &mut FileSink {
        self.sinks.entry(owner).or_insert_with(|| {
            let path = crate::paths::addon_log_path(owner);
            FileSink::new(&path)
        })
    }

    pub fn write(&mut self, owner: u32, level: LogLevel, msg: String) {
        if (level as u8) < (self.verbosity as u8) {
            return;
        }
        if owner == crate::core::CORE_OWNER {
            self.default.write(level, msg);
        } else {
            self.get_or_create(owner).write(level, msg);
        }
    }
}

pub struct RuntimeProcess {
    pub process: Process,
    pub is_group: bool,
}

pub struct RuntimeDrain {
    pub drain: DrainState<fn(&[u8]) -> bool>,
}

pub struct EffectExecutor {
    pub reactor: Reactor,
    pub fd_map: Vec<Option<(IoHandle, crate::core::IoStream)>>,
    pub processes: Arena<RuntimeProcess>,
    pub drains: Arena<RuntimeDrain>,
    pub log_router: LogRouter,
}

impl EffectExecutor {
    pub fn new(reactor: Reactor) -> Self {
        Self {
            reactor,
            fd_map: Vec::new(),
            processes: Arena::new(),
            drains: Arena::new(),
            log_router: LogRouter::new(),
        }
    }

    pub fn process_reactor_events(
        &mut self,
        events: &mut Vec<ReactorEvent>,
        timeout_ms: i32,
    ) -> Result<Vec<Event>, SysError> {
        let nevents = self.reactor.wait(events, 64, timeout_ms)?;
        let mut sys_events = Vec::new();
        for ev in events.iter().take(nevents) {
            let idx = ev.token.0 as usize;
            if idx < self.fd_map.len()
                && let Some((io, stream)) = self.fd_map[idx] {
                    sys_events.push(Event::IoReady {
                        io,
                        stream,
                        readable: ev.readable,
                        writable: ev.writable,
                        error: ev.error,
                    });
            }
        }
        Ok(sys_events)
    }

    fn format_log_event(&self, event: &LogEvent) -> String {
        match event {
            LogEvent::TickSummary { processed, dropped, queue_before, queue_after, elapsed_us } => {
                format!("tick processed={} dropped={} queue_before={} queue_after={} elapsed_ms={}",
                    processed, dropped, queue_before, queue_after, elapsed_us / 1000)
            }
            LogEvent::ActionDispatch { kind, id, addon_id, key, service, payload_len } => {
                let mut parts = vec![format!("action={:?}", kind)];
                if let Some(i) = id { parts.push(format!("id={}", i)); }
                if let Some(a) = addon_id { parts.push(format!("addon_id={}", a)); }
                if let Some(k) = key { parts.push(format!("key={}", k)); }
                if let Some(s) = service { parts.push(format!("service={:?}", s)); }
                if *payload_len > 0 { parts.push(format!("payload_len={}", payload_len)); }
                parts.join(" ")
            }
            LogEvent::PreloadForeground { pid, package } => {
                format!("preload foreground pid={} package={}", pid, package)
            }
            LogEvent::PreloadSkip { package, reason, remaining_ms } => {
                let mut s = format!("preload skip package={} reason={}", package, reason);
                if let Some(r) = remaining_ms { s.push_str(&format!(" remaining_ms={}", r)); }
                s
            }
            LogEvent::PreloadStart { package, paths } => {
                format!("preload start package={} paths={}", package, paths)
            }
            LogEvent::PreloadDone { package, paths, bytes, duration_ms } => {
                format!("preload done package={} paths={} bytes={} duration_ms={}", package, paths, bytes, duration_ms)
            }
            LogEvent::PreloadFail { package, reason, backoff_ms } => {
                format!("preload fail package={} reason={} backoff_ms={}", package, reason, backoff_ms)
            }
            LogEvent::Generic(s) => s.clone(),
            LogEvent::Error { id, err } => format!("Error id={}, err={}", id, err),
        }
    }

    pub fn apply(&mut self, effect: Effect) -> Vec<Event> {
        match effect {
            Effect::Log { owner, level, event } => {
                let msg = self.format_log_event(&event);
                self.log_router.write(owner, level, msg);
                vec![]
            }
            Effect::WatchStream { io, stream } => {
                let fd_ref = if let Some(rdrain) = self.drains.get(io.index, io.generation) {
                    match stream {
                        crate::core::IoStream::Stdout => {
                            rdrain.drain.stdout_slot.as_ref().map(|s| &s.fd)
                        }
                        crate::core::IoStream::Stderr => {
                            rdrain.drain.stderr_slot.as_ref().map(|s| &s.fd)
                        }
                        crate::core::IoStream::Stdin => {
                            rdrain.drain.stdin_slot.as_ref().map(|s| &s.fd)
                        }
                    }
                } else {
                    return vec![Event::WatchStreamFailed {
                        io,
                        err: "drain not found".to_string(),
                    }];
                };

                if let Some(fd) = fd_ref {
                    match self.reactor.add(fd, true, true) {
                        Ok(token) => {
                            let idx = token.0 as usize;
                            if idx >= self.fd_map.len() {
                                self.fd_map.resize(idx + 1, None);
                            }
                            self.fd_map[idx] = Some((io, stream));
                        }
                        Err(e) => {
                            return vec![Event::WatchStreamFailed {
                                io,
                                err: format!("reactor add failed: {}", e),
                            }];
                        }
                    }
                } else {
                    return vec![Event::WatchStreamFailed {
                        io,
                        err: "stream fd not available".to_string(),
                    }];
                }
                vec![]
            }
            Effect::UnwatchStream { io, stream } => {
                if let Some(rdrain) = self.drains.get(io.index, io.generation) {
                    let raw_fd = match stream {
                        crate::core::IoStream::Stdout => {
                            rdrain.drain.stdout_slot.as_ref().map(|s| s.fd.raw())
                        }
                        crate::core::IoStream::Stderr => {
                            rdrain.drain.stderr_slot.as_ref().map(|s| s.fd.raw())
                        }
                        crate::core::IoStream::Stdin => {
                            rdrain.drain.stdin_slot.as_ref().map(|s| s.fd.raw())
                        }
                    };

                    if let Some(fd) = raw_fd {
                        self.reactor.del_raw(fd);
                        for slot in self.fd_map.iter_mut() {
                            if let Some((mapped_io, mapped_stream)) = slot
                                && *mapped_io == io && *mapped_stream == stream {
                                    *slot = None;
                            }
                        }
                    } else {
                        return vec![Event::WatchStreamFailed {
                            io,
                            err: "unwatch: stream fd not available".to_string(),
                        }];
                    }
                } else {
                    return vec![Event::WatchStreamFailed {
                        io,
                        err: "unwatch: drain not found".to_string(),
                    }];
                }
                vec![]
            }
            Effect::StartProcess { id, exec, policy } => {
                let ctx = ExecContext::new(exec.argv, None, None);
                let stdin_buf = exec.stdin.map(|v| v.into_boxed_slice());

                let is_group = false; 
                let pgroup = ProcessGroup::default();

                let opts = SpawnOptions {
                    ctx,
                    stdin: stdin_buf,
                    capture_stdout: exec.capture_stdout,
                    capture_stderr: exec.capture_stderr,
                    wait: false,
                    pgroup,
                    max_output: exec.max_output,
                    timeout_ms: None,
                    kill_grace_ms: policy.kill_grace_ms,
                    cancel: match policy.cancel {
                        crate::core::CancelPolicy::None => {
                            crate::low_level::sys::CancelPolicy::None
                        }
                        crate::core::CancelPolicy::Graceful => {
                            crate::low_level::sys::CancelPolicy::Graceful
                        }
                        crate::core::CancelPolicy::Kill => {
                            crate::low_level::sys::CancelPolicy::Kill
                        }
                    },
                    backend: SpawnBackend::Auto,
                    early_exit: None,
                };

                match spawn_start(id, opts) {
                    Ok(running) => {
                        let (p_idx, p_gen) = self.processes.insert(RuntimeProcess {
                            process: running.process,
                            is_group,
                        });
                        let proc_h = crate::core::Handle {
                            index: p_idx,
                            generation: p_gen,
                            _marker: std::marker::PhantomData,
                        };

                        let (d_idx, d_gen) = self.drains.insert(RuntimeDrain {
                            drain: running.drain,
                        });
                        let io_h = crate::core::Handle {
                            index: d_idx,
                            generation: d_gen,
                            _marker: std::marker::PhantomData,
                        };

                        vec![Event::ProcessStarted {
                            id,
                            process: proc_h,
                            io: io_h,
                        }]
                    }
                    Err(e) => {
                        vec![Event::ProcessSpawnFailed {
                            id,
                            err: format!("spawn_failed: {}", e),
                        }]
                    }
                }
            }
            Effect::KillProcess { process, signal } => {
                if let Some(rproc) = self.processes.get_mut(process.index, process.generation) {
                    let res = match signal {
                        ControlSignal::GracefulStop => {
                            if rproc.is_group {
                                rproc.process.kill_pgroup(libc::SIGTERM)
                            } else {
                                rproc.process.kill(libc::SIGTERM)
                            }
                        }
                        ControlSignal::ForceKill => {
                            if rproc.is_group {
                                rproc.process.kill_pgroup(libc::SIGKILL)
                            } else {
                                rproc.process.kill(libc::SIGKILL)
                            }
                        }
                    };
                    if let Err(e) = res {
                        return vec![Event::KillProcessFailed {
                            process,
                            err: format!("kill failed: {}", e),
                        }];
                    }
                } else {
                    return vec![Event::KillProcessFailed {
                        process,
                        err: "process not found".to_string(),
                    }];
                }
                vec![]
            }
            Effect::PollProcess { process } => {
                if let Some(rproc) = self.processes.get_mut(process.index, process.generation) {
                    let status_res = rproc.process.wait_step();
                    match status_res {
                        Ok(Some(status)) => {
                            let s = match status {
                                crate::low_level::spawn::ExitStatus::Exited(c) => c,
                                crate::low_level::spawn::ExitStatus::Signaled(sig) => -sig,
                            };
                            vec![Event::ProcessExited {
                                process,
                                status: Some(s),
                            }]
                        }
                        Ok(None) => vec![],
                        Err(e) => {
                            vec![Event::KillProcessFailed {
                                process,
                                err: format!("poll wait_step failed: {}", e),
                            }]
                        }
                    }
                } else {
                    vec![Event::KillProcessFailed {
                        process,
                        err: "process not found for polling".to_string(),
                    }]
                }
            }
            Effect::PerformIo { io } => {
                if let Some(rdrain) = self.drains.get_mut(io.index, io.generation) {
                    let mut closed = false;
                    let mut err_reason = None;
                    if rdrain.drain.stdout_slot.is_some() {
                        match rdrain.drain.read_fd(true) {
                            Ok(Some(_)) => {}
                            Ok(None) => closed = true,
                            Err(e) => {
                                closed = true;
                                err_reason = Some(format!("stdout read: {}", e));
                            }
                        }
                    }
                    if rdrain.drain.stderr_slot.is_some() {
                        match rdrain.drain.read_fd(false) {
                            Ok(Some(_)) => {}
                            Ok(None) => closed = true,
                            Err(e) => {
                                closed = true;
                                err_reason = Some(format!("stderr read: {}", e));
                            }
                        }
                    }
                    if rdrain.drain.stdin_slot.is_some() {
                        match rdrain.drain.write_stdin() {
                            Ok(Some(_)) => {}
                            Ok(None) => closed = true,
                            Err(e) => {
                                closed = true;
                                err_reason = Some(format!("stdin write: {}", e));
                            }
                        }
                    }

                    if let Some(reason) = err_reason {
                        return vec![Event::IoFailed { io, reason }];
                    } else if closed {
                        return vec![Event::IoClosed { io }];
                    }
                } else {
                    return vec![Event::IoFailed {
                        io,
                        reason: "drain not found".to_string(),
                    }];
                }
                vec![]
            }
            Effect::AddonTask { addon_id, key, payload } => {
                if payload.is_empty() {
                    return vec![Event::AddonFailed { addon_id, key, err: "empty payload".to_string() }];
                }

                match payload[0] {
                    1 => { // Warmup (Preload Addon Specific)
                        let start = std::time::Instant::now();
                        let mut bytes = 0;
                        let mut failure_reason = None;

                        if let Ok(paths) = serde_json::from_slice::<Vec<String>>(&payload[1..]) {
                            for path in paths {
                                match std::ffi::CString::new(path.clone()) {
                                    Ok(c_path) => unsafe {
                                        let fd = libc::open(c_path.as_ptr(), libc::O_RDONLY);
                                        if fd >= 0 {
                                            let mut st: libc::stat = std::mem::zeroed();
                                            if libc::fstat(fd, &mut st) == 0 {
                                                bytes += st.st_size as u64;
                                            }
                                            do_readahead(fd);
                                            libc::close(fd);
                                        } else {
                                            failure_reason = Some(format!("open failed for {}", path));
                                            break;
                                        }
                                    },
                                    Err(e) => {
                                        failure_reason = Some(format!("invalid CString path: {}", e));
                                        break;
                                    }
                                }
                            }
                        } else {
                             failure_reason = Some("failed to decode paths".to_string());
                        }

                        if let Some(err) = failure_reason {
                            vec![Event::AddonFailed { addon_id, key, err }]
                        } else {
                            let duration_ms = start.elapsed().as_millis() as u64;
                            let res_payload = serde_json::to_vec(&(bytes, duration_ms)).unwrap_or_default();
                            vec![Event::AddonCompleted { addon_id, key, payload: res_payload }]
                        }
                    }
                    _ => vec![Event::AddonFailed { addon_id, key, err: "unknown task type".to_string() }]
                }
            }
            Effect::AddonLog { addon_id, level, msg } => {
                self.log_router.write(addon_id, level, msg);
                vec![]
            }
            Effect::SystemRequest { request_id, kind, payload } => {
                match kind {
                    SystemService::ResolveIdentity => {
                         if let Ok(pid) = serde_json::from_slice::<i32>(&payload) {
                            let cmdline_path = format!("/proc/{}/cmdline", pid);
                            match std::fs::read(&cmdline_path) {
                                Ok(cmdline) => {
                                    if let Some(null_pos) = cmdline.iter().position(|&c| c == 0) {
                                        let package_name = &cmdline[..null_pos];
                                        vec![Event::SystemResponse { request_id, kind, payload: package_name.to_vec() }]
                                    } else {
                                        vec![Event::SystemFailure { request_id, kind, err: "no null terminator".to_string() }]
                                    }
                                }
                                Err(e) => vec![Event::SystemFailure { request_id, kind, err: format!("read failed: {}", e) }]
                            }
                         } else {
                             vec![Event::SystemFailure { request_id, kind, err: "invalid pid payload".to_string() }]
                         }
                    }
                    SystemService::ResolveDirectory => {
                        if let Ok(package_name) = String::from_utf8(payload) {
                             match std::fs::read_dir("/data/app") {
                                Ok(data_app) => {
                                    let mut found = false;
                                    let mut res = vec![];
                                    for outer_entry in data_app.flatten() {
                                        let outer_path = outer_entry.path();
                                        if outer_path.is_dir() && let Ok(inner_dir) = std::fs::read_dir(&outer_path) {
                                            for inner_entry in inner_dir.flatten() {
                                                let inner_name = inner_entry.file_name();
                                                if inner_name.to_string_lossy().starts_with(&package_name) {
                                                    let base_dir = inner_entry.path().to_string_lossy().into_owned();
                                                    let resp_payload = serde_json::to_vec(&(package_name.clone(), base_dir)).unwrap_or_default();
                                                    res = vec![Event::SystemResponse { request_id, kind, payload: resp_payload }];
                                                    found = true;
                                                    break;
                                                }
                                            }
                                        }
                                        if found { break; }
                                    }
                                    if !found {
                                        vec![Event::SystemFailure { request_id, kind, err: "package dir not found".to_string() }]
                                    } else {
                                        res
                                    }
                                }
                                Err(e) => vec![Event::SystemFailure { request_id, kind, err: format!("read_dir /data/app failed: {}", e) }]
                            }
                        } else {
                            vec![Event::SystemFailure { request_id, kind, err: "invalid package name payload".to_string() }]
                        }
                    }
                    SystemService::DiscoverPaths => {
                        if let Ok((package_name, base_dir)) = serde_json::from_slice::<(String, String)>(&payload) {
                            let mut paths = Vec::new();
                            let base_path = std::path::PathBuf::from(&base_dir);

                            let lib_dir = base_path.join("lib/arm64");
                            if let Ok(entries) = std::fs::read_dir(&lib_dir) {
                                for entry in entries.flatten() {
                                    if let Some(ext) = entry.path().extension() && ext == "so" {
                                        paths.push(entry.path().to_string_lossy().into_owned());
                                    }
                                }
                            }

                            let oat_dir = base_path.join("oat/arm64");
                            if let Ok(entries) = std::fs::read_dir(&oat_dir) {
                                for entry in entries.flatten() {
                                    if let Some(ext) = entry.path().extension() && (ext == "odex" || ext == "vdex" || ext == "art") {
                                        paths.push(entry.path().to_string_lossy().into_owned());
                                    }
                                }
                            }

                            paths.push(base_path.join("base.apk").to_string_lossy().into_owned());

                            if let Ok(entries) = std::fs::read_dir(&base_path) {
                                for entry in entries.flatten() {
                                    let name = entry.file_name();
                                    let name_str = name.to_string_lossy();
                                    if name_str.starts_with("split_") && name_str.ends_with(".apk") {
                                        paths.push(entry.path().to_string_lossy().into_owned());
                                    }
                                }
                            }

                            if !paths.is_empty() {
                                paths.sort_unstable();
                                paths.truncate(64);
                                let resp_payload = serde_json::to_vec(&(package_name, paths)).unwrap_or_default();
                                vec![Event::SystemResponse { request_id, kind, payload: resp_payload }]
                            } else {
                                vec![Event::SystemFailure { request_id, kind, err: "no paths discovered".to_string() }]
                            }
                        } else {
                            vec![Event::SystemFailure { request_id, kind, err: "invalid discovery payload".to_string() }]
                        }
                    }
                }
            }
        }
    }
}
