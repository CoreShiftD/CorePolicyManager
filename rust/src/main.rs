use coreshift_policy::features::profile::CategoryDatabase;
use coreshift_policy::runtime::{daemon::Daemon, logging, signals, status};
use std::collections::BTreeSet;
use std::process::ExitCode;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum CliFeature {
    Preload,
    Usage,
    Pressure,
    AppIndex,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct StartConfig {
    preload_only: bool,
    used_deprecated_p: bool,
    requested: BTreeSet<CliFeature>,
}

fn print_help() {
    println!("CoreShift Policy");
    println!("Usage: corepolicy [start] [flags] [command]");
    println!();
    println!("Start Flags:");
    println!("  -f, --feature FEATURE   Enable preview feature selection");
    println!("  -p FEATURE             Deprecated alias for -f FEATURE");
    println!("  --all                  Enable all preview features");
    println!("  -h, --help             Show this help");
    println!();
    println!("Feature Names:");
    println!("  preload");
    println!("  usage");
    println!("  pressure");
    println!("  app_index");
    println!();
    println!("Compatibility:");
    println!("  profile                Deprecated alias for usage");
    println!();
    println!("Commands:");
    println!("  start          Start the daemon");
    println!("  status         Show current daemon status (from status.json)");
    println!("  profile ...    Manage app profile categories");
    println!("  help           Show this help");
}

fn parse_feature_name(value: &str) -> Result<CliFeature, String> {
    match value {
        "preload" => Ok(CliFeature::Preload),
        "usage" | "profile" => Ok(CliFeature::Usage),
        "pressure" => Ok(CliFeature::Pressure),
        "app_index" => Ok(CliFeature::AppIndex),
        _ => Err(format!("error: unknown feature '{}'", value)),
    }
}

fn parse_start_args<I>(args: I) -> Result<StartConfig, ExitCode>
where
    I: IntoIterator<Item = String>,
{
    let mut used_deprecated_p = false;
    let mut requested = BTreeSet::new();
    let mut explicit = false;
    let mut args = args.into_iter();

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "-f" | "--feature" => {
                let Some(value) = args.next() else {
                    eprintln!("Usage: corepolicy start -f <feature> [--feature <feature> ...]");
                    return Err(ExitCode::from(2));
                };
                let Ok(feature) = parse_feature_name(&value) else {
                    eprintln!("error: unknown feature '{}'", value);
                    return Err(ExitCode::from(2));
                };
                requested.insert(feature);
                explicit = true;
            }
            "-p" => {
                let Some(value) = args.next() else {
                    eprintln!("Usage: corepolicy start -p <feature>");
                    return Err(ExitCode::from(2));
                };
                let Ok(feature) = parse_feature_name(&value) else {
                    eprintln!("error: unknown feature '{}'", value);
                    return Err(ExitCode::from(2));
                };
                requested.insert(feature);
                explicit = true;
                used_deprecated_p = true;
            }
            "--all" => {
                requested.insert(CliFeature::Preload);
                requested.insert(CliFeature::Usage);
                requested.insert(CliFeature::Pressure);
                requested.insert(CliFeature::AppIndex);
                explicit = true;
            }
            "-h" | "--help" => {
                print_help();
                return Err(ExitCode::SUCCESS);
            }
            value => {
                eprintln!("error: unknown start argument '{}'", value);
                return Err(ExitCode::from(2));
            }
        }
    }

    let preload_only = explicit
        && requested.contains(&CliFeature::Preload)
        && !requested.contains(&CliFeature::Usage)
        && !requested.contains(&CliFeature::Pressure)
        && !requested.contains(&CliFeature::AppIndex);

    Ok(StartConfig {
        preload_only,
        used_deprecated_p,
        requested,
    })
}

fn start_daemon(config: StartConfig) -> ExitCode {
    if config.used_deprecated_p {
        eprintln!("warning: '-p FEATURE' is deprecated; use '-f FEATURE' or '--feature FEATURE'");
    }
    logging::init();
    signals::setup();
    let mut daemon = Daemon::new(config.preload_only);
    daemon.run();
    ExitCode::SUCCESS
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
        None => start_daemon(StartConfig {
            preload_only: false,
            used_deprecated_p: false,
            requested: BTreeSet::new(),
        }),
        Some("start") => match parse_start_args(args) {
            Ok(config) => start_daemon(config),
            Err(code) => code,
        },
        Some("-f" | "--feature" | "-p" | "--all") => {
            let mut forwarded = vec![first_arg.unwrap_or_default()];
            forwarded.extend(args);
            match parse_start_args(forwarded) {
                Ok(config) => start_daemon(config),
                Err(code) => code,
            }
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
    fn short_feature_flag_works() {
        let config = parse_start_args(vec!["-f".to_string(), "preload".to_string()]).unwrap();
        assert!(config.preload_only);
        assert!(config.requested.contains(&CliFeature::Preload));
    }

    #[test]
    fn long_feature_flag_works() {
        let config = parse_start_args(vec!["--feature".to_string(), "usage".to_string()]).unwrap();
        assert!(!config.preload_only);
        assert!(config.requested.contains(&CliFeature::Usage));
    }

    #[test]
    fn deprecated_p_still_works() {
        let config = parse_start_args(vec!["-p".to_string(), "preload".to_string()]).unwrap();
        assert!(config.preload_only);
        assert!(config.used_deprecated_p);
    }

    #[test]
    fn profile_alias_maps_to_usage() {
        let config = parse_start_args(vec!["-f".to_string(), "profile".to_string()]).unwrap();
        assert!(config.requested.contains(&CliFeature::Usage));
        assert!(!config.preload_only);
    }

    #[test]
    fn all_flag_works() {
        let config = parse_start_args(vec!["--all".to_string()]).unwrap();
        assert!(!config.preload_only);
        assert!(config.requested.contains(&CliFeature::Preload));
        assert!(config.requested.contains(&CliFeature::Usage));
        assert!(config.requested.contains(&CliFeature::Pressure));
        assert!(config.requested.contains(&CliFeature::AppIndex));
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
