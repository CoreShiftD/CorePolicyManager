// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/

//! Low-level execution context and signal helpers.
//!
//! `ExecContext` owns the exact C-compatible argv/env/cwd values that are later
//! passed into spawn backends. Validation happens here so higher layers cannot
//! silently drop malformed strings or rely on hidden fallbacks.
//!
//! Ownership and failure semantics:
//! - owned `CString` storage outlives the transient pointer arrays passed to
//!   `execve`-style backends
//! - validation failures are normal input errors and should be surfaced as
//!   spawn failures rather than repaired in place
//! - pointer helpers intentionally cap the pointer array size to keep stack
//!   usage bounded while preserving null termination

use crate::low_level::spawn::{SysError, syscall_ret};
use libc::sigset_t;

/// Probe whether a filesystem path is visible and accessible.
///
/// Uses `libc::access` with `F_OK`, which returns `true` if the path exists
/// and the process has permission to search all directories in the path
/// prefix. Returns `false` on any error (including `ENOENT`, `EACCES`, etc.).
///
/// Note: This does not follow symlinks for the final component if it's broken.
/// This is the canonical low-level path-existence helper.
pub fn path_exists(path: &str) -> bool {
    match std::ffi::CString::new(path) {
        Ok(c) => unsafe { libc::access(c.as_ptr(), libc::F_OK) == 0 },
        Err(_) => false,
    }
}

pub fn read_to_string(path: &str) -> Result<String, std::io::Error> {
    std::fs::read_to_string(path)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProcStatus {
    pub name: String,
    pub uid: u32,
}

/// Reads process status from `/proc/<pid>/status`.
/// Returns `Err` with `ErrorKind::NotFound` if the process does not exist.
pub fn read_proc_status(pid: i32) -> Result<ProcStatus, std::io::Error> {
    let path = format!("/proc/{}/status", pid);
    parse_proc_status(&std::fs::read_to_string(path)?)
}

/// Reads process command line from `/proc/<pid>/cmdline`.
/// Returns `Err` with `ErrorKind::NotFound` if the process does not exist.
pub fn read_proc_cmdline(pid: i32) -> Result<String, std::io::Error> {
    let path = format!("/proc/{}/cmdline", pid);
    let bytes = std::fs::read(path)?;
    Ok(String::from_utf8_lossy(&bytes)
        .trim_end_matches('\0')
        .replace('\0', " "))
}

pub fn parse_proc_status(content: &str) -> Result<ProcStatus, std::io::Error> {
    let mut name = None;
    let mut uid = None;

    for line in content.lines() {
        if let Some(rest) = line.strip_prefix("Name:") {
            name = Some(rest.trim().to_string());
        } else if let Some(rest) = line.strip_prefix("Uid:") {
            uid = rest
                .split_whitespace()
                .next()
                .and_then(|value| value.parse::<u32>().ok());
        }

        if name.is_some() && uid.is_some() {
            break;
        }
    }

    match (name, uid) {
        (Some(name), Some(uid)) => Ok(ProcStatus { name, uid }),
        _ => Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "proc status missing Name or Uid",
        )),
    }
}

pub struct SignalRuntime;

#[inline(always)]
pub fn get_clk_tck() -> u64 {
    unsafe { libc::sysconf(libc::_SC_CLK_TCK) as u64 }
}

impl SignalRuntime {
    pub fn empty_set() -> sigset_t {
        let mut set: sigset_t = unsafe { std::mem::zeroed() };
        unsafe { libc::sigemptyset(&mut set) };
        set
    }

    pub fn set_with(signals: &[i32]) -> sigset_t {
        let mut set: sigset_t = unsafe { std::mem::zeroed() };
        unsafe { libc::sigemptyset(&mut set) };
        for &sig in signals {
            unsafe { libc::sigaddset(&mut set, sig) };
        }
        set
    }

    pub fn unblock_all() -> Result<(), SysError> {
        let empty_mask = Self::empty_set();
        let r = unsafe { libc::sigprocmask(libc::SIG_SETMASK, &empty_mask, std::ptr::null_mut()) };
        syscall_ret(r, "sigprocmask")
    }

    pub fn reset_default(sig: i32) {
        unsafe { libc::signal(sig, libc::SIG_DFL) };
    }
}
use libc::{c_char, pid_t};
use serde::{Deserialize, Serialize};
use std::ffi::CString;
use std::ptr;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum CancelPolicy {
    #[default]
    None,
    Graceful, // implies term then kill
    Kill,     // implies direct kill
}

#[derive(Debug, Clone, Copy, Default)]
pub struct ProcessGroup {
    pub leader: Option<pid_t>,
    pub isolated: bool, // Corresponds to setsid
}

impl ProcessGroup {
    pub fn new(leader: Option<pid_t>, isolated: bool) -> Self {
        Self { leader, isolated }
    }
}

use arrayvec::ArrayVec;

pub enum ExecArgv {
    Dynamic(Vec<CString>),
}

pub struct ExecContext {
    pub argv: ExecArgv,
    pub envp: Option<Vec<CString>>,
    pub cwd: Option<CString>,
}

impl ExecContext {
    /// Build a validated execution context for process spawn.
    ///
    /// Rejections are explicit:
    /// - empty argv is invalid
    /// - interior NUL bytes in argv/env/cwd are invalid
    ///
    /// Higher layers should surface this as a normal spawn failure rather than
    /// attempting to repair or silently drop invalid inputs.
    pub fn new(
        argv: Vec<String>,
        env: Option<Vec<String>>,
        cwd: Option<String>,
    ) -> Result<Self, crate::low_level::spawn::SysError> {
        if argv.is_empty() {
            return Err(crate::low_level::spawn::SysError::sys(
                libc::EINVAL,
                "exec argv empty",
            ));
        }

        let c_argv: Vec<CString> = argv
            .into_iter()
            .map(|s| {
                CString::new(s).map_err(|_| {
                    crate::low_level::spawn::SysError::sys(libc::EINVAL, "exec argv contains nul")
                })
            })
            .collect::<Result<_, _>>()?;

        let c_envp = match env {
            Some(vars) => Some(
                vars.into_iter()
                    .map(|s| {
                        CString::new(s).map_err(|_| {
                            crate::low_level::spawn::SysError::sys(
                                libc::EINVAL,
                                "exec env contains nul",
                            )
                        })
                    })
                    .collect::<Result<Vec<_>, _>>()?,
            ),
            None => None,
        };

        let c_cwd = match cwd {
            Some(c) => Some(CString::new(c).map_err(|_| {
                crate::low_level::spawn::SysError::sys(libc::EINVAL, "exec cwd contains nul")
            })?),
            None => None,
        };

        Ok(Self {
            argv: ExecArgv::Dynamic(c_argv),
            envp: c_envp,
            cwd: c_cwd,
        })
    }

    pub fn get_argv_ptrs(&self) -> ArrayVec<*mut c_char, 64> {
        let mut ptrs = ArrayVec::new();
        match &self.argv {
            ExecArgv::Dynamic(v) => {
                // The pointed-to CString storage is owned by self; only the
                // pointer array is transient.
                for s in v {
                    if ptrs.try_push(s.as_ptr() as *mut c_char).is_err() {
                        break;
                    }
                }
            }
        }
        if ptrs.is_full() {
            ptrs.pop(); // Ensure room for null terminator
        }
        let _ = ptrs.try_push(ptr::null_mut());
        ptrs
    }

    pub fn get_envp_ptrs(&self) -> Option<ArrayVec<*mut c_char, 64>> {
        // We intentionally truncate to keep the stack-allocated pointer array
        // bounded; the owned CString storage remains valid for the lifetime of
        // the context.
        self.envp.as_ref().map(|envp| {
            let mut ptrs = ArrayVec::new();
            for s in envp {
                if ptrs.try_push(s.as_ptr() as *mut c_char).is_err() {
                    break;
                }
            }
            if ptrs.is_full() {
                ptrs.pop(); // Ensure room for null terminator
            }
            let _ = ptrs.try_push(ptr::null_mut());
            ptrs
        })
    }
}
