use crate::paths::CPUSET_TOP_APP;
use crate::runtime::logging;
use coreshift_lowlevel::sys::{proc_uid, read_proc_cmdline};
use std::fs;
use std::time::Duration;

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
    last_accepted_pid: Option<i32>,
    last_accepted_package: Option<String>,
    blacklist: Vec<String>,
}

impl ForegroundResolver {
    pub fn new(blacklist: Vec<String>) -> Self {
        logging::info(&format!("ForegroundResolver: watching {}", CPUSET_TOP_APP));
        Self {
            last_accepted_pid: None,
            last_accepted_package: None,
            blacklist,
        }
    }

    pub fn resolve_current_foreground(&mut self) -> Option<ForegroundSnapshot> {
        if let Ok(content) = fs::read_to_string(CPUSET_TOP_APP) {
            return self.scan_pids(&content);
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

            match self.classify_pid(pid) {
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

    fn classify_pid(&self, pid: i32) -> CandidateResult {
        let uid = match proc_uid(pid) {
            Ok(u) => u,
            Err(_) => return CandidateResult::SkipProcStatFailed,
        };

        if uid < 10000 {
            return CandidateResult::SkipSystemUid;
        }

        match read_proc_cmdline(pid) {
            Ok(cmdline) if !cmdline.is_empty() => {
                let candidate = self.normalize_package(&cmdline);

                if !self.is_package_like(&candidate) {
                    CandidateResult::SkipMalformed
                } else if self.is_blacklisted(&candidate) {
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

    fn normalize_package(&self, cmdline: &str) -> String {
        let first_token = cmdline.split_whitespace().next().unwrap_or("");
        if let Some(pos) = first_token.find(':') {
            first_token[..pos].to_string()
        } else {
            first_token.to_string()
        }
    }

    fn is_package_like(&self, name: &str) -> bool {
        !name.is_empty()
            && name.contains('.')
            && !name.contains(char::is_whitespace)
            && name.len() > 3
            && name.len() < 128
    }

    fn is_blacklisted(&self, name: &str) -> bool {
        self.blacklist.iter().any(|b| name.starts_with(b))
    }
}

#[cfg(test)]
mod tests {

    #[test]
    fn test_package_normalization_logic() {
        assert_eq!(normalize_package("com.foo.bar"), "com.foo.bar");
        assert_eq!(normalize_package("com.foo.bar:service"), "com.foo.bar");
    }

    fn normalize_package(cmdline: &str) -> String {
        let first_token = cmdline.split_whitespace().next().unwrap_or("");
        if let Some(pos) = first_token.find(':') {
            first_token[..pos].to_string()
        } else {
            first_token.to_string()
        }
    }
}
