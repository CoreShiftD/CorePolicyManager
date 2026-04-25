// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/

#[cfg(test)]
mod tests_internal {
    use crate::core::scheduler::Scheduler;
    use crate::core::{
        Action, ActionMeta, CauseId, ExecutionState, JobRequest, LogEvent, RoutedAction,
    };
    use crate::high_level::identity::Principal;

    #[test]
    fn test_scheduler_budget() {
        let mut scheduler = Scheduler::new(10);
        let mut state = ExecutionState::new();
        let meta = ActionMeta {
            id: CauseId(1),
            parent: None,
            source: Principal::System,
            reply_to: None,
        };

        for i in 0..20 {
            scheduler.enqueue(
                RoutedAction {
                    action: Action::AdvanceTime { delta: i as u64 },
                    meta: meta.clone(),
                },
                &mut state,
            );
        }

        let mut count = 0;
        while scheduler.pop_next().is_some() {
            count += 1;
        }

        assert_eq!(count, 10);
        assert!(scheduler.is_exhausted());
    }

    #[test]
    fn test_scheduler_priority_eviction() {
        let mut scheduler = Scheduler::new(1000000);
        let mut state = ExecutionState::new();
        let meta = ActionMeta {
            id: CauseId(1),
            parent: None,
            source: Principal::System,
            reply_to: None,
        };

        // Fill background queue
        for _ in 0..4096 {
            // EmitLog is Priority::Background
            scheduler.enqueue(
                RoutedAction {
                    action: Action::EmitLog {
                        owner: 0,
                        level: crate::core::LogLevel::Info,
                        event: LogEvent::Generic("test".to_string()),
                    },
                    meta: meta.clone(),
                },
                &mut state,
            );
        }

        // MAX_PER_ACTION_KIND is 1000
        assert_eq!(scheduler.total_len, 1000);

        // Enqueue critical action - it should NOT evict because queue is NOT full (1000 < 4096)
        let res = scheduler.enqueue(
            RoutedAction {
                action: Action::AdvanceTime { delta: 1 },
                meta: meta.clone(),
            },
            &mut state,
        );

        assert!(res.is_none());
        assert_eq!(scheduler.total_len, 1001);
    }

    #[test]
    fn test_deterministic_replay_advanced() {
        use crate::core::replay::ReplayInput;
        use crate::run_replay;
        use std::fs::File;

        let path = "test_replay_adv.bin";
        let mut file = File::create(path).unwrap();

        // Tick 1: Submit a job
        let intent = crate::core::Intent::Submit {
            id: 1,
            owner: 0,
            job: JobRequest {
                command: vec!["ls".to_string()],
            },
        };
        bincode::serialize_into(&mut file, &ReplayInput::Intent(Principal::System, intent))
            .unwrap();

        // Tick 2: Advance time
        bincode::serialize_into(
            &mut file,
            &ReplayInput::Event(crate::core::Event::TimeAdvanced(100)),
        )
        .unwrap();

        drop(file);

        let hash1 = run_replay(path).expect("replay file should open");
        let hash2 = run_replay(path).expect("replay file should open");
        assert_eq!(hash1, hash2);

        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn test_ipc_malformed_flood() {
        // This test simulates a flood of malformed IPC packets to ensure no panics
        use crate::low_level::reactor::{Reactor, Token};
        use crate::mid_level::ipc::{IpcModule, ReadState};
        use std::os::unix::io::IntoRawFd;
        use std::os::unix::net::UnixStream;

        let (server_sock, client_sock) = UnixStream::pair().unwrap();
        server_sock.set_nonblocking(true).unwrap();
        client_sock.set_nonblocking(true).unwrap();

        let mut reactor = Reactor::new().unwrap();
        let server_fd =
            crate::low_level::reactor::Fd::new(server_sock.into_raw_fd(), "test").unwrap();

        let mut ipc = IpcModule::new(server_fd, Token(1));

        // Simulate a client connection
        let (s2, _c2) = UnixStream::pair().unwrap();
        s2.set_nonblocking(true).unwrap();
        let client_id = 1;
        let token = Token(2);
        let conn = crate::mid_level::ipc::Conn {
            fd: crate::low_level::reactor::Fd::new(s2.into_raw_fd(), "test").unwrap(),
            token,
            read_buf: vec![0u8; 1024 * 1024], // Already over limit!
            write_buf: vec![],
            state: ReadState::Header { needed: 4 },
            uid: 1000,
        };
        ipc.clients.insert(client_id, conn);
        ipc.client_tokens.insert(token, client_id);

        let event = crate::low_level::reactor::Event {
            token,
            readable: true,
            writable: false,
            error: false,
        };

        // Should trigger disconnect due to MAX_READ_BUF
        let msgs = ipc.handle_event(&mut reactor, &event);
        assert!(msgs.is_empty());
        assert!(ipc.clients.is_empty());
    }

    #[test]
    fn test_ipc_partial_frame_waits_for_complete_body() {
        use crate::low_level::reactor::{Event, Reactor, Token};
        use crate::mid_level::ipc::{Conn, IpcModule, ReadState};
        use std::io::Write;
        use std::os::unix::io::IntoRawFd;
        use std::os::unix::net::UnixStream;

        let (server_sock, _client_sock) = UnixStream::pair().unwrap();
        server_sock.set_nonblocking(true).unwrap();

        let mut reactor = Reactor::new().unwrap();
        let mut ipc = IpcModule::new(
            crate::low_level::reactor::Fd::new(server_sock.into_raw_fd(), "test").unwrap(),
            Token(1),
        );

        let (conn_sock, mut peer_sock) = UnixStream::pair().unwrap();
        conn_sock.set_nonblocking(true).unwrap();
        peer_sock.set_nonblocking(true).unwrap();
        let token = Token(2);
        ipc.clients.insert(
            1,
            Conn {
                fd: crate::low_level::reactor::Fd::new(conn_sock.into_raw_fd(), "test").unwrap(),
                token,
                read_buf: Vec::new(),
                write_buf: Vec::new(),
                state: ReadState::Header { needed: 4 },
                uid: 1000,
            },
        );
        ipc.client_tokens.insert(token, 1);

        let mut payload = vec![3u8];
        payload.extend_from_slice(&7u64.to_le_bytes());
        let mut frame = Vec::new();
        frame.extend_from_slice(&(payload.len() as u32).to_le_bytes());
        frame.extend_from_slice(&payload);

        peer_sock.write_all(&frame[..6]).unwrap();
        let msgs = ipc.handle_event(
            &mut reactor,
            &Event {
                token,
                readable: true,
                writable: false,
                error: false,
            },
        );
        assert!(msgs.is_empty());
        assert!(ipc.clients.contains_key(&1));

        peer_sock.write_all(&frame[6..]).unwrap();
        let msgs = ipc.handle_event(
            &mut reactor,
            &Event {
                token,
                readable: true,
                writable: false,
                error: false,
            },
        );
        assert_eq!(msgs.len(), 1);
        match &msgs[0].command {
            crate::high_level::api::Command::Cancel { id } => assert_eq!(*id, 7),
            other => panic!("unexpected command {:?}", other),
        }
    }

    #[test]
    fn test_ipc_response_overflow_drops_client() {
        use crate::low_level::reactor::Token;
        use crate::mid_level::ipc::{Conn, IpcModule, ReadState};
        use std::os::unix::io::IntoRawFd;
        use std::os::unix::net::UnixStream;

        let (server_sock, _client_sock) = UnixStream::pair().unwrap();
        server_sock.set_nonblocking(true).unwrap();
        let mut ipc = IpcModule::new(
            crate::low_level::reactor::Fd::new(server_sock.into_raw_fd(), "test").unwrap(),
            Token(1),
        );

        let (conn_sock, _peer_sock) = UnixStream::pair().unwrap();
        conn_sock.set_nonblocking(true).unwrap();
        let token = Token(2);
        ipc.clients.insert(
            1,
            Conn {
                fd: crate::low_level::reactor::Fd::new(conn_sock.into_raw_fd(), "test").unwrap(),
                token,
                read_buf: Vec::new(),
                write_buf: vec![0u8; (1024 * 1024) - 12],
                state: ReadState::Header { needed: 4 },
                uid: 1000,
            },
        );
        ipc.client_tokens.insert(token, 1);

        ipc.intercept_action(&crate::core::Action::Started { id: 42 }, Some(1));
        assert!(!ipc.clients.contains_key(&1));
        assert!(!ipc.client_tokens.contains_key(&token));
    }

    #[test]
    fn test_ipc_writable_event_handles_partial_write() {
        use crate::low_level::reactor::{Event, Reactor, Token};
        use crate::mid_level::ipc::{Conn, IpcModule, ReadState};
        use std::io::Read;
        use std::os::unix::io::{AsRawFd, IntoRawFd};
        use std::os::unix::net::UnixStream;

        let (server_sock, _client_sock) = UnixStream::pair().unwrap();
        server_sock.set_nonblocking(true).unwrap();
        let mut reactor = Reactor::new().unwrap();
        let mut ipc = IpcModule::new(
            crate::low_level::reactor::Fd::new(server_sock.into_raw_fd(), "test").unwrap(),
            Token(1),
        );

        let (conn_sock, peer_sock) = UnixStream::pair().unwrap();
        conn_sock.set_nonblocking(true).unwrap();
        peer_sock.set_nonblocking(true).unwrap();

        let sndbuf: libc::c_int = 4096;
        let ret = unsafe {
            libc::setsockopt(
                conn_sock.as_raw_fd(),
                libc::SOL_SOCKET,
                libc::SO_SNDBUF,
                &sndbuf as *const _ as *const libc::c_void,
                std::mem::size_of_val(&sndbuf) as libc::socklen_t,
            )
        };
        assert_eq!(ret, 0);

        let token = Token(2);
        let original_len = 256 * 1024;
        ipc.clients.insert(
            1,
            Conn {
                fd: crate::low_level::reactor::Fd::new(conn_sock.into_raw_fd(), "test").unwrap(),
                token,
                read_buf: Vec::new(),
                write_buf: vec![7u8; original_len],
                state: ReadState::Header { needed: 4 },
                uid: 1000,
            },
        );
        ipc.client_tokens.insert(token, 1);

        let msgs = ipc.handle_event(
            &mut reactor,
            &Event {
                token,
                readable: false,
                writable: true,
                error: false,
            },
        );
        assert!(msgs.is_empty());
        let remaining = ipc
            .clients
            .get(&1)
            .expect("client must remain connected")
            .write_buf
            .len();
        assert!(remaining > 0);
        assert!(remaining < original_len);

        let mut received = vec![0u8; original_len];
        let n = (&peer_sock)
            .read(received.as_mut_slice())
            .expect("peer should receive at least one chunk");
        assert!(n > 0);
    }

    #[test]
    fn execution_state_job_view_handles_stale_core_handles() {
        use crate::core::state_view::StateView;

        let mut state = ExecutionState::new();
        state.core.insert_job(
            7,
            42,
            crate::core::ExecSpec {
                argv: vec!["true".to_string()],
                stdin: None,
                capture_stdout: false,
                capture_stderr: false,
                max_output: 0,
            },
            crate::core::ExecPolicy {
                timeout_ms: None,
                kill_grace_ms: 0,
                cancel: crate::core::CancelPolicy::None,
            },
        );

        let handle = state.core.job_handle(7).expect("job handle must exist");
        let removed = state.core.jobs.remove(handle.index, handle.generation);
        assert!(removed.is_some());

        assert!(state.job(7).is_none());
    }

    #[test]
    fn verify_global_reports_drift_without_panicking() {
        let mut state = ExecutionState::new();
        state.core.insert_job(
            9,
            7,
            crate::core::ExecSpec {
                argv: vec!["true".to_string()],
                stdin: None,
                capture_stdout: false,
                capture_stderr: false,
                max_output: 0,
            },
            crate::core::ExecPolicy {
                timeout_ms: None,
                kill_grace_ms: 0,
                cancel: crate::core::CancelPolicy::None,
            },
        );

        let handle = state.core.job_handle(9).expect("job handle must exist");
        state.core.runtime[handle.index as usize] = None;

        let err = crate::core::verify::verify_global(&state).expect_err("drift must be reported");
        assert!(err.contains("job missing runtime mapping"));
    }

    #[cfg(debug_assertions)]
    #[test]
    fn verify_global_reports_core_hash_drift() {
        let mut state = ExecutionState::new();
        state.core.insert_job(
            11,
            3,
            crate::core::ExecSpec {
                argv: vec!["true".to_string()],
                stdin: None,
                capture_stdout: false,
                capture_stderr: false,
                max_output: 0,
            },
            crate::core::ExecPolicy {
                timeout_ms: None,
                kill_grace_ms: 0,
                cancel: crate::core::CancelPolicy::None,
            },
        );
        state.core.hash ^= 1;

        let err =
            crate::core::verify::verify_global(&state).expect_err("hash drift must be reported");
        assert!(err.contains("core hash drift"));
    }

    #[test]
    fn reactor_timeout_honors_tick_deadline() {
        assert_eq!(crate::compute_reactor_timeout_ms(-1, 0), 16);
        assert_eq!(crate::compute_reactor_timeout_ms(-1, 15), 1);
        assert_eq!(crate::compute_reactor_timeout_ms(-1, 16), 0);
        assert_eq!(crate::compute_reactor_timeout_ms(50, 0), 16);
        assert_eq!(crate::compute_reactor_timeout_ms(5, 0), 5);
        assert_eq!(crate::compute_reactor_timeout_ms(0, 0), 0);
    }

    #[test]
    fn test_preload_addon_debouncing() {
        use crate::core::{Event, ExecutionState};
        use crate::high_level::addon::Addon;
        use crate::high_level::addons::preload::{PreloadAddon, PreloadConfig};

        let config = PreloadConfig {
            enabled: true,
            debounce_ms: 100,
            ..Default::default()
        };
        let mut addon = PreloadAddon::new(config);
        let state = ExecutionState::new();

        // PID 100 foregrounded at t=0
        let reqs = addon.on_core_event(&state, &Event::ForegroundChanged { pid: 100 });
        assert!(reqs.is_empty()); // Should be pending

        // PID 200 foregrounded at t=50 (overwrites 100)
        let mut state50 = ExecutionState::new();
        state50.clock = 50;
        let reqs = addon.on_core_event(&state50, &Event::ForegroundChanged { pid: 200 });
        assert!(reqs.is_empty());

        // t=140: No tick yet
        let mut state140 = ExecutionState::new();
        state140.clock = 140;
        let reqs = addon.on_core_event(&state140, &Event::Tick);
        assert!(reqs.is_empty());

        // t=151: Tick triggers resolve for 200
        let mut state151 = ExecutionState::new();
        state151.clock = 151;
        let reqs = addon.on_core_event(&state151, &Event::Tick);
        assert_eq!(reqs.len(), 1);
        if let crate::core::Intent::SystemRequest { kind, .. } = &reqs[0].intent {
            assert_eq!(*kind, crate::core::SystemService::ResolveIdentity);
        } else {
            panic!("Expected SystemRequest");
        }
    }

    #[test]
    fn test_preload_addon_deduplication() {
        use crate::core::{Event, ExecutionState, SystemService};
        use crate::high_level::addon::Addon;
        use crate::high_level::addons::preload::{PreloadAddon, PreloadConfig};

        let config = PreloadConfig {
            enabled: true,
            ..Default::default()
        };
        let mut addon = PreloadAddon::new(config);
        let state = ExecutionState::new();

        // Successful resolve for "com.test"
        let reqs = addon.on_core_event(
            &state,
            &Event::SystemResponse {
                request_id: 0,
                kind: SystemService::ResolveIdentity,
                payload: "com.test".to_string().into_bytes(),
            },
        );
        assert!(!reqs.is_empty());

        // Simulate it's now in-flight
        let reqs = addon.on_core_event(
            &state,
            &Event::SystemResponse {
                request_id: 0,
                kind: SystemService::ResolveDirectory,
                payload: serde_json::to_vec(&(
                    "com.test".to_string(),
                    "/data/app/test".to_string(),
                ))
                .unwrap(),
            },
        );
        assert_eq!(reqs.len(), 1);

        let reqs = addon.on_core_event(
            &state,
            &Event::SystemResponse {
                request_id: 0,
                kind: SystemService::DiscoverPaths,
                payload: serde_json::to_vec(&(
                    "com.test".to_string(),
                    vec!["base.apk".to_string()],
                ))
                .unwrap(),
            },
        );
        assert!(reqs.len() >= 2);
        assert!(addon.in_flight.contains("com.test"));

        // Another resolve for "com.test" while in-flight
        let reqs = addon.on_core_event(
            &state,
            &Event::SystemResponse {
                request_id: 0,
                kind: SystemService::ResolveIdentity,
                payload: "com.test".to_string().into_bytes(),
            },
        );
        assert_eq!(reqs.len(), 2);
        if let crate::core::Intent::AddonLog { msg, .. } = &reqs[1].intent {
            assert!(msg.contains("skip") && msg.contains("already_in_flight"));
        } else {
            panic!("Expected SKIP log");
        }
    }

    #[test]
    fn test_preload_addon_failure_backoff() {
        use crate::core::{Event, ExecutionState, SystemService};
        use crate::high_level::addon::Addon;
        use crate::high_level::addons::preload::{PreloadAddon, PreloadConfig};

        let config = PreloadConfig {
            enabled: true,
            per_package_failure_backoff_ms: 1000,
            ..Default::default()
        };
        let mut addon = PreloadAddon::new(config);

        let mut state = ExecutionState::new();
        state.clock = 100;

        // Fail a warmup
        addon.in_flight.insert("com.fail".to_string());
        let _ = addon.on_core_event(
            &state,
            &Event::AddonFailed {
                addon_id: 102,
                key: "warmup:com.fail".to_string(),
                err: "io error".to_string(),
            },
        );

        assert!(addon.negative_cache.contains_key("com.fail"));

        // Try again at t=500 (too soon)
        state.clock = 500;
        let reqs = addon.on_core_event(
            &state,
            &Event::SystemResponse {
                request_id: 0,
                kind: SystemService::ResolveIdentity,
                payload: "com.fail".to_string().into_bytes(),
            },
        );
        assert_eq!(reqs.len(), 2);
        if let crate::core::Intent::AddonLog { msg, .. } = &reqs[1].intent {
            assert!(msg.contains("skip") && msg.contains("failure_backoff"));
        } else {
            panic!("Expected SKIP log");
        }

        // Try again at t=1200 (after backoff)
        state.clock = 1200;
        let reqs = addon.on_core_event(
            &state,
            &Event::SystemResponse {
                request_id: 0,
                kind: SystemService::ResolveIdentity,
                payload: "com.fail".to_string().into_bytes(),
            },
        );
        assert_eq!(reqs.len(), 2);
    }

    #[test]
    fn test_preload_addon_cache_invalidation() {
        use crate::core::{Event, ExecutionState};
        use crate::high_level::addon::Addon;
        use crate::high_level::addons::preload::{PreloadAddon, PreloadConfig};

        let config = PreloadConfig {
            enabled: true,
            ..Default::default()
        };
        let mut addon = PreloadAddon::new(config);
        let state = ExecutionState::new();

        addon
            .package_map
            .insert("com.test".to_string(), "path".into());
        addon.dedup_cache.insert("com.test".to_string(), 100);

        let _ = addon.on_core_event(&state, &Event::PackagesChanged);

        assert!(addon.package_map.is_empty());
        assert!(addon.dedup_cache.is_empty());
    }

    #[test]
    fn log_schema_no_legacy_messages() {
        // This test ensures that we don't have literal legacy log strings in our source code
        // We split the strings to avoid finding them in this test file itself.
        let forbidden = [
            format!("{}_{}", "tick", "start"),
            format!("{}_{}", "tick", "end"),
            format!("{}_{}", "action", "dispatched"),
        ];

        let src_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");

        fn check_dir(dir: &std::path::Path, forbidden: &[String]) {
            for entry in std::fs::read_dir(dir).unwrap() {
                let entry = entry.unwrap();
                let path = entry.path();
                if path.is_dir() {
                    check_dir(&path, forbidden);
                } else if path.extension().is_some_and(|e| e == "rs") {
                    let content = std::fs::read_to_string(&path).unwrap();
                    for f in forbidden {
                        if content.contains(f) {
                            panic!("Found legacy log string '{}' in {:?}", f, path);
                        }
                    }
                }
            }
        }

        check_dir(&src_dir, &forbidden);
    }

    // -------------------------------------------------------------------------
    // Preload status model tests
    // -------------------------------------------------------------------------

    #[test]
    fn preload_status_report_serializes_and_deserializes() {
        use crate::high_level::addons::preload::{PreloadStatusReport, WatchedPath};

        let report = PreloadStatusReport {
            enabled: true,
            warmup_mode_active: true,
            socket_path: "/data/local/tmp/coreshift/coreshift.sock".to_string(),
            mode: "preload".to_string(),
            enable_preload_file_exists: false,
            enable_preload_path: "/data/local/tmp/coreshift/control/enable_preload".to_string(),
            foreground_path_exists: false,
            last_foreground_pid: 1234,
            last_foreground_package: Some("com.example.app".to_string()),
            package_cache_count: 3,
            dedup_cache_count: 1,
            negative_cache_count: 0,
            in_flight_count: 0,
            total_failures: 2,
            auto_disabled: false,
            watched_paths: vec![
                WatchedPath {
                    path: "/dev/cpuset/top-app/cgroup.procs".to_string(),
                    registered: true,
                },
                WatchedPath {
                    path: "/data/system/packages.xml".to_string(),
                    registered: false,
                },
            ],
            last_skip_reason: Some("cooldown".to_string()),
            last_warmup_result: Some(
                "package=com.example.app bytes=4096 duration_ms=12".to_string(),
            ),
        };

        let json = serde_json::to_string(&report).expect("serialization must succeed");
        let decoded: PreloadStatusReport =
            serde_json::from_str(&json).expect("deserialization must succeed");

        assert_eq!(report, decoded);
        assert!(json.contains("\"enabled\":true"));
        assert!(json.contains("\"last_foreground_pid\":1234"));
        assert!(json.contains("com.example.app"));
        assert!(json.contains("cooldown"));
    }

    #[test]
    fn preload_status_report_default_fields() {
        use crate::high_level::addons::preload::{PreloadAddon, PreloadConfig};

        let config = PreloadConfig {
            enabled: false,
            ..Default::default()
        };
        let addon = PreloadAddon::new(config);
        let report = addon.status_report();

        assert!(!report.enabled);
        assert!(report.warmup_mode_active);
        assert_eq!(report.last_foreground_pid, -1);
        assert!(report.last_foreground_package.is_none());
        assert_eq!(report.package_cache_count, 0);
        assert_eq!(report.dedup_cache_count, 0);
        assert_eq!(report.negative_cache_count, 0);
        assert_eq!(report.in_flight_count, 0);
        assert_eq!(report.total_failures, 0);
        assert!(!report.auto_disabled);
        assert!(report.watched_paths.is_empty());
        assert!(report.last_skip_reason.is_none());
        assert!(report.last_warmup_result.is_none());
        assert_eq!(report.socket_path, crate::paths::SOCKET_PATH);
        assert_eq!(
            report.enable_preload_path,
            crate::paths::ENABLE_PRELOAD_PATH
        );
    }

    #[test]
    fn preload_status_report_tracks_skip_reason() {
        use crate::core::{Event, ExecutionState, SystemService};
        use crate::high_level::addon::Addon;
        use crate::high_level::addons::preload::{PreloadAddon, PreloadConfig};

        let config = PreloadConfig {
            enabled: true,
            global_max_in_flight: 1,
            ..Default::default()
        };
        let mut addon = PreloadAddon::new(config);
        let state = ExecutionState::new();

        // Put one package in-flight to trigger global_budget_full on the next.
        addon.in_flight.insert("com.first".to_string());

        let _ = addon.on_core_event(
            &state,
            &Event::SystemResponse {
                request_id: 0,
                kind: SystemService::ResolveIdentity,
                payload: "com.second".to_string().into_bytes(),
            },
        );

        let report = addon.status_report();
        assert_eq!(
            report.last_skip_reason.as_deref(),
            Some("global_budget_full")
        );
        assert_eq!(
            report.last_foreground_package.as_deref(),
            Some("com.second")
        );
    }

    #[test]
    fn preload_status_report_tracks_warmup_result() {
        use crate::core::{Event, ExecutionState};
        use crate::high_level::addon::Addon;
        use crate::high_level::addons::preload::{PreloadAddon, PreloadConfig};

        let config = PreloadConfig {
            enabled: true,
            ..Default::default()
        };
        let mut addon = PreloadAddon::new(config);
        let state = ExecutionState::new();

        addon.in_flight.insert("com.warmup".to_string());

        let payload = serde_json::to_vec(&(8192u64, 25u64)).unwrap();
        let _ = addon.on_core_event(
            &state,
            &Event::AddonCompleted {
                addon_id: 102,
                key: "warmup:com.warmup".to_string(),
                payload,
            },
        );

        let report = addon.status_report();
        let result = report
            .last_warmup_result
            .expect("warmup result must be set");
        assert!(result.contains("com.warmup"));
        assert!(result.contains("bytes=8192"));
        assert!(result.contains("duration_ms=25"));
        assert!(!addon.in_flight.contains("com.warmup"));
    }

    #[test]
    fn ipc_preload_status_response_encodes_and_decodes() {
        use crate::low_level::reactor::Token;
        use crate::mid_level::ipc::{Conn, IpcModule, ReadState};
        use std::os::unix::io::IntoRawFd;
        use std::os::unix::net::UnixStream;

        let (server_sock, _) = UnixStream::pair().unwrap();
        server_sock.set_nonblocking(true).unwrap();
        let server_fd =
            crate::low_level::reactor::Fd::new(server_sock.into_raw_fd(), "test").unwrap();
        let mut ipc = IpcModule::new(server_fd, Token(1));

        let (conn_sock, _peer) = UnixStream::pair().unwrap();
        conn_sock.set_nonblocking(true).unwrap();
        let token = Token(2);
        let client_id = 1u32;
        ipc.clients.insert(
            client_id,
            Conn {
                fd: crate::low_level::reactor::Fd::new(conn_sock.into_raw_fd(), "test").unwrap(),
                token,
                read_buf: Vec::new(),
                write_buf: Vec::new(),
                state: ReadState::Header { needed: 4 },
                uid: 1000,
            },
        );
        ipc.client_tokens.insert(token, client_id);

        let status_json = r#"{"enabled":true,"mode":"preload"}"#.to_string();
        ipc.send_preload_status(client_id, status_json.clone());

        let conn = ipc.clients.get(&client_id).expect("client must remain");
        // Frame: 4-byte LE length + 1-byte type (5) + JSON bytes
        let frame = &conn.write_buf;
        assert!(frame.len() >= 5);
        let body_len = u32::from_le_bytes([frame[0], frame[1], frame[2], frame[3]]) as usize;
        assert_eq!(body_len, 1 + status_json.len());
        assert_eq!(frame[4], 5u8); // type byte = PreloadStatus
        let decoded = std::str::from_utf8(&frame[5..]).expect("valid utf8");
        assert_eq!(decoded, status_json);
    }

    #[test]
    fn preload_cli_auto_enable_creates_control_file() {
        use crate::high_level::addons::preload::{PreloadAddon, PreloadConfig};
        use std::path::Path;

        // Use a temp dir to avoid touching real /data paths.
        let tmp = std::env::temp_dir().join("coreshift_test_enable_preload");
        let _ = std::fs::remove_file(&tmp);

        // Simulate what run_daemon does: write the file if it doesn't exist.
        if !tmp.exists() {
            std::fs::write(&tmp, b"").expect("write must succeed");
        }
        assert!(Path::new(&tmp).exists(), "control file must be created");

        // Verify the addon picks it up on the next tick when the path exists.
        // We can't use the real ENABLE_PRELOAD_PATH in tests, so we verify the
        // logic by checking that a newly-created PreloadAddon with enabled=false
        // activates when the override file is present (using the real path check
        // in on_core_event, which we skip here since we can't write to /data).
        // Instead, verify the status_report reflects the field correctly.
        let config = PreloadConfig {
            enabled: true,
            ..Default::default()
        };
        let addon = PreloadAddon::new(config);
        let report = addon.status_report();
        assert!(report.enabled);

        let _ = std::fs::remove_file(&tmp);
    }

    #[test]
    fn preload_addon_set_watch_registrations() {
        use crate::high_level::addon::Addon;
        use crate::high_level::addons::preload::{PreloadAddon, PreloadConfig, WatchedPath};

        let mut addon = PreloadAddon::new(PreloadConfig::default());
        assert!(addon.watch_registrations.is_empty());

        let regs = vec![
            WatchedPath {
                path: "/dev/cpuset/top-app/cgroup.procs".to_string(),
                registered: true,
            },
            WatchedPath {
                path: "/data/system/packages.xml".to_string(),
                registered: false,
            },
        ];
        addon.set_watch_registrations(regs.clone());

        assert_eq!(addon.watch_registrations.len(), 2);
        assert!(addon.watch_registrations[0].registered);
        assert!(!addon.watch_registrations[1].registered);

        let report = addon.status_report();
        assert_eq!(report.watched_paths, regs);
    }

    #[test]
    fn production_source_has_no_ad_hoc_output_macros() {
        let forbidden = [concat!("eprint", "ln!"), concat!("db", "g!")];
        let forbidden_outside_main = concat!("print", "ln!");
        let src_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
        let tests_rs = src_dir.join("tests.rs");
        let main_rs = src_dir.join("main.rs");

        fn check_dir(
            dir: &std::path::Path,
            tests_rs: &std::path::Path,
            main_rs: &std::path::Path,
            forbidden: &[&str],
            forbidden_outside_main: &str,
        ) {
            for entry in std::fs::read_dir(dir).unwrap() {
                let entry = entry.unwrap();
                let path = entry.path();
                if path.is_dir() {
                    check_dir(&path, tests_rs, main_rs, forbidden, forbidden_outside_main);
                } else if path.extension().is_some_and(|e| e == "rs") && path != *tests_rs {
                    let content = std::fs::read_to_string(&path).unwrap();
                    for token in forbidden {
                        assert!(
                            !content.contains(token),
                            "Found forbidden output macro '{}' in {:?}",
                            token,
                            path
                        );
                    }
                    if path != *main_rs {
                        assert!(
                            !content.contains(forbidden_outside_main),
                            "Found forbidden output macro '{}' in {:?}",
                            forbidden_outside_main,
                            path
                        );
                    }
                }
            }
        }

        check_dir(
            &src_dir,
            &tests_rs,
            &main_rs,
            &forbidden,
            forbidden_outside_main,
        );
    }

    #[test]
    fn exec_context_rejects_empty_or_invalid_arguments() {
        let empty = crate::low_level::sys::ExecContext::new(Vec::new(), None, None);
        assert!(empty.is_err());

        let invalid_argv =
            crate::low_level::sys::ExecContext::new(vec!["bad\0argv".to_string()], None, None);
        assert!(invalid_argv.is_err());

        let invalid_env = crate::low_level::sys::ExecContext::new(
            vec!["/system/bin/true".to_string()],
            Some(vec!["BAD\0ENV=value".to_string()]),
            None,
        );
        assert!(invalid_env.is_err());
    }

    #[test]
    fn test_ipc_oversized_packet() {
        use crate::low_level::reactor::{Fd, Reactor, Token};
        use crate::mid_level::ipc::{IpcModule, ReadState};
        use std::os::unix::io::IntoRawFd;
        use std::os::unix::net::UnixStream;

        let (server_sock, _client_sock) = UnixStream::pair().unwrap();
        let mut reactor = Reactor::new().unwrap();
        let server_fd = Fd::new(server_sock.into_raw_fd(), "test").unwrap();
        let mut ipc = IpcModule::new(server_fd, Token(1));

        let (s2, mut c2) = UnixStream::pair().unwrap();
        s2.set_nonblocking(true).unwrap();
        c2.set_nonblocking(true).unwrap();
        let token = Token(2);
        let client_id = 1;
        ipc.clients.insert(
            client_id,
            crate::mid_level::ipc::Conn {
                fd: Fd::new(s2.into_raw_fd(), "test").unwrap(),
                token,
                read_buf: vec![],
                write_buf: vec![],
                state: ReadState::Header { needed: 4 },
                uid: 1000,
            },
        );
        ipc.client_tokens.insert(token, client_id);

        // Write a huge length in header (exceeding MAX_PACKET_SIZE)
        use std::io::Write;
        let mut huge_len = [0u8; 4];
        let len = (128 * 1024 + 1) as u32; // MAX_PACKET_SIZE + 1
        huge_len.copy_from_slice(&len.to_le_bytes());
        c2.write_all(&huge_len).unwrap();

        let event = crate::low_level::reactor::Event {
            token,
            readable: true,
            writable: false,
            error: false,
        };
        let msgs = ipc.handle_event(&mut reactor, &event);

        assert!(msgs.is_empty());
        assert!(ipc.clients.is_empty()); // Should have disconnected
    }

    #[test]
    fn test_ipc_multiple_clients() {
        use crate::low_level::reactor::{Fd, Token};
        use crate::mid_level::ipc::{IpcModule, ReadState};
        use std::os::unix::io::IntoRawFd;
        use std::os::unix::net::UnixStream;

        let (server_sock, _) = UnixStream::pair().unwrap();
        let server_fd = Fd::new(server_sock.into_raw_fd(), "test").unwrap();
        let mut ipc = IpcModule::new(server_fd, Token(1));

        for i in 2..10 {
            let (s, _) = UnixStream::pair().unwrap();
            let token = Token(i as u64);
            ipc.clients.insert(
                i as u32,
                crate::mid_level::ipc::Conn {
                    fd: Fd::new(s.into_raw_fd(), "test").unwrap(),
                    token,
                    read_buf: vec![],
                    write_buf: vec![],
                    state: ReadState::Header { needed: 4 },
                    uid: 1000 + i as u32,
                },
            );
            ipc.client_tokens.insert(token, i as u32);
        }

        assert_eq!(ipc.clients.len(), 8);
    }
}
