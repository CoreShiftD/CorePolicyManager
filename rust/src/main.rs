// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/

fn print_help() {
    println!("CoreShift Policy");
    println!("Usage: corepolicy [command] [args]");
    println!("Commands:");
    println!("  help           Show this help");
}

fn main() {
    let mut args = std::env::args().skip(1);
    match args.next().as_deref() {
        Some("help" | "--help" | "-h") => print_help(),
        _ => {
            println!("CoreShift Policy");
            println!("Low-level substrate only. Daemon logic not yet implemented.");
        }
    }
}
