// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/

use std::process::ExitCode;

fn print_help() {
    println!("CoreShift Policy");
    println!("Usage: corepolicy [flags] [command]");
    println!();
    println!("Flags:");
    println!("  -p             Run with autonomous preload enabled (Planned)");
    println!("  -h, --help     Show this help");
    println!();
    println!("Commands:");
    println!("  status         Show current daemon status (Planned)");
    println!("  help           Show this help");
}

fn main() -> ExitCode {
    let mut args = std::env::args().skip(1);
    match args.next().as_deref() {
        None => {
            println!("CoreShift Policy Daemon");
            println!("Low-level substrate only. Daemon logic not yet implemented.");
            ExitCode::SUCCESS
        }
        Some("help" | "--help" | "-h") => {
            print_help();
            ExitCode::SUCCESS
        }
        Some("-p") => {
            eprintln!("error: preload runtime is not implemented yet");
            ExitCode::from(1)
        }
        Some("status") => {
            eprintln!("error: status reader is not implemented yet");
            ExitCode::from(1)
        }
        Some(cmd) => {
            eprintln!("error: unknown argument '{}'", cmd);
            print_help();
            ExitCode::from(2)
        }
    }
}
