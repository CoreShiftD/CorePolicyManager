// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/

//! Semantic inotify mapping for preload runtime signals.

use crate::low_level::inotify::InotifyEvent;
use crate::low_level::reactor::Fd;
use crate::low_level::spawn::SysError;

pub const CGROUP_PROCS_PATH: &str = "/dev/cpuset/top-app/cgroup.procs";
pub const PACKAGES_XML_PATH: &str = "/data/system/packages.xml";
pub const PACKAGES_LIST_PATH: &str = "/data/system/packages.list";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PreloadInotifyEvent {
    ForegroundChanged { old_pid: Option<i32>, new_pid: i32 },
    PackagesChanged { path: &'static str },
}

pub struct PreloadInotify {
    pub fd: Fd,
    pub wd_cgroup: i32,
    pub wd_pkg_xml: i32,
    pub wd_pkg_list: i32,
    last_foreground_pid: Option<i32>,
    packages_dirty: bool,
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
        }
    }

    pub fn packages_dirty(&self) -> bool {
        self.packages_dirty
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
            if event.wd == self.wd_pkg_xml {
                self.packages_dirty = true;
                out.push(PreloadInotifyEvent::PackagesChanged {
                    path: PACKAGES_XML_PATH,
                });
            } else if event.wd == self.wd_pkg_list {
                self.packages_dirty = true;
                out.push(PreloadInotifyEvent::PackagesChanged {
                    path: PACKAGES_LIST_PATH,
                });
            } else if event.wd == self.wd_cgroup {
                cgroup_changed = true;
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
}

pub fn parse_top_app_pid(content: &str) -> Option<i32> {
    content
        .split_whitespace()
        .find_map(|pid| pid.parse::<i32>().ok())
        .filter(|pid| *pid > 0)
}
