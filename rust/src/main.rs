// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/

use CoreShift::DaemonConfig;
use std::process::ExitCode;

enum Command {
    Daemon,
    Help,
    Preload,
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
    println!("  record <file>  Run and record input to file");
    println!("  replay <file>  Replay recorded session");
    println!("  help           Show this help");
}

fn parse_command(mut args: impl Iterator<Item = String>) -> Result<Command, String> {
    match args.next().as_deref() {
        None => Ok(Command::Daemon),
        Some("help" | "--help" | "-h") => Ok(Command::Help),
        Some("preload") => Ok(Command::Preload),
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
        Command::Record(path) => CoreShift::run_daemon(DaemonConfig {
            enable_warmup: false,
            record_path: Some(path),
        })
        .map_err(|e| format!("{:?}", e)),
        Command::Replay(path) => {
            CoreShift::run_replay(&path);
            Ok(())
        }
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
