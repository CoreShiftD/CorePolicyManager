use coreshift_policy::features::profile::CategoryDatabase;
use coreshift_policy::runtime::{daemon::Daemon, logging, signals, status};
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

fn handle_profile_cmd<I>(mut args: I) -> ExitCode
where
    I: Iterator<Item = String>,
{
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
            let Some(cat) = args.next() else {
                eprintln!("Usage: corepolicy profile show <category>");
                return ExitCode::from(2);
            };
            let Some(pkgs) = db.categories.get(&cat) else {
                eprintln!("error: unsupported category '{}'", cat);
                return ExitCode::from(1);
            };
            for pkg in pkgs {
                println!("{}", pkg);
            }
            ExitCode::SUCCESS
        }
        Some("add") => {
            let Some(cat) = args.next() else {
                eprintln!("Usage: corepolicy profile add <category> <package>");
                return ExitCode::from(2);
            };
            let Some(pkg) = args.next() else {
                eprintln!("Usage: corepolicy profile add <category> <package>");
                return ExitCode::from(2);
            };
            if db.add(&cat, &pkg) {
                if let Err(error) = db.save() {
                    eprintln!("error: failed to save profile database: {}", error);
                    return ExitCode::from(1);
                }
            } else {
                eprintln!("error: unsupported category '{}'", cat);
                return ExitCode::from(1);
            }
            ExitCode::SUCCESS
        }
        Some("remove") => {
            let Some(pkg) = args.next() else {
                eprintln!("Usage: corepolicy profile remove <package>");
                return ExitCode::from(2);
            };
            db.remove(&pkg);
            if let Err(error) = db.save() {
                eprintln!("error: failed to save profile database: {}", error);
                return ExitCode::from(1);
            }
            ExitCode::SUCCESS
        }
        Some("classify") => {
            let Some(pkg) = args.next() else {
                eprintln!("Usage: corepolicy profile classify <package>");
                return ExitCode::from(2);
            };
            println!("{}", db.classify(&pkg));
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
        Some("status") => {
            let db = CategoryDatabase::load();
            match status::read_public_status(&db) {
                Some(public) => {
                    println!("{}", serde_json::to_string_pretty(&public).unwrap());
                    ExitCode::SUCCESS
                }
                None => {
                    eprintln!("error: could not read status.json (daemon not running?)");
                    ExitCode::from(1)
                }
            }
        }
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    fn with_test_categories_file(path: &Path, f: impl FnOnce()) {
        unsafe {
            std::env::set_var("COREPOLICY_TEST_CATEGORIES_FILE", path);
        }
        f();
        unsafe {
            std::env::remove_var("COREPOLICY_TEST_CATEGORIES_FILE");
        }
    }

    #[test]
    fn profile_add_missing_args_returns_nonzero() {
        assert_ne!(
            handle_profile_cmd(vec!["add".to_string()].into_iter()),
            ExitCode::SUCCESS
        );
        assert_ne!(
            handle_profile_cmd(vec!["add".to_string(), "game".to_string()].into_iter()),
            ExitCode::SUCCESS
        );
    }

    #[test]
    fn profile_remove_missing_arg_returns_nonzero() {
        assert_ne!(
            handle_profile_cmd(vec!["remove".to_string()].into_iter()),
            ExitCode::SUCCESS
        );
    }

    #[test]
    fn profile_show_invalid_category_returns_nonzero() {
        assert_ne!(
            handle_profile_cmd(vec!["show".to_string(), "invalid".to_string()].into_iter()),
            ExitCode::SUCCESS
        );
    }

    #[test]
    fn profile_save_failure_returns_nonzero() {
        let categories_path = Path::new("/proc/self/status");

        with_test_categories_file(categories_path, || {
            assert_ne!(
                handle_profile_cmd(
                    vec![
                        "add".to_string(),
                        "game".to_string(),
                        "com.example.game".to_string()
                    ]
                    .into_iter()
                ),
                ExitCode::SUCCESS
            );
        });
    }
}
