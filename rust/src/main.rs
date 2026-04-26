use coreshift_policy::features::profile::CategoryDatabase;
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
    println!("  profile ...    Manage app profile categories");
    println!("  help           Show this help");
}

fn handle_profile_cmd(mut args: std::iter::Skip<std::env::Args>) -> ExitCode {
    let cmd = args.next();
    let mut db = CategoryDatabase::load();
    match cmd.as_deref() {
        Some("list") => {
            for (cat, pkgs) in &db.categories {
                println!("{}: {} apps", cat, pkgs.len());
            }
            ExitCode::SUCCESS
        }
        Some("show") => {
            if let Some(cat) = args.next()
                && let Some(pkgs) = db.categories.get(&cat)
            {
                for pkg in pkgs {
                    println!("{}", pkg);
                }
            }
            ExitCode::SUCCESS
        }
        Some("add") => {
            if let (Some(cat), Some(pkg)) = (args.next(), args.next()) {
                if db.add(&cat, &pkg) {
                    let _ = db.save();
                } else {
                    eprintln!("error: unsupported category '{}'", cat);
                    return ExitCode::from(1);
                }
            }
            ExitCode::SUCCESS
        }
        Some("remove") => {
            if let Some(pkg) = args.next() {
                db.remove(&pkg);
                let _ = db.save();
            }
            ExitCode::SUCCESS
        }
        Some("classify") => {
            if let Some(pkg) = args.next() {
                println!("{}", db.classify(&pkg));
            }
            ExitCode::SUCCESS
        }
        Some("validate") => {
            println!("Validating categories...");
            let mut seen = std::collections::HashSet::new();
            for pkgs in db.categories.values() {
                for pkg in pkgs {
                    if !seen.insert(pkg) {
                        eprintln!("Duplicate package: {}", pkg);
                    }
                }
            }
            ExitCode::SUCCESS
        }
        _ => {
            eprintln!("Usage: corepolicy profile [list|show|add|remove|classify|validate]");
            ExitCode::from(2)
        }
    }
}

fn main() -> ExitCode {
    let mut args = std::env::args().skip(1);
    let first_arg = args.next();

    match first_arg.as_deref() {
        None => {
            logging::init();
            signals::setup();
            let mut daemon = Daemon::new(false);
            daemon.run();
            ExitCode::SUCCESS
        }
        Some("-p") => {
            logging::init();
            signals::setup();
            let mut daemon = Daemon::new(true);
            daemon.run();
            ExitCode::SUCCESS
        }
        Some("status") => match DaemonStatus::read() {
            Some(status) => {
                if cfg!(debug_assertions) {
                    println!("{}", serde_json::to_string_pretty(&status).unwrap());
                } else {
                    let db = CategoryDatabase::load();
                    let public = status.to_public_status(&db);
                    println!("{}", serde_json::to_string_pretty(&public).unwrap());
                }
                ExitCode::SUCCESS
            }
            None => {
                eprintln!("error: could not read status.json (daemon not running?)");
                ExitCode::from(1)
            }
        },
        Some("profile") => handle_profile_cmd(args),
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
