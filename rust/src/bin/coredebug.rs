// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/

use std::process::ExitCode;

fn print_help() {
    println!("CoreShift Policy Diagnostics");
    println!("Usage: coredebug [command] [args]");
    println!();
    println!("Commands:");
    println!(
        "  probe <cat>    Run diagnostic substrate probes (e.g. procfs, inotify, paths, spawn)"
    );
    println!("  help           Show this help");
}

fn main() -> ExitCode {
    let mut args = std::env::args().skip(1);
    match args.next().as_deref() {
        None => {
            println!("CoreShift Policy Diagnostics");
            println!("Run 'coredebug help' for usage.");
            ExitCode::SUCCESS
        }
        Some("help" | "--help" | "-h") => {
            print_help();
            ExitCode::SUCCESS
        }
        Some("probe") => {
            let cat = args.next();

            match cat.as_deref() {
                None => {
                    println!("Running all planned substrate diagnostic probes...");
                    println!("Status: Not implemented yet.");
                    ExitCode::from(1)
                }
                Some("procfs") => {
                    println!("Probing procfs helper behavior...");
                    println!("Status: Not implemented yet.");
                    ExitCode::from(1)
                }
                Some("inotify") => {
                    println!("Probing inotify substrate...");
                    println!("Status: Not implemented yet.");
                    ExitCode::from(1)
                }
                Some("paths") => {
                    println!("Probing path existence/visibility...");
                    println!("Status: Not implemented yet.");
                    ExitCode::from(1)
                }
                Some("spawn") => {
                    println!("Probing process spawning primitives...");
                    println!("Status: Not implemented yet.");
                    ExitCode::from(1)
                }
                Some(c) => {
                    eprintln!("error: unknown diagnostic category '{}'", c);
                    ExitCode::from(2)
                }
            }
        }
        Some(cmd) => {
            eprintln!("error: unknown argument '{}'", cmd);
            print_help();
            ExitCode::from(2)
        }
    }
}
