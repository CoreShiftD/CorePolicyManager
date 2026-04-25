// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/

//! Semantic inotify mapping for preload runtime signals.

use crate::high_level::api::InotifyStatus;
use crate::low_level::inotify::{
    DELETE_SELF_MASK, IGNORED_MASK, InotifyEvent, MODIFY_MASK, MOVE_SELF_MASK, PACKAGE_FILE_MASK,
    QUEUE_OVERFLOW_MASK, UNMOUNT_MASK,
};
use crate::low_level::reactor::Fd;
use crate::low_level::spawn::SysError;
use crate::low_level::sys::ProcStatus;

pub const CGROUP_PROCS_PATH: &str = "/dev/cpuset/top-app/cgroup.procs";
pub const PACKAGES_XML_PATH: &str = "/data/system/packages.xml";
pub const PACKAGES_LIST_PATH: &str = "/data/system/packages.list";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InotifySource {
    Cgroup,
    PackagesXml,
    PackagesList,
    Unknown,
}

impl InotifySource {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Cgroup => "cgroup",
            Self::PackagesXml => "packages_xml",
            Self::PackagesList => "packages_list",
            Self::Unknown => "unknown",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PreloadInotifyEvent {
    ForegroundAccepted {
        old_pid: Option<i32>,
        new_pid: i32,
        uid: u32,
        package: String,
    },
    ForegroundSkipped {
        pid: i32,
        uid: Option<u32>,
        name: Option<String>,
        cmdline: Option<String>,
        reason: &'static str,
    },
    PackagesChanged {
        path: &'static str,
    },
    Exceptional {
        source: InotifySource,
        description: &'static str,
        mask: u32,
    },
}

pub struct PreloadInotify {
    pub fd: Fd,
    pub wd_cgroup: i32,
    pub wd_pkg_xml: i32,
    pub wd_pkg_list: i32,
    last_foreground_pid: Option<i32>,
    last_accepted_package: Option<String>,
    packages_dirty: bool,
    events_seen: u64,
    last_raw_mask: Option<u32>,
    last_source: Option<InotifySource>,
    last_exception: Option<String>,
}

impl PreloadInotify {
    pub fn new(fd: Fd, wd_cgroup: i32, wd_pkg_xml: i32, wd_pkg_list: i32) -> Self {
        Self {
            fd,
            wd_cgroup,
            wd_pkg_xml,
            wd_pkg_list,
            last_foreground_pid: None,
            last_accepted_package: None,
            packages_dirty: false,
            events_seen: 0,
            last_raw_mask: None,
            last_source: None,
            last_exception: None,
        }
    }

    pub fn packages_dirty(&self) -> bool {
        self.packages_dirty
    }

    pub fn status(&self) -> InotifyStatus {
        InotifyStatus {
            fd_active: self.fd.raw() >= 0,
            wd_cgroup: (self.wd_cgroup >= 0).then_some(self.wd_cgroup),
            wd_pkg_xml: (self.wd_pkg_xml >= 0).then_some(self.wd_pkg_xml),
            wd_pkg_list: (self.wd_pkg_list >= 0).then_some(self.wd_pkg_list),
            events_seen: self.events_seen,
            last_raw_mask: self.last_raw_mask,
            last_source: self.last_source.map(|source| source.as_str().to_string()),
            last_foreground_pid: self.last_foreground_pid,
            package_cache_dirty: self.packages_dirty,
            last_exception: self.last_exception.clone(),
        }
    }

    pub fn handle_readable(&mut self) -> Result<Vec<PreloadInotifyEvent>, SysError> {
        let raw_events = crate::low_level::inotify::read_events(&self.fd)?;
        Ok(self.handle_decoded_events_with_procfs(
            &raw_events,
            || crate::low_level::sys::read_to_string(CGROUP_PROCS_PATH),
            crate::low_level::sys::read_proc_status,
            crate::low_level::sys::read_proc_cmdline,
        ))
    }

    pub fn handle_decoded_events<F>(
        &mut self,
        raw_events: &[InotifyEvent],
        read_cgroup: F,
    ) -> Vec<PreloadInotifyEvent>
    where
        F: FnMut() -> Result<String, std::io::Error>,
    {
        self.handle_decoded_events_with_procfs(
            raw_events,
            read_cgroup,
            crate::low_level::sys::read_proc_status,
            crate::low_level::sys::read_proc_cmdline,
        )
    }

    pub fn handle_decoded_events_with_procfs<F, S, C>(
        &mut self,
        raw_events: &[InotifyEvent],
        mut read_cgroup: F,
        mut read_status: S,
        mut read_cmdline: C,
    ) -> Vec<PreloadInotifyEvent>
    where
        F: FnMut() -> Result<String, std::io::Error>,
        S: FnMut(i32) -> Result<ProcStatus, std::io::Error>,
        C: FnMut(i32) -> Result<String, std::io::Error>,
    {
        let mut out = Vec::new();
        let mut cgroup_changed = false;

        for event in raw_events {
            self.events_seen = self.events_seen.saturating_add(1);
            self.last_raw_mask = Some(event.mask);
            let source = self.source_for_wd(event.wd);
            self.last_source = Some(source);

            if event.mask & QUEUE_OVERFLOW_MASK != 0 {
                self.packages_dirty = true;
                self.record_exception("queue_overflow");
                out.push(PreloadInotifyEvent::Exceptional {
                    source,
                    description: "queue_overflow",
                    mask: event.mask,
                });
                continue;
            }

            if event.mask & UNMOUNT_MASK != 0 {
                self.record_exception("unmount");
                out.push(PreloadInotifyEvent::Exceptional {
                    source,
                    description: "unmount",
                    mask: event.mask,
                });
                continue;
            }

            if event.mask & IGNORED_MASK != 0 {
                let description = if self.try_reregister_source(source) {
                    "ignored_reregistered"
                } else {
                    "ignored"
                };
                self.record_exception(description);
                out.push(PreloadInotifyEvent::Exceptional {
                    source,
                    description,
                    mask: event.mask,
                });
                continue;
            }

            if event.mask & (DELETE_SELF_MASK | MOVE_SELF_MASK) != 0 {
                self.record_exception("watch_target_replaced");
                out.push(PreloadInotifyEvent::Exceptional {
                    source,
                    description: "watch_target_replaced",
                    mask: event.mask,
                });
            }

            match source {
                InotifySource::PackagesXml => {
                    self.packages_dirty = true;
                    out.push(PreloadInotifyEvent::PackagesChanged {
                        path: PACKAGES_XML_PATH,
                    });
                }
                InotifySource::PackagesList => {
                    self.packages_dirty = true;
                    out.push(PreloadInotifyEvent::PackagesChanged {
                        path: PACKAGES_LIST_PATH,
                    });
                }
                InotifySource::Cgroup if event.mask & MODIFY_MASK != 0 => {
                    cgroup_changed = true;
                }
                _ => {}
            }
        }

        if cgroup_changed
            && let Ok(content) = read_cgroup()
            && let Some(new_pid) = parse_top_app_pid(&content)
        {
            let old_pid = self.last_foreground_pid;
            if old_pid != Some(new_pid) {
                self.last_foreground_pid = Some(new_pid);
                match classify_foreground_pid(new_pid, &mut read_status, &mut read_cmdline) {
                    ForegroundClassification::Accept { uid, package } => {
                        // Only emit event if the normalized package name actually changed.
                        // This suppresses churn where same app starts new PIDs/threads.
                        if self.last_accepted_package.as_deref() != Some(package.as_str()) {
                            self.last_accepted_package = Some(package.clone());
                            out.push(PreloadInotifyEvent::ForegroundAccepted {
                                old_pid,
                                new_pid,
                                uid,
                                package,
                            });
                        }
                    }
                    ForegroundClassification::Reject {
                        uid,
                        name,
                        cmdline,
                        reason,
                    } => {
                        // Suppress logs for common noisy system processes to keep logs meaningful.
                        let is_noisy_system = name.as_deref().map(is_noisy_system_process).unwrap_or(false);
                        if !is_noisy_system {
                            out.push(PreloadInotifyEvent::ForegroundSkipped {
                                pid: new_pid,
                                uid,
                                name,
                                cmdline,
                                reason,
                            });
                        }
                    }
                    ForegroundClassification::Vanished => {}
                }
            }
        }

        out
    }

    fn source_for_wd(&self, wd: i32) -> InotifySource {
        if wd == self.wd_cgroup {
            InotifySource::Cgroup
        } else if wd == self.wd_pkg_xml {
            InotifySource::PackagesXml
        } else if wd == self.wd_pkg_list {
            InotifySource::PackagesList
        } else {
            InotifySource::Unknown
        }
    }

    fn record_exception(&mut self, description: &'static str) {
        self.last_exception = Some(description.to_string());
    }

    fn try_reregister_source(&mut self, source: InotifySource) -> bool {
        let registered = match source {
            InotifySource::Cgroup => crate::low_level::inotify::add_watch(
                &self.fd,
                CGROUP_PROCS_PATH,
                crate::low_level::inotify::MODIFY_MASK,
            )
            .map(|wd| self.wd_cgroup = wd),
            InotifySource::PackagesXml => {
                crate::low_level::inotify::add_watch(&self.fd, PACKAGES_XML_PATH, PACKAGE_FILE_MASK)
                    .map(|wd| self.wd_pkg_xml = wd)
            }
            InotifySource::PackagesList => crate::low_level::inotify::add_watch(
                &self.fd,
                PACKAGES_LIST_PATH,
                PACKAGE_FILE_MASK,
            )
            .map(|wd| self.wd_pkg_list = wd),
            InotifySource::Unknown => return false,
        };
        registered.is_ok()
    }
}

pub fn parse_top_app_pid(content: &str) -> Option<i32> {
    content
        .split_whitespace()
        .find_map(|pid| pid.parse::<i32>().ok())
        .filter(|pid| *pid > 0)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ForegroundClassification {
    Accept {
        uid: u32,
        package: String,
    },
    Reject {
        uid: Option<u32>,
        name: Option<String>,
        cmdline: Option<String>,
        reason: &'static str,
    },
    Vanished,
}

pub fn classify_foreground_pid<S, C>(
    pid: i32,
    mut read_status: S,
    mut read_cmdline: C,
) -> ForegroundClassification
where
    S: FnMut(i32) -> Result<ProcStatus, std::io::Error>,
    C: FnMut(i32) -> Result<String, std::io::Error>,
{
    let status = match read_status(pid) {
        Ok(status) => status,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            return ForegroundClassification::Vanished;
        }
        Err(_) => {
            return ForegroundClassification::Reject {
                uid: None,
                name: None,
                cmdline: None,
                reason: "status_unreadable",
            };
        }
    };

    if status.uid < 10_000 {
        return ForegroundClassification::Reject {
            uid: Some(status.uid),
            name: Some(status.name),
            cmdline: None,
            reason: "system_uid",
        };
    }

    if core_process_name(&status.name) {
        return ForegroundClassification::Reject {
            uid: Some(status.uid),
            name: Some(status.name),
            cmdline: None,
            reason: "system_process",
        };
    }

    if status.name.starts_with("com.android.") || status.name.starts_with("com.google.android.") {
        return ForegroundClassification::Reject {
            uid: Some(status.uid),
            name: Some(status.name),
            cmdline: None,
            reason: "system_process",
        };
    }

    if !status.name.contains('.') {
        return ForegroundClassification::Reject {
            uid: Some(status.uid),
            name: Some(status.name),
            cmdline: None,
            reason: "no_dot_name",
        };
    }

    let cmdline = match read_cmdline(pid) {
        Ok(cmdline) => cmdline,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            return ForegroundClassification::Vanished;
        }
        Err(_) => {
            return ForegroundClassification::Reject {
                uid: Some(status.uid),
                name: Some(status.name),
                cmdline: None,
                reason: "cmdline_unreadable",
            };
        }
    };

    let package = match normalize_foreground_package(&cmdline) {
        PackageNormalization::Accept(package) => package,
        PackageNormalization::RejectHelperProcess => {
            return ForegroundClassification::Reject {
                uid: Some(status.uid),
                name: Some(status.name),
                cmdline: Some(cmdline),
                reason: "helper_process",
            };
        }
    };

    ForegroundClassification::Accept {
        uid: status.uid,
        package,
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum PackageNormalization {
    Accept(String),
    RejectHelperProcess,
}

fn normalize_foreground_package(cmdline: &str) -> PackageNormalization {
    let (base_pkg, suffix) = match cmdline.split_once(':') {
        Some((base, suffix)) => (base, Some(suffix)),
        None => (cmdline, None),
    };

    if let Some(suffix) = suffix
        && helper_process_suffix(suffix)
    {
        return PackageNormalization::RejectHelperProcess;
    }

    PackageNormalization::Accept(base_pkg.to_string())
}

fn helper_process_suffix(suffix: &str) -> bool {
    [
        "sandboxed_process",
        "renderer",
        "webview",
        "gpu",
        "isolated",
        "privileged_process",
    ]
    .iter()
    .any(|needle| suffix.contains(needle))
}

fn is_noisy_system_process(name: &str) -> bool {
    matches!(
        name,
        "system_server" | "surfaceflinger" | "zygote" | "zygote64" | "init"
    )
}

fn core_process_name(name: &str) -> bool {
    matches!(
        name,
        "system_server"
            | "zygote"
            | "zygote64"
            | "surfaceflinger"
            | "servicemanager"
            | "hwservicemanager"
            | "vndservicemanager"
            | "logd"
            | "netd"
            | "installd"
    )
}
