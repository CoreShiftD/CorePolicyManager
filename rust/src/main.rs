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
  corepolicy help

Arguments:
  -f, --feature <feature>   Enable a specific feature. Can be repeated.
  -f, --feature --all       Enable all available features. This overrides any
                            other features specified.

Commands:
  status                    Print daemon status.
  help, -h, --help          Print this help message.

If invoked with feature flags, the daemon will start.
If invoked with no arguments, this help message is printed.

Available features:
  preload, usage, pressure, app_index, profile"
    );
}

fn parse_args(args: &[String]) -> Result<Command, CliError> {
    if args.is_empty() {
        return Ok(Command::ShowHelp);
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
        // This case can be hit if only invalid flags were passed, which is an error.
        // Or if no flags were passed but the arg count > 1 (e.g. "corepolicy foo bar")
        // which the default case in the loop already handles.
        // We can treat this as an implicit request for help.
        // Let's refine this to be an error as `corepolicy -f` is an error, not help.
        if !args.is_empty() {
            return Err(CliError("no features specified".to_string()));
        }
        return Ok(Command::ShowHelp);
    }

    Ok(Command::StartDaemon(features))
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

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match parse_args(&args) {
        Ok(Command::ShowHelp) => {
            print_help();
            ExitCode::SUCCESS
        }
        Ok(Command::RunStatus) => run_status(),
        Ok(Command::StartDaemon(features)) => start_daemon(features),
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
    fn test_empty_arg_is_help() {
        assert_eq!(parse_args(&to_str_vec(&[""])), Ok(Command::ShowHelp));
    }

    #[test]
    fn test_help_variants() {
        assert_eq!(parse_args(&to_str_vec(&["help"])), Ok(Command::ShowHelp));
        assert_eq!(parse_args(&to_str_vec(&["-h"])), Ok(Command::ShowHelp));
        assert_eq!(parse_args(&to_str_vec(&["--help"])), Ok(Command::ShowHelp));
    }

    #[test]
    fn test_status_command() {
        assert_eq!(parse_args(&to_str_vec(&["status"])), Ok(Command::RunStatus));
    }

    #[test]
    fn test_parse_single_feature() {
        let expected = Command::StartDaemon([Feature::Preload].iter().cloned().collect());
        assert_eq!(parse_args(&to_str_vec(&["-f", "preload"])), Ok(expected));
    }

    #[test]
    fn test_parse_multiple_features() {
        let expected = Command::StartDaemon(
            [Feature::Preload, Feature::Profile]
                .iter()
                .cloned()
                .collect(),
        );
        assert_eq!(
            parse_args(&to_str_vec(&["-f", "preload", "--feature", "profile"])),
            Ok(expected)
        );
    }

    #[test]
    fn test_all_overrides_other_features() {
        let expected = Command::StartDaemon(ALL_FEATURES.iter().cloned().collect());
        assert_eq!(
            parse_args(&to_str_vec(&["-f", "preload", "-f", "--all"])),
            Ok(expected)
        );
    }

    #[test]
    fn test_all_long_overrides() {
        let expected = Command::StartDaemon(ALL_FEATURES.iter().cloned().collect());
        assert_eq!(
            parse_args(&to_str_vec(&["--feature", "preload", "--feature", "--all"])),
            Ok(expected)
        );
    }

    #[test]
    fn test_err_unknown_arg() {
        assert!(parse_args(&to_str_vec(&["bogus"])).is_err());
        assert!(parse_args(&to_str_vec(&["-f", "preload", "bogus"])).is_err());
    }

    #[test]
    fn test_err_start_is_not_a_command() {
        let err = parse_args(&to_str_vec(&["start", "-f", "preload"])).unwrap_err();
        assert_eq!(err, CliError("unknown argument 'start'".to_string()));
    }

    #[test]
    fn test_err_missing_feature_value() {
        assert!(parse_args(&to_str_vec(&["-f"])).is_err());
    }

    #[test]
    fn test_err_unknown_feature_name() {
        assert!(parse_args(&to_str_vec(&["--feature", "bogus"])).is_err());
    }

    #[test]
    fn test_err_standalone_all() {
        assert!(parse_args(&to_str_vec(&["--all"])).is_err());
    }

    #[test]
    fn test_err_help_in_wrong_position() {
        assert!(parse_args(&to_str_vec(&["-f", "preload", "help"])).is_err());
        assert!(parse_args(&to_str_vec(&["-f", "preload", "--help"])).is_err());
    }
}
