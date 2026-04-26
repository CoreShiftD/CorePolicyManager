use coreshift_policy::features::profile::CategoryDatabase;
use coreshift_policy::runtime::{
    daemon::{Daemon, DaemonConfig},
    logging, signals, status,
};
use std::collections::BTreeSet;
use std::process::ExitCode;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Feature {
    Preload,
    Usage,
    Pressure,
    AppIndex,
    Profile,
}

const ALL_FEATURES: [Feature; 5] = [
    Feature::Preload,
    Feature::Usage,
    Feature::Pressure,
    Feature::AppIndex,
    Feature::Profile,
];

#[derive(Debug, PartialEq, Eq)]
enum Command {
    ShowHelp,
    RunStatus,
    StartDaemon(BTreeSet<Feature>),
    CategoryList,
    CategorySet(String, String),
    CategoryRemove(String),
}

#[derive(Debug, PartialEq, Eq)]
struct CliError(String);

fn parse_feature_name(value: &str) -> Result<Feature, CliError> {
    match value {
        "preload" => Ok(Feature::Preload),
        "usage" => Ok(Feature::Usage),
        "pressure" => Ok(Feature::Pressure),
        "app_index" => Ok(Feature::AppIndex),
        "profile" => Ok(Feature::Profile),
        _ => Err(CliError(format!("unknown feature '{}'", value))),
    }
}

fn print_help() {
    println!(
        "CoreShift Policy Daemon CLI

Usage:
  corepolicy [ -f <feature> | --feature <feature> ... | -f --all | --feature --all ]
  corepolicy status
  corepolicy category <subcommand>
  corepolicy help

Arguments:
  -f, --feature <feature>   Enable a specific feature. Can be repeated.
  -f, --feature --all       Enable all available features. This overrides any
                            other features specified.

Commands:
  status                    Print daemon status.
  category list             List all packages and their assigned categories.
  category set <pkg> <cat>  Assign a package to a profile category.
  category remove <pkg>     Remove a package from all profile categories.
  help, -h, --help          Print this help message.

If invoked with feature flags, the daemon will start.
If invoked with no arguments, this help message is printed.

Available features:
  preload, usage, pressure, app_index, profile

Available categories:
  game, social, tool, launcher, keyboard, system"
    );
}

fn parse_args(args: &[String]) -> Result<Command, CliError> {
    if args.is_empty() {
        return Ok(Command::ShowHelp);
    }

    if !args.is_empty() && args[0] == "category" {
        return parse_category_args(&args[1..]);
    }

    if args.len() == 1 {
        match args[0].as_str() {
            "" | "help" | "-h" | "--help" => return Ok(Command::ShowHelp),
            "status" => return Ok(Command::RunStatus),
            _ => {}
        }
    }

    let mut features = BTreeSet::new();
    let mut all_requested = false;
    let mut iter = args.iter().peekable();

    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "-f" | "--feature" => {
                let Some(value) = iter.next() else {
                    return Err(CliError(format!("missing value for argument '{}'", arg)));
                };
                if value == "--all" {
                    all_requested = true;
                    continue;
                }
                let feature = parse_feature_name(value)?;
                features.insert(feature);
            }
            "--all" => {
                return Err(CliError(
                    "'--all' can only be used after '-f' or '--feature'".to_string(),
                ));
            }
            "help" | "-h" | "--help" => {
                return Err(CliError(format!(
                    "unexpected help command '{}' in this position",
                    arg
                )));
            }
            other => {
                return Err(CliError(format!("unknown argument '{}'", other)));
            }
        }
    }

    if all_requested {
        return Ok(Command::StartDaemon(ALL_FEATURES.iter().copied().collect()));
    }

    if features.is_empty() {
        if !args.is_empty() {
            return Err(CliError("no features specified".to_string()));
        }
        return Ok(Command::ShowHelp);
    }

    Ok(Command::StartDaemon(features))
}

fn parse_category_args(args: &[String]) -> Result<Command, CliError> {
    let Some(subcommand) = args.first() else {
        return Err(CliError("missing category subcommand".to_string()));
    };

    match subcommand.as_str() {
        "list" => Ok(Command::CategoryList),
        "set" => {
            let Some(package) = args.get(1) else {
                return Err(CliError(
                    "missing package name for 'category set'".to_string(),
                ));
            };
            let Some(category) = args.get(2) else {
                return Err(CliError(
                    "missing category name for 'category set'".to_string(),
                ));
            };
            Ok(Command::CategorySet(package.clone(), category.clone()))
        }
        "remove" => {
            let Some(package) = args.get(1) else {
                return Err(CliError(
                    "missing package name for 'category remove'".to_string(),
                ));
            };
            Ok(Command::CategoryRemove(package.clone()))
        }
        _ => Err(CliError(format!(
            "unknown category subcommand '{}'",
            subcommand
        ))),
    }
}

fn start_daemon(features: BTreeSet<Feature>) -> ExitCode {
    if features.is_empty() {
        eprintln!("error: at least one feature must be specified to start the daemon.");
        print_help();
        return ExitCode::from(2);
    }
    logging::init();
    signals::setup();
    let config = DaemonConfig {
        preload: features.contains(&Feature::Preload),
        usage: features.contains(&Feature::Usage),
        pressure: features.contains(&Feature::Pressure),
        app_index: features.contains(&Feature::AppIndex),
        profile: features.contains(&Feature::Profile),
    };
    let mut daemon = Daemon::new(config);
    daemon.run();
    ExitCode::SUCCESS
}

fn run_status() -> ExitCode {
    let db = CategoryDatabase::default();
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

fn run_category_list() -> ExitCode {
    let db = CategoryDatabase::load();
    println!("{}", serde_json::to_string_pretty(&db).unwrap());
    ExitCode::SUCCESS
}

fn run_category_set(package: &str, category: &str) -> ExitCode {
    if !CategoryDatabase::is_supported_category(category) {
        eprintln!("error: unsupported category '{}'.", category);
        eprintln!("Available categories: game, social, tool, launcher, keyboard, system");
        return ExitCode::from(2);
    }
    let mut db = CategoryDatabase::load();
    db.add(category, package);
    if let Err(e) = db.save() {
        eprintln!("error: failed to save category database: {}", e);
        return ExitCode::from(1);
    }
    println!("'{}' set to category '{}'.", package, category);
    ExitCode::SUCCESS
}

fn run_category_remove(package: &str) -> ExitCode {
    let mut db = CategoryDatabase::load();
    db.remove(package);
    if let Err(e) = db.save() {
        eprintln!("error: failed to save category database: {}", e);
        return ExitCode::from(1);
    }
    println!("'{}' removed from all categories.", package);
    ExitCode::SUCCESS
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match parse_args(&args) {
        Ok(Command::ShowHelp) => {
            print_help();
            ExitCode::SUCCESS
        }
        Ok(Command::RunStatus) => run_status(),
        Ok(Command::StartDaemon(features)) => start_daemon(features),
        Ok(Command::CategoryList) => run_category_list(),
        Ok(Command::CategorySet(pkg, cat)) => run_category_set(&pkg, &cat),
        Ok(Command::CategoryRemove(pkg)) => run_category_remove(&pkg),
        Err(CliError(msg)) => {
            eprintln!("error: {}", msg);
            print_help();
            ExitCode::from(2)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn to_str_vec(args: &[&str]) -> Vec<String> {
        args.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn test_no_args_is_help() {
        assert_eq!(parse_args(&to_str_vec(&[])), Ok(Command::ShowHelp));
    }

    #[test]
    fn test_help_variants() {
        assert_eq!(parse_args(&to_str_vec(&["help"])), Ok(Command::ShowHelp));
        assert_eq!(parse_args(&to_str_vec(&["-h"])), Ok(Command::ShowHelp));
    }

    #[test]
    fn test_status_command() {
        assert_eq!(parse_args(&to_str_vec(&["status"])), Ok(Command::RunStatus));
    }

    #[test]
    fn test_err_unknown_arg() {
        assert!(parse_args(&to_str_vec(&["bogus"])).is_err());
    }

    #[test]
    fn test_category_list_parsing() {
        assert_eq!(
            parse_args(&to_str_vec(&["category", "list"])),
            Ok(Command::CategoryList)
        );
    }

    #[test]
    fn test_category_set_parsing() {
        assert_eq!(
            parse_args(&to_str_vec(&["category", "set", "com.foo", "game"])),
            Ok(Command::CategorySet(
                "com.foo".to_string(),
                "game".to_string()
            ))
        );
    }

    #[test]
    fn test_category_remove_parsing() {
        assert_eq!(
            parse_args(&to_str_vec(&["category", "remove", "com.foo"])),
            Ok(Command::CategoryRemove("com.foo".to_string()))
        );
    }

    #[test]
    fn test_err_category_set_missing_args() {
        assert!(parse_args(&to_str_vec(&["category", "set", "com.foo"])).is_err());
        assert!(parse_args(&to_str_vec(&["category", "set"])).is_err());
    }

    #[test]
    fn test_err_category_remove_missing_args() {
        assert!(parse_args(&to_str_vec(&["category", "remove"])).is_err());
    }

    #[test]
    fn test_err_category_unknown_subcommand() {
        assert!(parse_args(&to_str_vec(&["category", "bogus"])).is_err());
    }

    #[test]
    fn test_feature_flags_still_work() {
        let expected = Command::StartDaemon([Feature::Profile].iter().cloned().collect());
        assert_eq!(parse_args(&to_str_vec(&["-f", "profile"])), Ok(expected));
    }

    #[test]
    fn test_all_override_still_works() {
        let expected = Command::StartDaemon(ALL_FEATURES.iter().cloned().collect());
        assert_eq!(
            parse_args(&to_str_vec(&["-f", "preload", "--feature", "--all"])),
            Ok(expected)
        );
    }
}
