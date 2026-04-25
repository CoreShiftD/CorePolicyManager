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
    ForegroundChanged {
        old_pid: Option<i32>,
        new_pid: i32,
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
        Ok(self.handle_decoded_events(&raw_events, || {
            crate::low_level::sys::read_to_string(CGROUP_PROCS_PATH)
        }))
    }

    pub fn handle_decoded_events<F>(
        &mut self,
        raw_events: &[InotifyEvent],
        mut read_cgroup: F,
    ) -> Vec<PreloadInotifyEvent>
    where
        F: FnMut() -> Result<String, std::io::Error>,
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
                out.push(PreloadInotifyEvent::ForegroundChanged { old_pid, new_pid });
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
