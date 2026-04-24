// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/

use crate::core::{LogEvent, LogLevel};
use std::collections::HashMap;

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

pub(super) fn format_log_event(event: &LogEvent) -> String {
    match event {
        LogEvent::TickSummary {
            processed,
            dropped,
            queue_before,
            queue_after,
            elapsed_us,
        } => {
            format!(
                "tick processed={} dropped={} queue_before={} queue_after={} elapsed_ms={}",
                processed,
                dropped,
                queue_before,
                queue_after,
                elapsed_us / 1000
            )
        }
        LogEvent::ActionDispatch {
            kind,
            id,
            addon_id,
            key,
            service,
            payload_len,
        } => {
            let mut parts = vec![format!("action={:?}", kind)];
            if let Some(i) = id {
                parts.push(format!("id={}", i));
            }
            if let Some(a) = addon_id {
                parts.push(format!("addon_id={}", a));
            }
            if let Some(k) = key {
                parts.push(format!("key={}", k));
            }
            if let Some(s) = service {
                parts.push(format!("service={:?}", s));
            }
            if *payload_len > 0 {
                parts.push(format!("payload_len={}", payload_len));
            }
            parts.join(" ")
        }
        LogEvent::PreloadForeground { pid, package } => {
            format!("preload foreground pid={} package={}", pid, package)
        }
        LogEvent::PreloadSkip {
            package,
            reason,
            remaining_ms,
        } => {
            let mut s = format!("preload skip package={} reason={}", package, reason);
            if let Some(r) = remaining_ms {
                s.push_str(&format!(" remaining_ms={}", r));
            }
            s
        }
        LogEvent::PreloadStart { package, paths } => {
            format!("preload start package={} paths={}", package, paths)
        }
        LogEvent::PreloadDone {
            package,
            paths,
            bytes,
            duration_ms,
        } => {
            format!(
                "preload done package={} paths={} bytes={} duration_ms={}",
                package, paths, bytes, duration_ms
            )
        }
        LogEvent::PreloadFail {
            package,
            reason,
            backoff_ms,
        } => {
            format!(
                "preload fail package={} reason={} backoff_ms={}",
                package, reason, backoff_ms
            )
        }
        LogEvent::Generic(s) => s.clone(),
        LogEvent::Error { id, err } => format!("Error id={}, err={}", id, err),
    }
}

pub fn log_runtime_event(owner: u32, level: LogLevel, event: LogEvent) {
    let mut log_router = LogRouter::new();
    let msg = format_log_event(&event);
    log_router.write(owner, level, msg);
}
