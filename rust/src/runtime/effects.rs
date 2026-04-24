// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/

use super::control::{apply_control_signal, exit_status_code};
use super::logging::{LogRouter, format_log_event};
use super::system_services::handle_system_request;
use crate::arena::Arena;
use crate::core::{Effect, Event, IoHandle};
use crate::low_level::io::DrainState;
use crate::low_level::reactor::{Event as ReactorEvent, Reactor};
use crate::low_level::spawn::{Process, SpawnBackend, SpawnOptions, SysError, spawn_start};
use crate::low_level::sys::{ExecContext, ProcessGroup};

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
                && let Some((io, stream)) = self.fd_map[idx]
            {
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

    pub fn apply(&mut self, effect: Effect) -> Vec<Event> {
        match effect {
            Effect::Log {
                owner,
                level,
                event,
            } => {
                let msg = format_log_event(&event);
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
                                && *mapped_io == io
                                && *mapped_stream == stream
                            {
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
                    if let Err(e) = apply_control_signal(&mut rproc.process, rproc.is_group, signal)
                    {
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
                            vec![Event::ProcessExited {
                                process,
                                status: Some(exit_status_code(status)),
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
            Effect::AddonTask {
                addon_id,
                key,
                payload,
            } => {
                if payload.is_empty() {
                    return vec![Event::AddonFailed {
                        addon_id,
                        key,
                        err: "empty payload".to_string(),
                    }];
                }

                match payload[0] {
                    1 => {
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
                                            failure_reason =
                                                Some(format!("open failed for {}", path));
                                            break;
                                        }
                                    },
                                    Err(e) => {
                                        failure_reason =
                                            Some(format!("invalid CString path: {}", e));
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
                            let res_payload =
                                serde_json::to_vec(&(bytes, duration_ms)).unwrap_or_default();
                            vec![Event::AddonCompleted {
                                addon_id,
                                key,
                                payload: res_payload,
                            }]
                        }
                    }
                    _ => vec![Event::AddonFailed {
                        addon_id,
                        key,
                        err: "unknown task type".to_string(),
                    }],
                }
            }
            Effect::AddonLog {
                addon_id,
                level,
                msg,
            } => {
                self.log_router.write(addon_id, level, msg);
                vec![]
            }
            Effect::SystemRequest {
                request_id,
                kind,
                payload,
            } => handle_system_request(request_id, kind, payload),
        }
    }
}
