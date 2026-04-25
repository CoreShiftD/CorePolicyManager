// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/

use std::process::ExitCode;

fn print_help() {
    println!("CoreShift Policy Diagnostics");
    println!("Usage: coredebug [command] [args]");
    println!();
    println!("Commands:");
    println!("  test           Run all substrate diagnostic probes (Planned)");
    println!("  test <cat>     Run diagnostics for a specific category (e.g. low_level)");
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
        Some("test") => {
            let cat = args.next();
            let subcat = args.next();

            match (cat.as_deref(), subcat.as_deref()) {
                (None, _) => {
                    println!("Running all planned substrate diagnostic probes...");
                    println!("Status: Not implemented yet.");
                    ExitCode::from(1)
                }
                (Some("low_level"), None) => {
                    println!("Running low_level diagnostic probes...");
                    println!("Status: Not implemented yet.");
                    ExitCode::from(1)
                }
                (Some("low_level"), Some("spawn")) => {
                    println!("Running low_level::spawn diagnostic probes...");
                    println!("Status: Not implemented yet.");
                    ExitCode::from(1)
                }
                (Some(c), _) => {
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
