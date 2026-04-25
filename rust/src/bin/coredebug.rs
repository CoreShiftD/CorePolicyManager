// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/

use coreshift_policy::low_level::sys::{path_exists, read_proc_cmdline, read_proc_status};
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

fn probe_paths() -> bool {
    let mut all_pass = true;

    println!("--- Probing paths ---");

    // 1. /data/local/tmp exists/accessible
    let tmp_path = "/data/local/tmp";
    if path_exists(tmp_path) {
        println!("PASS: {} exists and is accessible", tmp_path);
    } else {
        // On non-Android it might not exist, but let's WARN/FAIL based on intent
        println!(
            "WARN: {} is not accessible (normal on non-Android)",
            tmp_path
        );
        // We don't fail 'paths' probe just because of this if we are on host
    }

    // 2. /data/local/tmp/coreshift can be created or already exists
    let coreshift_path = "/data/local/tmp/coreshift";
    if path_exists(coreshift_path) {
        println!("PASS: {} already exists", coreshift_path);
    } else {
        match std::fs::create_dir_all(coreshift_path) {
            Ok(_) => {
                println!("PASS: {} created successfully", coreshift_path);
            }
            Err(e) => {
                println!("FAIL: {} could not be created: {}", coreshift_path, e);
                all_pass = false;
            }
        }
    }

    // 3. /proc/self/status can be read
    let pid = unsafe { libc::getpid() };
    match read_proc_status(pid) {
        Ok(status) => {
            println!("PASS: /proc/self/status read (Name: {})", status.name);
        }
        Err(e) => {
            println!("FAIL: /proc/self/status could not be read: {}", e);
            all_pass = false;
        }
    }

    // 4. /proc/self/cmdline can be read
    match read_proc_cmdline(pid) {
        Ok(cmdline) => {
            println!("PASS: /proc/self/cmdline read: {}", cmdline);
        }
        Err(e) => {
            println!("FAIL: /proc/self/cmdline could not be read: {}", e);
            all_pass = false;
        }
    }

    if all_pass {
        println!("RESULT: PASS");
    } else {
        println!("RESULT: FAIL");
    }

    all_pass
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
                    let p1 = probe_paths();
                    if p1 {
                        ExitCode::SUCCESS
                    } else {
                        ExitCode::from(1)
                    }
                }
                Some("paths") => {
                    if probe_paths() {
                        ExitCode::SUCCESS
                    } else {
                        ExitCode::from(1)
                    }
                }
                Some("procfs") | Some("inotify") | Some("spawn") => {
                    println!("Probing {} substrate...", cat.unwrap());
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
