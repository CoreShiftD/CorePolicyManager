fn main() {
    let args = std::env::args().collect::<Vec<_>>();
    match args.as_slice().get(1).map(String::as_str) {
        Some("daemon") if args.len() == 2 => {
            if let Err(err) = coreshift_policy::run_corepolicy_daemon() {
                eprintln!("corepolicy daemon: {err}");
                std::process::exit(1);
            }
        }
        Some("preload-package") if args.len() == 3 => {
            let foreground = coreshift_policy::AndroidForegroundConfig::default();
            let preload = coreshift_policy::PreloadConfig::default();
            match coreshift_policy::preload_package_by_name(&args[2], &foreground, &preload) {
                Ok(report) => {
                    println!(
                        "preloaded {} bytes from {} files ({} skipped, {} failed)",
                        report.preloaded_bytes(),
                        report.preloaded_files(),
                        report.discovery_skipped.len(),
                        report.failed_count()
                    );
                }
                Err(err) => {
                    eprintln!("corepolicy preload-package {}: {err}", args[2]);
                    std::process::exit(1);
                }
            }
        }
        _ => {
            eprintln!("usage: corepolicy daemon | preload-package <package>");
            std::process::exit(2);
        }
    }
}
