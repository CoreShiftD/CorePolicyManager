// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/

use crate::core::{CancelPolicy, ExecPolicy, ExecSpec};
use serde::{Deserialize, Serialize};

/// Token/Capability enforcement layers bounding operation intent.
// Replaced by CapabilityToken
/// Strict command schema defining finite execution space.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Command {
    Cmd { service: String, args: Vec<String> },
    Dumpsys { service: String, args: Vec<String> },
    GetResult { id: u64 },
    Cancel { id: u64 },
    PreloadStatus,
}

use crate::high_level::android::ExecConfig;

impl Command {
    pub fn map_to_exec(self) -> (ExecSpec, ExecPolicy) {
        match self {
            Command::Cmd { service, args } => {
                let args_refs: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
                let cfg = ExecConfig {
                    timeout_ms: None,
                    kill_grace_ms: 1000,
                    cancel: CancelPolicy::Graceful,
                    max_output: 1024 * 1024,
                };
                let req = crate::high_level::android::cmd(&service, &args_refs, cfg);
                (
                    ExecSpec {
                        argv: req.argv,
                        stdin: req.stdin,
                        capture_stdout: req.capture_stdout,
                        capture_stderr: req.capture_stderr,
                        max_output: req.max_output,
                    },
                    ExecPolicy {
                        timeout_ms: req.timeout_ms,
                        kill_grace_ms: req.kill_grace_ms,
                        cancel: req.cancel,
                    },
                )
            }
            Command::Dumpsys { service, args } => {
                let args_refs: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
                let cfg = ExecConfig {
                    timeout_ms: None,
                    kill_grace_ms: 1000,
                    cancel: CancelPolicy::Graceful,
                    max_output: 4 * 1024 * 1024,
                };
                let req = crate::high_level::android::dumpsys(&service, &args_refs, cfg);
                (
                    ExecSpec {
                        argv: req.argv,
                        stdin: req.stdin,
                        capture_stdout: req.capture_stdout,
                        capture_stderr: req.capture_stderr,
                        max_output: req.max_output,
                    },
                    ExecPolicy {
                        timeout_ms: req.timeout_ms,
                        kill_grace_ms: req.kill_grace_ms,
                        cancel: req.cancel,
                    },
                )
            }
            Command::GetResult { .. } | Command::Cancel { .. } | Command::PreloadStatus => {
                unreachable!()
            }
        }
    }
}
