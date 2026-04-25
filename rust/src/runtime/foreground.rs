use crate::paths::CPUSET_TOP_APP;
use crate::runtime::logging;
use coreshift_lowlevel::inotify::read_events;
use coreshift_lowlevel::reactor::Fd;
use coreshift_lowlevel::sys::{proc_uid, read_proc_cmdline};
use std::fs;
use std::time::Duration;

const BLACKLIST_PREFIXES: &[&str] = &[
    "com.google.android.googlequicksearchbox",
    "com.google.android.gms",
    "com.android.systemui",
    "com.android.launcher3",
    "com.android.inputmethod.latin",
];

enum CandidateResult {
    Accept { pid: i32, package: String },
    SkipSystemUid,
    SkipBlacklisted,
    SkipCmdlineFailed,
    SkipMalformed,
    SkipProcStatFailed,
}

#[derive(Default)]
struct ScanSummary {
    total: usize,
    system_uid: usize,
    blacklisted: usize,
    cmdline_failed: usize,
    malformed: usize,
    proc_stat_failed: usize,
}

pub struct ForegroundSnapshot {
    pub pid: Option<i32>,
    pub package: Option<String>,
    pub last_skip_reason: Option<String>,
}

pub struct ForegroundResolver {
    inotify_fd: Fd,
    last_accepted_pid: Option<i32>,
    last_accepted_package: Option<String>,
}

impl ForegroundResolver {
    pub fn new(inotify_fd: Fd) -> Self {
        logging::info(&format!("ForegroundResolver: watching {}", CPUSET_TOP_APP));
        Self {
            inotify_fd,
            last_accepted_pid: None,
            last_accepted_package: None,
        }
    }

    pub fn handle_event(&mut self) -> Option<ForegroundSnapshot> {
        if let Ok(events) = read_events(&self.inotify_fd) {
            if events.is_empty() {
                return None;
            }

            if let Ok(content) = fs::read_to_string(CPUSET_TOP_APP) {
                return self.scan_pids(&content);
            }
        }
        None
    }

    fn scan_pids(&mut self, content: &str) -> Option<ForegroundSnapshot> {
        let mut summary = ScanSummary::default();

        for token in content.split_whitespace() {
            let pid = match token.parse::<i32>() {
                Ok(p) => p,
                Err(_) => continue,
            };

            summary.total += 1;

            match classify_pid(pid) {
                CandidateResult::Accept { pid, package } => {
                    return self.accept(pid, &package);
                }
                CandidateResult::SkipSystemUid => summary.system_uid += 1,
                CandidateResult::SkipBlacklisted => summary.blacklisted += 1,
                CandidateResult::SkipCmdlineFailed => summary.cmdline_failed += 1,
                CandidateResult::SkipMalformed => summary.malformed += 1,
                CandidateResult::SkipProcStatFailed => summary.proc_stat_failed += 1,
            }
        }

        if summary.total > 0 {
            return self.skip_all(summary);
        }

        None
    }

    fn accept(&mut self, pid: i32, pkg: &str) -> Option<ForegroundSnapshot> {
        if Some(pid) == self.last_accepted_pid && self.last_accepted_package.as_deref() == Some(pkg)
        {
            return None;
        }

        logging::debug(&format!(
            "ForegroundResolver: foreground pid={} package={}",
            pid, pkg
        ));

        self.last_accepted_pid = Some(pid);
        self.last_accepted_package = Some(pkg.to_string());

        Some(ForegroundSnapshot {
            pid: Some(pid),
            package: Some(pkg.to_string()),
            last_skip_reason: None,
        })
    }

    fn skip_all(&mut self, s: ScanSummary) -> Option<ForegroundSnapshot> {
        let skip_reason = "no_app_candidate".to_string();
        self.last_accepted_pid = None;
        self.last_accepted_package = None;

        let msg = format!(
            "ForegroundResolver: skip no_app_candidate total={} system_uid={} blacklisted={} malformed={} cmdline_failed={} proc_stat_failed={}",
            s.total, s.system_uid, s.blacklisted, s.malformed, s.cmdline_failed, s.proc_stat_failed
        );
        logging::dedup_debug("no_app_candidate", &msg, Duration::from_secs(60));

        Some(ForegroundSnapshot {
            pid: None,
            package: None,
            last_skip_reason: Some(skip_reason),
        })
    }
}

fn classify_pid(pid: i32) -> CandidateResult {
    let uid = match proc_uid(pid) {
        Ok(u) => u,
        Err(_) => return CandidateResult::SkipProcStatFailed,
    };

    if uid < 10000 {
        return CandidateResult::SkipSystemUid;
    }

    match read_proc_cmdline(pid) {
        Ok(cmdline) if !cmdline.is_empty() => {
            let candidate = normalize_package(&cmdline);

            if !is_package_like(&candidate) {
                CandidateResult::SkipMalformed
            } else if is_blacklisted(&candidate) {
                CandidateResult::SkipBlacklisted
            } else {
                CandidateResult::Accept {
                    pid,
                    package: candidate,
                }
            }
        }
        _ => CandidateResult::SkipCmdlineFailed,
    }
}

fn normalize_package(cmdline: &str) -> String {
    let first_token = cmdline.split_whitespace().next().unwrap_or("");
    if let Some(pos) = first_token.find(':') {
        first_token[..pos].to_string()
    } else {
        first_token.to_string()
    }
}

fn is_package_like(name: &str) -> bool {
    !name.is_empty()
        && name.contains('.')
        && !name.contains(char::is_whitespace)
        && name.len() > 3
        && name.len() < 128
}

fn is_blacklisted(name: &str) -> bool {
    BLACKLIST_PREFIXES
        .iter()
        .any(|&prefix| name.starts_with(prefix))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_package_normalization() {
        assert_eq!(normalize_package("com.foo.bar"), "com.foo.bar");
        assert_eq!(normalize_package("com.foo.bar:service"), "com.foo.bar");
        assert_eq!(
            normalize_package("org.telegram.messenger:push "),
            "org.telegram.messenger"
        );

        assert!(is_package_like("com.foo.bar"));
        assert!(!is_package_like("com"));
        assert!(!is_package_like("system server"));
        assert!(!is_package_like("malformed:suffix"));
    }

    #[test]
    fn test_blacklist() {
        assert!(is_blacklisted("com.google.android.googlequicksearchbox"));
        assert!(is_blacklisted("com.android.systemui"));
        assert!(is_blacklisted("com.android.launcher3"));
        assert!(is_blacklisted("com.android.launcher3:extra"));
        assert!(is_blacklisted("com.android.inputmethod.latin"));

        assert!(!is_blacklisted("com.foo.bar"));
        assert!(!is_blacklisted("org.telegram.messenger"));
    }
}
