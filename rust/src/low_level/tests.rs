// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/

use crate::low_level::inotify::{InotifyEvent, decode_events};
use crate::low_level::spawn::Process;
use crate::low_level::sys::{ExecContext, parse_proc_status};

#[test]
fn test_decode_inotify_events() {
    // Mock multiple inotify_event records.
    let mut buf = Vec::new();

    // Event 1: wd=1, mask=0x2, len=0
    buf.extend_from_slice(&1i32.to_ne_bytes());
    buf.extend_from_slice(&2u32.to_ne_bytes());
    buf.extend_from_slice(&0u32.to_ne_bytes()); // cookie
    buf.extend_from_slice(&0u32.to_ne_bytes()); // len

    // Event 2: wd=2, mask=0x4, len=8 (with name padding)
    buf.extend_from_slice(&2i32.to_ne_bytes());
    buf.extend_from_slice(&4u32.to_ne_bytes());
    buf.extend_from_slice(&0u32.to_ne_bytes()); // cookie
    buf.extend_from_slice(&8u32.to_ne_bytes()); // len
    buf.extend_from_slice(b"file.txt"); // 8 bytes

    // Event 3: truncated (only 8 bytes of header)
    buf.extend_from_slice(&3i32.to_ne_bytes());
    buf.extend_from_slice(&8u32.to_ne_bytes());

    let events = decode_events(&buf);
    assert_eq!(events.len(), 2);
    assert_eq!(
        events[0],
        InotifyEvent {
            wd: 1,
            mask: 2,
            name_len: 0
        }
    );
    assert_eq!(
        events[1],
        InotifyEvent {
            wd: 2,
            mask: 4,
            name_len: 8
        }
    );
}

#[test]
fn test_parse_proc_status() {
    let content = "Name:\tcore_daemon\nState:\tR (running)\nUid:\t1000\t1000\t1000\t1000\nGid:\t1000\t1000\t1000\t1000\n";
    let status = parse_proc_status(content).unwrap();
    assert_eq!(status.name, "core_daemon");
    assert_eq!(status.uid, 1000);
}

#[test]
fn test_exec_context_validation() {
    // Empty argv
    let res = ExecContext::new(vec![], None, None);
    assert!(res.is_err());

    // Interior NUL in argv
    let res = ExecContext::new(
        vec!["valid".to_string(), "inv\0alid".to_string()],
        None,
        None,
    );
    assert!(res.is_err());

    // Valid
    let res = ExecContext::new(
        vec!["ls".to_string(), "-l".to_string()],
        None,
        Some("/tmp".to_string()),
    );
    assert!(res.is_ok());
}

#[test]
fn test_process_echild() {
    // Use an invalid PID that likely doesn't exist and isn't our child.
    let p = Process::new(999999);
    let res = p.wait_step();
    // Should be an error (ECHILD), not Ok(Some(Exited(0))).
    assert!(res.is_err());
}
