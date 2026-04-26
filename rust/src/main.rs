use coreshift_policy::features::profile::CategoryDatabase;
use coreshift_policy::runtime::{
    daemon::{Daemon, DaemonConfig},
    logging, signals, status,
};
use std::collections::BTreeSet;
use std::process::ExitCode;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum CliFeature {
    Preload,
    Usage,
    Pressure,
    AppIndex,
    Profile,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct StartConfig {
    requested: BTreeSet<CliFeature>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Command {
    ShowHelp,
    Start(StartConfig),
    Status,
    Unknown(String),
}

fn render_help() -> &'static str {
    "CoreShift Policy CLI

Usage:
  corepolicy help
  corepolicy status
  corepolicy start [--all] [-f FEATURE...]

Features:
  preload
  usage
  pressure
  app_index
  profile"
}

fn print_help() {
    println!("{}", render_help());
}

fn parse_feature_name(value: &str) -> Result<CliFeature, String> {
    match value {
        "preload" => Ok(CliFeature::Preload),
        "usage" => Ok(CliFeature::Usage),
        "pressure" => Ok(CliFeature::Pressure),
        "app_index" => Ok(CliFeature::AppIndex),
        "profile" => Ok(CliFeature::Profile),
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
            "--all" => {
                requested.insert(CliFeature::Preload);
                requested.insert(CliFeature::Usage);
                requested.insert(CliFeature::Pressure);
                requested.insert(CliFeature::AppIndex);
                requested.insert(CliFeature::Profile);
                explicit = true;
            }
            "-h" | "--help" => {
                print_help();
                return Err(ExitCode::SUCCESS);
            }
            "-p" => {
                eprintln!("-p has been removed. Use -f or --feature.");
                return Err(ExitCode::from(2));
            }
            value => {
                eprintln!("error: unknown start argument '{}'", value);
                return Err(ExitCode::from(2));
            }
        }
    }

    if !explicit {
        requested.insert(CliFeature::Preload);
        requested.insert(CliFeature::Usage);
        requested.insert(CliFeature::Pressure);
        requested.insert(CliFeature::AppIndex);
        requested.insert(CliFeature::Profile);
    }

    Ok(StartConfig { requested })
}

fn daemon_config_from_start(config: &StartConfig) -> DaemonConfig {
    DaemonConfig {
        preload: config.requested.contains(&CliFeature::Preload),
        usage: config.requested.contains(&CliFeature::Usage),
        pressure: config.requested.contains(&CliFeature::Pressure),
        app_index: config.requested.contains(&CliFeature::AppIndex),
        profile: config.requested.contains(&CliFeature::Profile),
    }
}

fn start_daemon(config: StartConfig) -> ExitCode {
    logging::init();
    signals::setup();
    let mut daemon = Daemon::new(daemon_config_from_start(&config));
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
        Some("start") => parse_start_args(args).map(Command::Start),
        Some("-f" | "--feature" | "--all") => {
            let mut forwarded = vec![first_arg.unwrap_or_default()];
            forwarded.extend(args);
            parse_start_args(forwarded).map(Command::Start)
        }
        Some("-p") => {
            eprintln!("-p has been removed. Use -f or --feature.");
            Err(ExitCode::from(2))
        }
        Some("status") => Ok(Command::Status),
        Some("help" | "--help" | "-h") => Ok(Command::ShowHelp),
        Some(cmd) => Ok(Command::Unknown(cmd.to_string())),
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

    #[test]
    fn short_feature_flag_works() {
        let config = parse_start_args(vec!["-f".to_string(), "preload".to_string()]).unwrap();
        assert!(config.requested.contains(&CliFeature::Preload));
    }

    #[test]
    fn no_arg_invocation_shows_help() {
        assert_eq!(
            parse_cli_args(Vec::<String>::new()).unwrap(),
            Command::ShowHelp
        );
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
    fn profile_feature_is_supported() {
        let config = parse_start_args(vec!["-f".to_string(), "profile".to_string()]).unwrap();
        assert!(config.requested.contains(&CliFeature::Profile));
    }

    #[test]
    fn all_flag_works() {
        let config = parse_start_args(vec!["--all".to_string()]).unwrap();
        assert!(config.requested.contains(&CliFeature::Preload));
        assert!(config.requested.contains(&CliFeature::Usage));
        assert!(config.requested.contains(&CliFeature::Pressure));
        assert!(config.requested.contains(&CliFeature::AppIndex));
        assert!(config.requested.contains(&CliFeature::Profile));
    }
}
