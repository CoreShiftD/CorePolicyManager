use coreshift_policy::runtime::{daemon::Daemon, logging, signals, status::DaemonStatus};
use std::process::ExitCode;

fn print_help() {
    println!("CoreShift Policy");
    println!("Usage: corepolicy [flags] [command]");
    println!();
    println!("Flags:");
    println!("  -p             Preload-only daemon mode");
    println!("  -h, --help     Show this help");
    println!();
    println!("Commands:");
    println!("  status         Show current daemon status (from status.json)");
    println!("  help           Show this help");
}

fn main() -> ExitCode {
    let mut args = std::env::args().skip(1);
    let first_arg = args.next();

    match first_arg.as_deref() {
        None => {
            // Default daemon mode (currently same as preload enabled)
            logging::init();
            signals::setup();
            let mut daemon = Daemon::new(false);
            daemon.run();
            ExitCode::SUCCESS
        }
        Some("-p") => {
            // Preload-only daemon mode
            logging::init();
            signals::setup();
            let mut daemon = Daemon::new(true);
            daemon.run();
            ExitCode::SUCCESS
        }
        Some("status") => match DaemonStatus::read() {
            Some(status) => {
                println!("{}", serde_json::to_string_pretty(&status).unwrap());
                ExitCode::SUCCESS
            }
            None => {
                eprintln!("error: could not read status.json (daemon not running?)");
                ExitCode::from(1)
            }
        },
        Some("help" | "--help" | "-h") => {
            print_help();
            ExitCode::SUCCESS
        }
        Some(cmd) => {
            eprintln!("error: unknown argument '{}'", cmd);
            print_help();
            ExitCode::from(2)
        }
    }
}
