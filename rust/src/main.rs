use coreshift_policy::api::features::{self as feature_api, TweakProfile};
use coreshift_policy::api::status::{self as status_api, ALL_FEATURES, Feature};
use std::collections::BTreeSet;
use std::process::ExitCode;

#[derive(Debug, PartialEq, Eq)]
enum Command {
    ShowHelp,
    RunStatus,
    StartDaemon(BTreeSet<Feature>),
    TweakRun(String),
    TweakPreset(TweakProfile),
    TweakShowCache,
    TweakClearCache,
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
        "CoreShift Policy CLI

Usage:
  corepolicy help
  corepolicy status
  corepolicy start [--all] [-f FEATURE...]
  corepolicy tweak run <command...>
  corepolicy tweak preset <profile>
  corepolicy tweak cache
  corepolicy tweak cache clear

Features:
  preload
  usage
  pressure
  app_index
  profile"
    );
}

fn parse_args(args: &[String]) -> Result<Command, CliError> {
    if args.is_empty() {
        return Ok(Command::ShowHelp);
    }

    match args[0].as_str() {
        "" | "help" | "-h" | "--help" => return Ok(Command::ShowHelp),
        "status" if args.len() == 1 => return Ok(Command::RunStatus),
        "start" => return parse_start_args(&args[1..]),
        "tweak" => return parse_tweak_args(&args[1..]),
        _ => {}
    }

    parse_start_args(args)
}

fn parse_start_args(args: &[String]) -> Result<Command, CliError> {
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
            "--all" => all_requested = true,
            "-p" => {
                return Err(CliError(
                    "-p has been removed. Use -f or --feature.".to_string(),
                ));
            }
            other => return Err(CliError(format!("unknown argument '{}'", other))),
        }
    }

    if all_requested {
        return Ok(Command::StartDaemon(ALL_FEATURES.iter().copied().collect()));
    }
    if features.is_empty() {
        return Err(CliError(
            "no features specified or unknown command".to_string(),
        ));
    }
    Ok(Command::StartDaemon(features))
}

fn parse_tweak_args(args: &[String]) -> Result<Command, CliError> {
    let Some(subcommand) = args.first() else {
        return Err(CliError("missing tweak subcommand".to_string()));
    };

    match subcommand.as_str() {
        "run" => {
            if args.len() < 2 {
                return Err(CliError("missing tweak command".to_string()));
            }
            Ok(Command::TweakRun(args[1..].join(" ")))
        }
        "preset" => {
            let profile_name = args
                .get(1)
                .ok_or(CliError("missing profile name".to_string()))?;
            let profile = profile_name
                .parse::<TweakProfile>()
                .map_err(|e| CliError(e.to_string()))?;
            Ok(Command::TweakPreset(profile))
        }
        "cache" => {
            if args.get(1).map(|s| s.as_str()) == Some("clear") {
                Ok(Command::TweakClearCache)
            } else {
                Ok(Command::TweakShowCache)
            }
        }
        _ => Err(CliError(format!(
            "unknown tweak subcommand '{}'",
            subcommand
        ))),
    }
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match parse_args(&args) {
        Ok(Command::ShowHelp) => {
            print_help();
            ExitCode::SUCCESS
        }
        Ok(Command::RunStatus) => status_api::run_status_cli(),
        Ok(Command::StartDaemon(features)) => status_api::start_daemon(features),
        Ok(Command::TweakRun(command_line)) => {
            let summary = match feature_api::run_tweak_command_line("cli", &command_line) {
                Ok(summary) => summary,
                Err(error) => {
                    eprintln!("error: {}", error);
                    return ExitCode::from(2);
                }
            };
            println!("{}", serde_json::to_string_pretty(&summary).unwrap());
            if summary.failed_writes > 0 {
                ExitCode::from(1)
            } else {
                ExitCode::SUCCESS
            }
        }
        Ok(Command::TweakPreset(profile)) => {
            let summary = feature_api::apply_tweak_preset(profile);
            println!("{}", serde_json::to_string_pretty(&summary).unwrap());
            if summary.failed_writes > 0 {
                ExitCode::from(1)
            } else {
                ExitCode::SUCCESS
            }
        }
        Ok(Command::TweakShowCache) => {
            let cache = feature_api::TweakCache::load();
            println!("{}", serde_json::to_string_pretty(&cache).unwrap());
            ExitCode::SUCCESS
        }
        Ok(Command::TweakClearCache) => {
            if let Err(error) = feature_api::TweakCache::clear() {
                eprintln!("error: failed to clear tweak cache: {}", error);
                return ExitCode::from(1);
            }
            println!("Tweak cache cleared.");
            ExitCode::SUCCESS
        }
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
        assert_eq!(parse_args(&to_str_vec(&["--help"])), Ok(Command::ShowHelp));
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
    fn test_start_feature_parsing() {
        assert_eq!(
            parse_args(&to_str_vec(&["start", "-f", "preload"])),
            Ok(Command::StartDaemon(BTreeSet::from([Feature::Preload])))
        );
        assert_eq!(
            parse_args(&to_str_vec(&["start", "--feature", "profile"])),
            Ok(Command::StartDaemon(BTreeSet::from([Feature::Profile])))
        );
        assert_eq!(
            parse_args(&to_str_vec(&["start", "--all"])),
            Ok(Command::StartDaemon(ALL_FEATURES.iter().copied().collect()))
        );
    }

    #[test]
    fn test_tweak_run_parsing() {
        assert_eq!(
            parse_args(&to_str_vec(&[
                "tweak",
                "run",
                "tweak",
                "write",
                "/proc/sys/vm/swappiness",
                "5"
            ])),
            Ok(Command::TweakRun(
                "tweak write /proc/sys/vm/swappiness 5".to_string()
            ))
        );
    }

    #[test]
    fn test_tweak_preset_parsing() {
        assert_eq!(
            parse_args(&to_str_vec(&["tweak", "preset", "performance"])),
            Ok(Command::TweakPreset(TweakProfile::Performance))
        );
    }

    #[test]
    fn test_tweak_cache_parsing() {
        assert_eq!(
            parse_args(&to_str_vec(&["tweak", "cache"])),
            Ok(Command::TweakShowCache)
        );
    }

    #[test]
    fn test_tweak_cache_clear_parsing() {
        assert_eq!(
            parse_args(&to_str_vec(&["tweak", "cache", "clear"])),
            Ok(Command::TweakClearCache)
        );
    }

    #[test]
    fn test_removed_p_flag() {
        assert!(parse_args(&to_str_vec(&["start", "-p", "preload"])).is_err());
    }
}
