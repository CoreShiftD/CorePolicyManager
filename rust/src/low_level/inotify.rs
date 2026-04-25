// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/

//! Raw inotify helpers.
//!
//! Runtime code owns the semantic mapping from watch descriptors to daemon
//! events. This module owns only syscalls and byte-level event decoding.

use crate::low_level::reactor::Fd;
use crate::low_level::spawn::SysError;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct InotifyEvent {
    pub wd: i32,
    pub mask: u32,
    /// Raw `inotify_event.len` value.
    ///
    /// File watches commonly report an empty name; directory watches may carry
    /// the affected child name. Runtime code maps by watch descriptor and uses
    /// this only for diagnostics/tests.
    pub name_len: u32,
}

pub const MODIFY_MASK: u32 = libc::IN_MODIFY;
pub const PACKAGE_FILE_MASK: u32 = libc::IN_MODIFY | libc::IN_DELETE_SELF | libc::IN_MOVE_SELF;
pub const QUEUE_OVERFLOW_MASK: u32 = libc::IN_Q_OVERFLOW;
pub const IGNORED_MASK: u32 = libc::IN_IGNORED;
pub const UNMOUNT_MASK: u32 = libc::IN_UNMOUNT;
pub const DELETE_SELF_MASK: u32 = libc::IN_DELETE_SELF;
pub const MOVE_SELF_MASK: u32 = libc::IN_MOVE_SELF;

pub fn add_watch(fd: &Fd, path: &str, mask: u32) -> Result<i32, SysError> {
    let path = std::ffi::CString::new(path)
        .map_err(|_| SysError::sys(libc::EINVAL, "inotify path contains nul"))?;
    let wd = unsafe { libc::inotify_add_watch(fd.raw(), path.as_ptr(), mask) };
    if wd < 0 {
        return Err(SysError::sys(
            std::io::Error::last_os_error().raw_os_error().unwrap_or(0),
            "inotify_add_watch",
        ));
    }
    Ok(wd)
}

pub fn read_events(fd: &Fd) -> Result<Vec<InotifyEvent>, SysError> {
    let mut len: libc::c_int = 0;
    let ret = unsafe { libc::ioctl(fd.raw(), libc::FIONREAD, &mut len) };
    if ret < 0 {
        return Err(SysError::sys(
            std::io::Error::last_os_error().raw_os_error().unwrap_or(0),
            "ioctl(FIONREAD)",
        ));
    }
    if len <= 0 {
        return Ok(Vec::new());
    }

    let mut buf = vec![0u8; len as usize];
    let n = match fd.read(buf.as_mut_ptr(), buf.len()) {
        Ok(Some(0)) => return Ok(Vec::new()),
        Ok(Some(n)) => n,
        Ok(None) => return Ok(Vec::new()),
        Err(e) => return Err(e),
    };

    Ok(decode_events(&buf[..n]))
}

pub fn decode_events(buf: &[u8]) -> Vec<InotifyEvent> {
    let mut events = Vec::new();
    let mut offset = 0;
    let base = std::mem::size_of::<libc::inotify_event>();

    while offset + base <= buf.len() {
        let event = unsafe { &*(buf.as_ptr().add(offset) as *const libc::inotify_event) };
        let size = base + event.len as usize;
        if offset + size > buf.len() {
            break;
        }
        events.push(InotifyEvent {
            wd: event.wd,
            mask: event.mask,
            name_len: event.len,
        });
        offset += size;
    }

    events
}
