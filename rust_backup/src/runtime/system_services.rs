// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/

use crate::core::{Event, SystemService};

pub(super) fn handle_system_request(
    request_id: u64,
    kind: SystemService,
    payload: Vec<u8>,
) -> Vec<Event> {
    match kind {
        SystemService::ResolveIdentity => {
            if let Ok(pid) = serde_json::from_slice::<i32>(&payload) {
                let cmdline_path = format!("/proc/{}/cmdline", pid);
                match std::fs::read(&cmdline_path) {
                    Ok(cmdline) => {
                        if let Some(null_pos) = cmdline.iter().position(|&c| c == 0) {
                            let package_name = &cmdline[..null_pos];
                            vec![Event::SystemResponse {
                                request_id,
                                kind,
                                payload: package_name.to_vec(),
                            }]
                        } else {
                            vec![Event::SystemFailure {
                                request_id,
                                kind,
                                err: "no null terminator".to_string(),
                            }]
                        }
                    }
                    Err(e) => vec![Event::SystemFailure {
                        request_id,
                        kind,
                        err: format!("read failed: {}", e),
                    }],
                }
            } else {
                vec![Event::SystemFailure {
                    request_id,
                    kind,
                    err: "invalid pid payload".to_string(),
                }]
            }
        }
        SystemService::ResolveDirectory => {
            if let Ok(package_name) = String::from_utf8(payload) {
                match std::fs::read_dir("/data/app") {
                    Ok(data_app) => {
                        let mut found = false;
                        let mut res = vec![];
                        for outer_entry in data_app.flatten() {
                            let outer_path = outer_entry.path();
                            if outer_path.is_dir()
                                && let Ok(inner_dir) = std::fs::read_dir(&outer_path)
                            {
                                for inner_entry in inner_dir.flatten() {
                                    let inner_name = inner_entry.file_name();
                                    if inner_name.to_string_lossy().starts_with(&package_name) {
                                        let base_dir =
                                            inner_entry.path().to_string_lossy().into_owned();
                                        let resp_payload =
                                            serde_json::to_vec(&(package_name.clone(), base_dir))
                                                .unwrap_or_default();
                                        res = vec![Event::SystemResponse {
                                            request_id,
                                            kind,
                                            payload: resp_payload,
                                        }];
                                        found = true;
                                        break;
                                    }
                                }
                            }
                            if found {
                                break;
                            }
                        }
                        if !found {
                            vec![Event::SystemFailure {
                                request_id,
                                kind,
                                err: "package dir not found".to_string(),
                            }]
                        } else {
                            res
                        }
                    }
                    Err(e) => vec![Event::SystemFailure {
                        request_id,
                        kind,
                        err: format!("read_dir /data/app failed: {}", e),
                    }],
                }
            } else {
                vec![Event::SystemFailure {
                    request_id,
                    kind,
                    err: "invalid package name payload".to_string(),
                }]
            }
        }
        SystemService::DiscoverPaths => {
            if let Ok((package_name, base_dir)) =
                serde_json::from_slice::<(String, String)>(&payload)
            {
                let mut paths = Vec::new();
                let base_path = std::path::PathBuf::from(&base_dir);

                let lib_dir = base_path.join("lib/arm64");
                if let Ok(entries) = std::fs::read_dir(&lib_dir) {
                    for entry in entries.flatten() {
                        if let Some(ext) = entry.path().extension()
                            && ext == "so"
                        {
                            paths.push(entry.path().to_string_lossy().into_owned());
                        }
                    }
                }

                let oat_dir = base_path.join("oat/arm64");
                if let Ok(entries) = std::fs::read_dir(&oat_dir) {
                    for entry in entries.flatten() {
                        if let Some(ext) = entry.path().extension()
                            && (ext == "odex" || ext == "vdex" || ext == "art")
                        {
                            paths.push(entry.path().to_string_lossy().into_owned());
                        }
                    }
                }

                paths.push(base_path.join("base.apk").to_string_lossy().into_owned());

                if let Ok(entries) = std::fs::read_dir(&base_path) {
                    for entry in entries.flatten() {
                        let name = entry.file_name();
                        let name_str = name.to_string_lossy();
                        if name_str.starts_with("split_") && name_str.ends_with(".apk") {
                            paths.push(entry.path().to_string_lossy().into_owned());
                        }
                    }
                }

                if !paths.is_empty() {
                    paths.sort_unstable();
                    paths.truncate(64);
                    let resp_payload =
                        serde_json::to_vec(&(package_name, paths)).unwrap_or_default();
                    vec![Event::SystemResponse {
                        request_id,
                        kind,
                        payload: resp_payload,
                    }]
                } else {
                    vec![Event::SystemFailure {
                        request_id,
                        kind,
                        err: "no paths discovered".to_string(),
                    }]
                }
            } else {
                vec![Event::SystemFailure {
                    request_id,
                    kind,
                    err: "invalid discovery payload".to_string(),
                }]
            }
        }
    }
}
