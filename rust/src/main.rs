use coreshift_policy::features::tweaks::{self, TweakProfile};
use coreshift_policy::runtime::status::{self, Feature, ALL_FEATURES};
use std::collections::BTreeSet;
use std::process::ExitCode;

#[derive(Debug, PartialEq, Eq)]
enum Command {
    ShowHelp,
    RunStatus,
    StartDaemon(BTreeSet<Feature>),
    CategoryList,
    CategorySet(String, String),
    CategoryRemove(String),
    TweakApply(TweakProfile),
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
        "CoreShift Policy Daemon CLI

Usage:
  corepolicy [ -f <feature> | --feature <feature> ... | -f --all | --feature --all ]
  corepolicy status
  corepolicy category <subcommand>
  corepolicy tweak <subcommand>
  corepolicy help

Feature Flags:
  -f, --feature <feature>   Enable a specific feature. Can be repeated.
  -f, --feature --all       Enable all available features. This overrides any
                            other features specified.

Commands:
  status                    Print daemon status.
  category list             List all packages and their assigned categories.
  category set <pkg> <cat>  Assign a package to a profile category.
  category remove <pkg>     Remove a package from all profile categories.
  tweak apply <profile>     Apply a system-wide tweak profile (balance, performance, power).
  tweak cache               Show the discovered system values in the tweak cache.
  tweak cache clear         Clear the tweak cache.
  help, -h, --help          Print this help message.
"
    );
}

fn parse_args(args: &[String]) -> Result<Command, CliError> {
    if args.is_empty() {
        return Ok(Command::ShowHelp);
    }

    if !args.is_empty() {
        match args[0].as_str() {
            "category" => return parse_category_args(&args[1..]),
            "tweak" => return parse_tweak_args(&args[1..]),
            _ => {}
        }
    }

    if args.len() == 1 {
        match args[0].as_str() {
            "" | "help" | "-h" | "--help" => return Ok(Command::ShowHelp),
            "status" => return Ok(Command::RunStatus),
            _ => {}
        }
    }

    // Fallback to feature parsing
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
            other => {
                return Err(CliError(format!("unknown argument '{}'", other)));
            }
        }
    }

    if all_requested {
        return Ok(Command::StartDaemon(ALL_FEATURES.iter().copied().collect()));
    }
    if features.is_empty() {
        return Err(CliError("no features specified or unknown command".to_string()));
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
            let pkg = args.get(1).ok_or(CliError("missing package name".to_string()))?;
            let cat = args.get(2).ok_or(CliError("missing category name".to_string()))?;
            Ok(Command::CategorySet(pkg.clone(), cat.clone()))
        }
        "remove" => {
            let pkg = args.get(1).ok_or(CliError("missing package name".to_string()))?;
            Ok(Command::CategoryRemove(pkg.clone()))
        }
        _ => Err(CliError(format!("unknown category subcommand '{}'", subcommand))),
    }
}

fn parse_tweak_args(args: &[String]) -> Result<Command, CliError> {
    let Some(subcommand) = args.first() else {
        return Err(CliError("missing tweak subcommand".to_string()));
    };

    match subcommand.as_str() {
        "apply" => {
            let profile_name = args.get(1).ok_or(CliError("missing profile name".to_string()))?;
            let profile = profile_name.parse::<TweakProfile>().map_err(|e| CliError(e.to_string()))?;
            Ok(Command::TweakApply(profile))
        }
        "cache" => {
            if args.get(1).map(|s| s.as_str()) == Some("clear") {
                Ok(Command::TweakClearCache)
            } else {
                Ok(Command::TweakShowCache)
            }
        }
        _ => Err(CliError(format!("unknown tweak subcommand '{}'", subcommand))),
    }
}


fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match parse_args(&args) {
        Ok(Command::ShowHelp) => {
            print_help();
            ExitCode::SUCCESS
        }
        Ok(Command::RunStatus) => status::run_status_cli(),
        Ok(Command::StartDaemon(features)) => status::start_daemon(features),
        Ok(Command::CategoryList) => status::run_category_list_cli(),
        Ok(Command::CategorySet(pkg, cat)) => status::run_category_set_cli(&pkg, &cat),
        Ok(Command::CategoryRemove(pkg)) => status::run_category_remove_cli(&pkg),
        Ok(Command::TweakApply(profile)) => {
            let summary = tweaks::apply_tweak_profile(profile);
            println!("{}", serde_json::to_string_pretty(&summary).unwrap());
            if summary.failed_writes > 0 { ExitCode::from(1) } else { ExitCode::SUCCESS }
        }
        Ok(Command::TweakShowCache) => {
            let cache = tweaks::TweakCache::load();
            println!("{}", serde_json::to_string_pretty(&cache).unwrap());
            ExitCode::SUCCESS
        }
        Ok(Command::TweakClearCache) => {
            if let Err(e) = tweaks::TweakCache::clear() {
                eprintln!("error: failed to clear tweak cache: {}", e);
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
    use coreshift_policy::runtime::status;

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
        let expected = Command::StartDaemon(
            [status::Feature::Profile].iter().cloned().collect(),
        );
        assert_eq!(parse_args(&to_str_vec(&["-f", "profile"])), Ok(expected));
    }

    #[test]
    fn test_all_override_still_works() {
        let expected = Command::StartDaemon(status::ALL_FEATURES.iter().cloned().collect());
        assert_eq!(
            parse_args(&to_str_vec(&["-f", "preload", "--feature", "--all"])),
            Ok(expected)
        );
    }

    #[test]
    fn test_tweak_apply_parsing() {
        assert_eq!(
            parse_args(&to_str_vec(&["tweak", "apply", "performance"])),
            Ok(Command::TweakApply(TweakProfile::Performance))
        );
    }

    #[test]
    fn test_tweak_cache_parsing() {
        assert_eq!(parse_args(&to_str_vec(&["tweak", "cache"])), Ok(Command::TweakShowCache));
    }

    #[test]
    fn test_tweak_cache_clear_parsing() {
        assert_eq!(parse_args(&to_str_vec(&["tweak", "cache", "clear"])), Ok(Command::TweakClearCache));
    }

    #[test]
    fn test_err_tweak_invalid_profile() {
        assert!(parse_args(&to_str_vec(&["tweak", "apply", "bogus"])).is_err());
    }

    #[test]
    fn test_err_tweak_missing_args() {
        assert!(parse_args(&to_str_vec(&["tweak", "apply"])).is_err());
        assert!(parse_args(&to_str_vec(&["tweak"])).is_err());
    }
}
