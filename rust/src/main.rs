// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/

use CoreShift::DaemonConfig;

fn main() {
    let mut args = std::env::args().skip(1);

    if let Some(cmd) = args.next() {
        match cmd.as_str() {
            "preload" => {
                let _ = CoreShift::run_daemon(DaemonConfig {
                    enable_warmup: true,
                    record_path: None,
                });
            }
            "replay" => {
                if let Some(path) = args.next() {
                    CoreShift::run_replay(&path);
                } else {
                    eprintln!("Usage: replay <file>");
                }
            }
            "record" => {
                if let Some(path) = args.next() {
                    let _ = CoreShift::run_daemon(DaemonConfig {
                        enable_warmup: false,
                        record_path: Some(path),
                    });
                } else {
                    eprintln!("Usage: record <file>");
                }
            }
            _ => {
                let _ = CoreShift::run_daemon(DaemonConfig {
                    enable_warmup: false,
                    record_path: None,
                });
            }
        }
    } else {
        let _ = CoreShift::run_daemon(DaemonConfig {
            enable_warmup: false,
            record_path: None,
        });
    }
}
