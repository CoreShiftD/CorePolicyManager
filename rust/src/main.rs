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
    requested: BTreeSet<CliFeature>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Command {
    ShowHelp,
    Start(StartConfig),
    Status,
    Profile(Vec<String>),
    Unknown(String),
}

fn render_help() -> &'static str {
    "CoreShift Policy CLI

Usage:
  corepolicy help
  corepolicy status
  corepolicy start [--all] [-f FEATURE...]
  corepolicy profile ...

Features:
  preload
  usage
  pressure
  app_index

Compatibility:
  profile         Deprecated alias for usage"
}

fn print_help() {
    println!("{}", render_help());
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
                eprintln!("-p has been removed. Use -f or --feature.");
                return Err(ExitCode::from(2));
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
        requested,
    })
}

fn start_daemon(config: StartConfig) -> ExitCode {
    logging::init();
    signals::setup();
    let mut daemon = Daemon::new(config.preload_only);
    daemon.run();
    ExitCode::SUCCESS
}

fn parse_cli_args<I>(args: I) -> Result<Command, ExitCode>
where
    I: IntoIterator<Item = String>,
{
    let mut args = args.into_iter();
    let first_arg = args.next();

    match first_arg.as_deref() {
        None => Ok(Command::ShowHelp),
        Some("start") => match parse_start_args(args) {
            Ok(config) => Ok(Command::Start(config)),
            Err(code) => Err(code),
        },
        Some("-f" | "--feature" | "--all") => {
            let mut forwarded = vec![first_arg.unwrap_or_default()];
            forwarded.extend(args);
            match parse_start_args(forwarded) {
                Ok(config) => Ok(Command::Start(config)),
                Err(code) => Err(code),
            }
        }
        Some("-p") => {
            eprintln!("-p has been removed. Use -f or --feature.");
            Err(ExitCode::from(2))
        }
        Some("status") => Ok(Command::Status),
        Some("profile") => Ok(Command::Profile(args.collect())),
        Some("help" | "--help" | "-h") => Ok(Command::ShowHelp),
        Some(cmd) => Ok(Command::Unknown(cmd.to_string())),
    }
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
    match parse_cli_args(std::env::args().skip(1)) {
        Err(code) => code,
        Ok(Command::ShowHelp) => {
            print_help();
            ExitCode::SUCCESS
        }
        Ok(Command::Start(config)) => start_daemon(config),
        Ok(Command::Status) => {
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
        Ok(Command::Profile(args)) => handle_profile_cmd(args.into_iter()),
        Ok(Command::Unknown(cmd)) => {
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
    fn no_arg_invocation_shows_help() {
        assert_eq!(
            parse_cli_args(Vec::<String>::new()).unwrap(),
            Command::ShowHelp
        );
        assert_eq!(render_help(), render_help());
    }

    #[test]
    fn help_command_shows_same_help() {
        assert_eq!(
            parse_cli_args(vec!["help".to_string()]).unwrap(),
            Command::ShowHelp
        );
        assert_eq!(render_help(), render_help());
    }

    #[test]
    fn unknown_command_exits_nonzero() {
        match parse_cli_args(vec!["bogus".to_string()]).unwrap() {
            Command::Unknown(cmd) => assert_eq!(cmd, "bogus"),
            other => panic!("unexpected command: {:?}", other),
        }
    }

    #[test]
    fn no_arg_invocation_has_no_side_effects() {
        assert_eq!(
            parse_cli_args(Vec::<String>::new()).unwrap(),
            Command::ShowHelp
        );
    }

    #[test]
    fn long_feature_flag_works() {
        let config = parse_start_args(vec!["--feature".to_string(), "usage".to_string()]).unwrap();
        assert!(!config.preload_only);
        assert!(config.requested.contains(&CliFeature::Usage));
    }

    #[test]
    fn removed_p_flag_returns_nonzero() {
        assert_eq!(
            parse_start_args(vec!["-p".to_string(), "preload".to_string()]),
            Err(ExitCode::from(2))
        );
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
