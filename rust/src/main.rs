fn main() {
    let args = std::env::args().collect::<Vec<_>>();
    let result = match args.as_slice().get(1).map(String::as_str) {
        Some("daemon") if args.len() == 2 => run_daemon(),
        Some("status") if args.len() == 2 => status(),
        Some("restart") if args.len() == 2 => restart(),
        Some("watch") if args.len() == 2 => watch(),
        Some("stats") if args.len() == 2 => stats(false),
        Some("stats") if args.get(2).map(String::as_str) == Some("raw") && args.len() == 3 => {
            stats(true)
        }
        Some("stats") if args.get(2).map(String::as_str) == Some("reset") && args.len() == 3 => {
            stats_reset("corepolicy stats reset")
        }
        Some("stats-reset") if args.len() == 2 => stats_reset("corepolicy stats-reset"),
        Some("gamelist") if args.len() == 2 => gamelist(),
        Some("debug") => debug(&args),
        Some("preload-package") if args.len() == 3 => {
            preload_package(&args[2], "corepolicy preload-package")
        }
        Some("game-apply") if args.len() == 2 => game_apply("corepolicy game-apply"),
        Some("game-list") if args.len() == 2 => gamelist_raw("corepolicy game-list"),
        Some("game-revert") if args.len() == 3 => game_revert(&args[2], "corepolicy game-revert"),
        _ => usage(),
    };

    if let Err(code) = result {
        std::process::exit(code);
    }
}

fn debug(args: &[String]) -> Result<(), i32> {
    match args.get(2).map(String::as_str) {
        Some("preload-package") if args.len() == 4 => {
            preload_package(&args[3], "corepolicy debug preload-package")
        }
        Some("game-apply") if args.len() == 3 => game_apply("corepolicy debug game-apply"),
        Some("game-revert") if args.len() == 4 => {
            game_revert(&args[3], "corepolicy debug game-revert")
        }
        Some("gamelist-raw") if args.len() == 3 => gamelist_raw("corepolicy debug gamelist-raw"),
        _ => usage(),
    }
}

fn run_daemon() -> Result<(), i32> {
    if let Err(err) = coreshift_policy::run_corepolicy_daemon() {
        eprintln!("corepolicy daemon: {err}");
        return Err(1);
    }
    Ok(())
}

fn status() -> Result<(), i32> {
    match coreshift_policy::daemon_request(
        coreshift_policy::COREPOLICY_DEFAULT_ABSTRACT_SOCKET,
        "STATUS",
    ) {
        Ok(status) => print!("{status}"),
        Err(err) => {
            println!("daemon=offline");
            eprintln!("corepolicy status: daemon offline: {err}");
            return Err(1);
        }
    }
    Ok(())
}

fn restart() -> Result<(), i32> {
    match coreshift_policy::daemon_request(
        coreshift_policy::COREPOLICY_DEFAULT_ABSTRACT_SOCKET,
        "RESTART",
    ) {
        Ok(reply) => print!("{reply}"),
        Err(err) => {
            eprintln!("corepolicy restart: daemon offline: {err}");
            return Err(1);
        }
    }
    Ok(())
}

fn watch() -> Result<(), i32> {
    if let Err(err) =
        coreshift_policy::daemon_watch(coreshift_policy::COREPOLICY_DEFAULT_ABSTRACT_SOCKET)
    {
        eprintln!("corepolicy watch: daemon offline: {err}");
        return Err(1);
    }
    Ok(())
}

fn preload_package(package: &str, label: &str) -> Result<(), i32> {
    let config = load_config(label)?;
    let game_classifier = match coreshift_policy::load_game_classifier(&config.game) {
        Ok(classifier) => classifier,
        Err(err) => {
            eprintln!("{label}: {err}");
            return Err(1);
        }
    };
    let foreground = coreshift_policy::AndroidForegroundConfig::default();
    match coreshift_policy::preload_package_by_name_with_game(
        package,
        &foreground,
        &config.preload,
        &game_classifier,
    ) {
        Ok(report) => {
            println!(
                "preloaded {} bytes from {} files ({} skipped, {} failed)",
                report.preloaded_bytes(),
                report.preloaded_files(),
                report.skipped_count(),
                report.failed_count()
            );
            for skipped in report.report.skipped {
                eprintln!("skip {}: {}", skipped.path.display(), skipped.error);
            }
            if config.log.preload_enabled() {
                eprintln!(
                    "preload-package package={} tier={} source={}",
                    package,
                    report.adaptive_tier.as_str(),
                    report.tier_source.as_str()
                );
                for target in report.targets {
                    eprintln!(
                        "preload-target method={:?} len={} path={}",
                        target.method,
                        target.len,
                        target.path.display()
                    );
                }
            }
        }
        Err(err) => {
            eprintln!("{label}: {err}");
            return Err(1);
        }
    }
    Ok(())
}

fn stats(raw: bool) -> Result<(), i32> {
    let config = load_config("corepolicy stats")?;
    match coreshift_policy::read_stats(&config.stats.path) {
        Ok(stats) if raw => print!("{}", coreshift_policy::format_stats(&stats)),
        Ok(stats) => print!("{}", format_pretty_stats(&stats)),
        Err(err) => {
            eprintln!("corepolicy stats: {err}");
            return Err(1);
        }
    }
    Ok(())
}

fn format_pretty_stats(stats: &[coreshift_policy::UsageStat]) -> String {
    if stats.is_empty() {
        return String::from("No usage stats collected yet.\n");
    }

    let mut stats = stats.to_vec();
    stats.sort_by(|a, b| {
        b.foreground_ms
            .cmp(&a.foreground_ms)
            .then_with(|| a.package.cmp(&b.package))
            .then_with(|| a.uid.cmp(&b.uid))
    });

    let mut out = String::from("CoreShift usage stats\n\n");
    for (idx, stat) in stats.iter().enumerate() {
        if idx > 0 {
            out.push('\n');
        }
        out.push_str(&format!(
            "{}. {}\n   uid: {}\n   sessions: {}\n   foreground: {}\n",
            idx + 1,
            stat.package,
            stat.uid,
            stat.sessions,
            format_foreground_duration(stat.foreground_ms)
        ));
    }
    out
}

fn format_foreground_duration(ms: u64) -> String {
    let total_seconds = ms / 1_000;
    let seconds = total_seconds % 60;
    let total_minutes = total_seconds / 60;
    let minutes = total_minutes % 60;
    let total_hours = total_minutes / 60;
    let hours = total_hours % 24;
    let days = total_hours / 24;

    if days > 0 {
        format!("{days}d {hours:02}h")
    } else if total_hours > 0 {
        format!("{total_hours}h {minutes:02}m")
    } else if total_minutes > 0 {
        format!("{total_minutes}m {seconds:02}s")
    } else {
        format!("{seconds}s")
    }
}

fn stats_reset(label: &str) -> Result<(), i32> {
    let config = load_config(label)?;
    if let Err(err) = coreshift_policy::reset_stats_path(&config.stats.path) {
        eprintln!("{label}: {err}");
        return Err(1);
    }
    Ok(())
}

fn gamelist() -> Result<(), i32> {
    let config = load_config("corepolicy gamelist")?;
    let game_list = load_game_list(&config.game.list_path, "corepolicy gamelist")?;
    let foreground_config = android_foreground_config_from_env();
    let foreground =
        match coreshift_policy::AndroidForegroundPackageProvider::new(foreground_config) {
            Ok(provider) => provider,
            Err(err) => {
                eprintln!("corepolicy gamelist: {err}");
                return Err(1);
            }
        };
    let targets = foreground.cached_installed_game_targets(&game_list);
    for package in targets.packages() {
        println!("{package}");
    }
    if targets.is_empty() {
        eprintln!("corepolicy gamelist: no installed gamelist packages");
    }
    Ok(())
}

fn game_apply(label: &str) -> Result<(), i32> {
    let config = load_config(label)?;
    let game_list = load_game_list(&config.game.list_path, label)?;
    let foreground_config = android_foreground_config_from_env();
    let foreground =
        match coreshift_policy::AndroidForegroundPackageProvider::new(foreground_config) {
            Ok(provider) => provider,
            Err(err) => {
                eprintln!("{label}: {err}");
                return Err(1);
            }
        };
    let targets = foreground.cached_installed_game_targets(&game_list);
    let report = coreshift_policy::apply_game_downscales(&config.game, &targets, &config.log);
    println!(
        "game downscale attempted={} ok={} failed={} dry-run={}",
        report.attempted, report.succeeded, report.failed, report.dry_run
    );
    Ok(())
}

fn gamelist_raw(label: &str) -> Result<(), i32> {
    let config = load_config(label)?;
    let game_list = load_game_list(&config.game.list_path, label)?;
    for package in game_list.packages() {
        println!(
            "{} tier={:?} downscale=performance factor={}",
            package,
            config.game.preload_tier,
            config.game.downscale.downscale_factor.as_str()
        );
    }
    Ok(())
}

fn game_revert(package: &str, label: &str) -> Result<(), i32> {
    let config = load_config(label)?;
    let report = coreshift_policy::revert_game_downscale(&config.game, package, &config.log);
    println!(
        "game downscale revert attempted={} ok={} failed={} dry-run={}",
        report.attempted, report.succeeded, report.failed, report.dry_run
    );
    Ok(())
}

fn load_config(label: &str) -> Result<coreshift_policy::DaemonConfig, i32> {
    match coreshift_policy::load_daemon_config() {
        Ok(config) => Ok(config),
        Err(err) => {
            eprintln!("{label}: {err}");
            Err(1)
        }
    }
}

fn load_game_list(path: &std::path::Path, label: &str) -> Result<coreshift_policy::GameList, i32> {
    match coreshift_policy::load_game_list(path) {
        Ok(list) => Ok(list),
        Err(err) => {
            eprintln!("{label}: {err}");
            Err(1)
        }
    }
}

fn android_foreground_config_from_env() -> coreshift_policy::AndroidForegroundConfig {
    let mut config = coreshift_policy::AndroidForegroundConfig::default();
    if let Ok(path) = std::env::var("COREPOLICY_ANDROID_CMD_PATH") {
        config.cmd_path = path.into();
    }
    if let Ok(path) = std::env::var("COREPOLICY_ANDROID_BLOCKLIST_PATH") {
        config.blocklist_path = path.into();
    }
    if let Ok(path) = std::env::var("COREPOLICY_ANDROID_PACKAGES_XML_PATH") {
        config.packages_xml_path = path.into();
    }
    config
}

fn usage() -> Result<(), i32> {
    eprintln!("usage: corepolicy status|restart|watch|stats [raw|reset]|gamelist|debug <command>");
    Err(2)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pretty_stats_matches_policy_cli_shape() {
        let stats = vec![
            coreshift_policy::UsageStat {
                package: "com.seconds".to_string(),
                uid: 10001,
                sessions: 1,
                foreground_ms: 26_000,
                last_seen_ms: 111_111_111,
            },
            coreshift_policy::UsageStat {
                package: "com.days".to_string(),
                uid: 10004,
                sessions: 4,
                foreground_ms: 97_200_000,
                last_seen_ms: 444_444_444,
            },
        ];

        assert_eq!(
            format_pretty_stats(&stats),
            concat!(
                "CoreShift usage stats\n\n",
                "1. com.days\n",
                "   uid: 10004\n",
                "   sessions: 4\n",
                "   foreground: 1d 03h\n\n",
                "2. com.seconds\n",
                "   uid: 10001\n",
                "   sessions: 1\n",
                "   foreground: 26s\n",
            )
        );
    }

    #[test]
    fn pretty_stats_empty_message_matches_policy_cli() {
        assert_eq!(format_pretty_stats(&[]), "No usage stats collected yet.\n");
    }
}
