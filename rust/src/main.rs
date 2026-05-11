fn main() {
    let args = std::env::args().collect::<Vec<_>>();
    match args.as_slice().get(1).map(String::as_str) {
        Some("daemon") if args.len() == 2 => {
            if let Err(err) = coreshift_policy::run_corepolicy_daemon() {
                eprintln!("corepolicy daemon: {err}");
                std::process::exit(1);
            }
        }
        Some("status") if args.len() == 2 => {
            let config = match coreshift_policy::load_daemon_config() {
                Ok(config) => config,
                Err(err) => {
                    eprintln!("corepolicy status: {err}");
                    std::process::exit(1);
                }
            };
            match coreshift_policy::daemon_request(&config.socket, "STATUS") {
                Ok(status) => print!("{status}"),
                Err(err) => {
                    println!("daemon=offline");
                    eprintln!("corepolicy status: daemon offline: {err}");
                    std::process::exit(1);
                }
            }
        }
        Some("restart") if args.len() == 2 => {
            let config = match coreshift_policy::load_daemon_config() {
                Ok(config) => config,
                Err(err) => {
                    eprintln!("corepolicy restart: {err}");
                    std::process::exit(1);
                }
            };
            match coreshift_policy::daemon_request(&config.socket, "RESTART") {
                Ok(reply) => print!("{reply}"),
                Err(err) => {
                    eprintln!("corepolicy restart: daemon offline: {err}");
                    std::process::exit(1);
                }
            }
        }
        Some("watch") if args.len() == 2 => {
            let config = match coreshift_policy::load_daemon_config() {
                Ok(config) => config,
                Err(err) => {
                    eprintln!("corepolicy watch: {err}");
                    std::process::exit(1);
                }
            };
            if let Err(err) = coreshift_policy::daemon_watch(&config.socket) {
                eprintln!("corepolicy watch: daemon offline: {err}");
                std::process::exit(1);
            }
        }
        Some("preload-package") if args.len() == 3 => {
            let config = match coreshift_policy::load_daemon_config() {
                Ok(config) => config,
                Err(err) => {
                    eprintln!("corepolicy preload-package: {err}");
                    std::process::exit(1);
                }
            };
            let game_classifier = match coreshift_policy::load_game_classifier(&config.game) {
                Ok(classifier) => classifier,
                Err(err) => {
                    eprintln!("corepolicy preload-package: {err}");
                    std::process::exit(1);
                }
            };
            let foreground = coreshift_policy::AndroidForegroundConfig::default();
            match coreshift_policy::preload_package_by_name_with_game(
                &args[2],
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
                }
                Err(err) => {
                    eprintln!("corepolicy preload-package: {err}");
                    std::process::exit(1);
                }
            }
        }
        Some("stats") if args.len() == 2 => {
            let config = match coreshift_policy::load_daemon_config() {
                Ok(config) => config,
                Err(err) => {
                    eprintln!("corepolicy stats: {err}");
                    std::process::exit(1);
                }
            };
            match coreshift_policy::read_stats(&config.stats.path) {
                Ok(stats) => print!("{}", coreshift_policy::format_stats(&stats)),
                Err(err) => {
                    eprintln!("corepolicy stats: {err}");
                    std::process::exit(1);
                }
            }
        }
        Some("stats-reset") if args.len() == 2 => {
            let config = match coreshift_policy::load_daemon_config() {
                Ok(config) => config,
                Err(err) => {
                    eprintln!("corepolicy stats-reset: {err}");
                    std::process::exit(1);
                }
            };
            if let Err(err) = coreshift_policy::reset_stats_path(&config.stats.path) {
                eprintln!("corepolicy stats-reset: {err}");
                std::process::exit(1);
            }
        }
        Some("game-apply") if args.len() == 2 => {
            let config = match coreshift_policy::load_daemon_config() {
                Ok(config) => config,
                Err(err) => {
                    eprintln!("corepolicy game-apply: {err}");
                    std::process::exit(1);
                }
            };
            let game_list = match coreshift_policy::load_game_list(&config.game.list_path) {
                Ok(list) => list,
                Err(err) => {
                    eprintln!("corepolicy game-apply: {err}");
                    std::process::exit(1);
                }
            };
            let targets = match installed_game_targets(&game_list) {
                Ok(targets) => targets,
                Err(err) => {
                    eprintln!("corepolicy game-apply: {err}");
                    std::process::exit(1);
                }
            };
            let report =
                coreshift_policy::apply_game_interventions(&config.game, &targets, &config.log);
            println!(
                "game interventions attempted={} ok={} failed={} dry-run={}",
                report.attempted, report.succeeded, report.failed, report.dry_run
            );
        }
        Some("game-list") if args.len() == 2 => {
            let config = match coreshift_policy::load_daemon_config() {
                Ok(config) => config,
                Err(err) => {
                    eprintln!("corepolicy game-list: {err}");
                    std::process::exit(1);
                }
            };
            let game_list = match coreshift_policy::load_game_list(&config.game.list_path) {
                Ok(list) => list,
                Err(err) => {
                    eprintln!("corepolicy game-list: {err}");
                    std::process::exit(1);
                }
            };
            for package in game_list.packages() {
                println!(
                    "{} tier={:?} intervention={}",
                    package,
                    config.game.preload_tier,
                    config.game.intervention.mode.as_str()
                );
            }
        }
        Some("game-revert") if args.len() == 3 => {
            let config = match coreshift_policy::load_daemon_config() {
                Ok(config) => config,
                Err(err) => {
                    eprintln!("corepolicy game-revert: {err}");
                    std::process::exit(1);
                }
            };
            let report =
                coreshift_policy::revert_game_intervention(&config.game, &args[2], &config.log);
            println!(
                "game revert attempted={} ok={} failed={} dry-run={}",
                report.attempted, report.succeeded, report.failed, report.dry_run
            );
        }
        _ => {
            eprintln!(
                "usage: corepolicy daemon|status|restart|watch|preload-package <package>|stats|stats-reset|game-apply|game-list|game-revert <package>"
            );
            std::process::exit(2);
        }
    }
}

fn installed_game_targets(
    game_list: &coreshift_policy::GameList,
) -> std::io::Result<coreshift_policy::GameList> {
    let foreground = coreshift_policy::AndroidForegroundConfig::default();
    let argv = coreshift_policy::package_install_list_argv(&foreground.cmd_path, 0);
    let output = std::process::Command::new(&argv[0])
        .args(&argv[1..])
        .output()?;
    if !output.status.success() {
        return Err(std::io::Error::other("cmd package list packages failed"));
    }
    let packages = coreshift_policy::parse_android_package_list_stdout(&output.stdout)
        .into_iter()
        .filter(|entry| game_list.contains(&entry.package))
        .map(|entry| entry.package)
        .collect();
    Ok(coreshift_policy::GameList::from_packages(packages))
}
