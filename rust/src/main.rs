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
            let mut stream = UnixStream::connect("/data/local/tmp/coreshift/coreshift.sock")
                .map_err(|e| format!("failed to connect to daemon: {}", e))?;

            // Type 1 is Command, then JSON payload.
            let cmd = CoreShift::high_level::api::Command::PreloadStatus;
            let payload = serde_json::to_vec(&cmd).map_err(|e| e.to_string())?;
            let mut req = Vec::with_capacity(5 + payload.len());
            let len = (payload.len() as u32 + 1).to_le_bytes(); // length includes type byte
            req.extend_from_slice(&len);
            req.push(1u8); // Type 1: JSON Command
            req.extend_from_slice(&payload);
            stream.write_all(&req).map_err(|e| e.to_string())?;

            // Read length
            let mut len_buf = [0u8; 4];
            stream.read_exact(&mut len_buf).map_err(|e| e.to_string())?;
            let resp_len = u32::from_le_bytes(len_buf) as usize;

            if resp_len > 0 {
                let mut resp_buf = vec![0u8; resp_len];
                stream
                    .read_exact(&mut resp_buf)
                    .map_err(|e| e.to_string())?;
                if resp_buf[0] == 5 {
                    // PreloadStatus
                    let status = String::from_utf8_lossy(&resp_buf[1..]);
                    println!("{}", status);
                    Ok(())
                } else {
                    Err(format!("unexpected response type: {}", resp_buf[0]))
                }
            } else {
                Err("empty response".to_string())
            }
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
