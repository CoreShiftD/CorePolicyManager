// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/

use CoreShift::DaemonConfig;
use std::process::ExitCode;

enum Command {
    Daemon,
    Help,
    Preload,
    PreloadStatus,
    Record(String),
    Replay(String),
}

fn log_cli(level: CoreShift::core::LogLevel, message: impl Into<String>) {
    CoreShift::runtime::log_runtime_event(
        CoreShift::core::CORE_OWNER,
        level,
        CoreShift::core::LogEvent::Generic(message.into()),
    );
}

fn print_help() {
    println!("CoreShift Daemon");
    println!("Usage: coreshift [command] [args]");
    println!("Commands:");
    println!("  preload        Run with warmup enabled");
    println!("  preload-status Print the current Preload Addon diagnostic state");
    println!("  record <file>  Run and record input to file");
    println!("  replay <file>  Replay recorded session");
    println!("  help           Show this help");
}

fn parse_command(mut args: impl Iterator<Item = String>) -> Result<Command, String> {
    match args.next().as_deref() {
        None => Ok(Command::Daemon),
        Some("help" | "--help" | "-h") => Ok(Command::Help),
        Some("preload") => Ok(Command::Preload),
        Some("preload-status") => Ok(Command::PreloadStatus),
        Some("record") => args
            .next()
            .map(Command::Record)
            .ok_or_else(|| "invalid arguments usage='record <file>'".to_string()),
        Some("replay") => args
            .next()
            .map(Command::Replay)
            .ok_or_else(|| "invalid arguments usage='replay <file>'".to_string()),
        Some(other) => Err(format!("unknown command '{}'", other)),
    }
}

fn run_command(command: Command) -> Result<(), String> {
    match command {
        Command::Daemon => CoreShift::run_daemon(DaemonConfig {
            enable_warmup: false,
            record_path: None,
        })
        .map_err(|e| format!("{:?}", e)),
        Command::Help => {
            print_help();
            Ok(())
        }
        Command::Preload => CoreShift::run_daemon(DaemonConfig {
            enable_warmup: true,
            record_path: None,
        })
        .map_err(|e| format!("{:?}", e)),
        Command::PreloadStatus => {
            use std::io::{Read, Write};
            use std::os::unix::net::UnixStream;
            let mut stream = UnixStream::connect(CoreShift::paths::SOCKET_PATH)
                .map_err(|e| format!("failed to connect to daemon: {}", e))?;

            // Type 4: PreloadStatus request (single-byte body, no JSON payload).
            let body: [u8; 1] = [4u8];
            let len = (body.len() as u32).to_le_bytes();
            stream.write_all(&len).map_err(|e| e.to_string())?;
            stream.write_all(&body).map_err(|e| e.to_string())?;

            // Read 4-byte length prefix.
            let mut len_buf = [0u8; 4];
            stream.read_exact(&mut len_buf).map_err(|e| e.to_string())?;
            let resp_len = u32::from_le_bytes(len_buf) as usize;

            if resp_len == 0 {
                return Err("empty response from daemon".to_string());
            }

            let mut resp_buf = vec![0u8; resp_len];
            stream
                .read_exact(&mut resp_buf)
                .map_err(|e| e.to_string())?;

            // Response type byte 5 = PreloadStatus JSON payload.
            if resp_buf[0] != 5 {
                return Err(format!("unexpected response type: {}", resp_buf[0]));
            }

            let json_bytes = &resp_buf[1..];
            // Deserialize into the typed report; fall back to raw UTF-8 on
            // parse failure so the CLI is never silently empty.
            match serde_json::from_slice::<CoreShift::high_level::api::DaemonStatusReport>(
                json_bytes,
            ) {
                Ok(report) => {
                    println!(
                        "{}",
                        serde_json::to_string_pretty(&report).unwrap_or_default()
                    );
                }
                Err(_) => {
                    println!("{}", String::from_utf8_lossy(json_bytes));
                }
            }
            Ok(())
        }
        Command::Record(path) => CoreShift::run_daemon(DaemonConfig {
            enable_warmup: false,
            record_path: Some(path),
        })
        .map_err(|e| format!("{:?}", e)),
        Command::Replay(path) => CoreShift::run_replay(&path)
            .map(|_| ())
            .map_err(|e| format!("{:?}", e)),
    }
}

fn main() -> ExitCode {
    let command = match parse_command(std::env::args().skip(1)) {
        Ok(command) => command,
        Err(err) => {
            log_cli(CoreShift::core::LogLevel::Error, err);
            return ExitCode::from(2);
        }
    };

    match run_command(command) {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            log_cli(
                CoreShift::core::LogLevel::Error,
                format!("fatal error: {}", err),
            );
            ExitCode::FAILURE
        }
    }
}
