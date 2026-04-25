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
    fn runtime_log_level_gate_matches_router_filtering() {
        assert!(crate::log_level_enabled(
            crate::core::LogLevel::Info,
            crate::core::LogLevel::Warn
        ));
        assert!(crate::log_level_enabled(
            crate::core::LogLevel::Info,
            crate::core::LogLevel::Info
        ));
        assert!(!crate::log_level_enabled(
            crate::core::LogLevel::Info,
            crate::core::LogLevel::Debug
        ));
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
        assert_eq!(reqs.len(), 1); // Log

        // PID 200 foregrounded at t=50 (overwrites 100)
        let mut state50 = ExecutionState::new();
        state50.clock = 50;
        let reqs = addon.on_core_event(&state50, &Event::ForegroundChanged { pid: 200 });
        assert_eq!(reqs.len(), 1); // Log

        // t=140: No tick yet
        let mut state140 = ExecutionState::new();
        state140.clock = 140;
        let reqs = addon.on_core_event(&state140, &Event::Tick);
        assert!(reqs.is_empty()); // Tick cleanup only, or nothing

        // t=151: Tick triggers resolve for 200
        let mut state151 = ExecutionState::new();
        state151.clock = 151;
        let reqs = addon.on_core_event(&state151, &Event::Tick);
        assert_eq!(reqs.len(), 2); // Log + ResolveIdentity
        if let crate::core::Intent::SystemRequest { kind, .. } = &reqs[1].intent {
            assert_eq!(*kind, crate::core::SystemService::ResolveIdentity);
        } else {
            panic!("Expected SystemRequest at index 1");
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
            assert!(msg.contains("skipped") && msg.contains("negative_cache"));
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
        assert_eq!(reqs.len(), 3);
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
        assert!(addon.package_cache_dirty);
        assert!(addon.dedup_cache.is_empty());
    }

    #[test]
    fn cgroup_content_parser_uses_first_positive_pid() {
        assert_eq!(crate::runtime::parse_top_app_pid("1234\n"), Some(1234));
        assert_eq!(crate::runtime::parse_top_app_pid("  42 99\n"), Some(42));
        assert_eq!(crate::runtime::parse_top_app_pid("bad\n77\n"), Some(77));
        assert_eq!(crate::runtime::parse_top_app_pid("-1\n0\n"), None);
        assert_eq!(crate::runtime::parse_top_app_pid(""), None);
    }

    #[test]
    fn foreground_classifier_rejects_system_uid() {
        let decision = crate::runtime::classify_foreground_pid(
            123,
            |_| {
                Ok(crate::low_level::sys::ProcStatus {
                    name: "com.foo.bar".to_string(),
                    uid: 1000,
                })
            },
            |_| panic!("cmdline should not be read for system uid"),
        );

        assert_eq!(
            decision,
            crate::runtime::ForegroundClassification::Reject {
                uid: Some(1000),
                name: Some("com.foo.bar".to_string()),
                cmdline: None,
                reason: "system_uid",
            }
        );
    }

    #[test]
    fn foreground_classifier_rejects_system_server_name() {
        let decision = crate::runtime::classify_foreground_pid(
            123,
            |_| {
                Ok(crate::low_level::sys::ProcStatus {
                    name: "system_server".to_string(),
                    uid: 10000,
                })
            },
            |_| panic!("cmdline should not be read for no-dot process names"),
        );

        assert!(matches!(
            decision,
            crate::runtime::ForegroundClassification::Reject {
                reason: "system_process",
                ..
            }
        ));
    }

    #[test]
    fn foreground_classifier_rejects_no_dot_name() {
        let decision = crate::runtime::classify_foreground_pid(
            123,
            |_| {
                Ok(crate::low_level::sys::ProcStatus {
                    name: "launcher".to_string(),
                    uid: 10000,
                })
            },
            |_| panic!("cmdline should not be read for no-dot process names"),
        );

        assert!(matches!(
            decision,
            crate::runtime::ForegroundClassification::Reject {
                reason: "no_dot_name",
                ..
            }
        ));
    }

    #[test]
    fn foreground_classifier_rejects_obvious_android_package_names() {
        let decision = crate::runtime::classify_foreground_pid(
            123,
            |_| {
                Ok(crate::low_level::sys::ProcStatus {
                    name: "com.android.settings".to_string(),
                    uid: 10000,
                })
            },
            |_| panic!("cmdline should not be read for obvious system packages"),
        );

        assert!(matches!(
            decision,
            crate::runtime::ForegroundClassification::Reject {
                reason: "system_process",
                ..
            }
        ));
    }

    #[test]
    fn foreground_classifier_accepts_primary_package_process() {
        let decision = crate::runtime::classify_foreground_pid(
            123,
            |_| {
                Ok(crate::low_level::sys::ProcStatus {
                    name: "com.foo.bar".to_string(),
                    uid: 10234,
                })
            },
            |_| Ok("com.foo.bar".to_string()),
        );

        assert_eq!(
            decision,
            crate::runtime::ForegroundClassification::Accept {
                uid: 10234,
                package: "com.foo.bar".to_string(),
            }
        );
    }

    #[test]
    fn foreground_classifier_accepts_telegram_push_as_base_package() {
        let decision = crate::runtime::classify_foreground_pid(
            123,
            |_| {
                Ok(crate::low_level::sys::ProcStatus {
                    name: "org.telegram.messenger".to_string(),
                    uid: 10234,
                })
            },
            |_| Ok("org.telegram.messenger:push".to_string()),
        );

        assert_eq!(
            decision,
            crate::runtime::ForegroundClassification::Accept {
                uid: 10234,
                package: "org.telegram.messenger".to_string(),
            }
        );
    }

    #[test]
    fn foreground_classifier_accepts_service_process_as_base_package() {
        let decision = crate::runtime::classify_foreground_pid(
            123,
            |_| {
                Ok(crate::low_level::sys::ProcStatus {
                    name: "com.foo.bar".to_string(),
                    uid: 10234,
                })
            },
            |_| Ok("com.foo.bar:service".to_string()),
        );

        assert_eq!(
            decision,
            crate::runtime::ForegroundClassification::Accept {
                uid: 10234,
                package: "com.foo.bar".to_string(),
            }
        );
    }

    #[test]
    fn foreground_classifier_rejects_helper_process_cmdline() {
        let decision = crate::runtime::classify_foreground_pid(
            123,
            |_| {
                Ok(crate::low_level::sys::ProcStatus {
                    name: "com.android.chrome".to_string(),
                    uid: 10234,
                })
            },
            |_| Ok("com.android.chrome:sandboxed_process0".to_string()),
        );

        assert!(matches!(
            decision,
            crate::runtime::ForegroundClassification::Reject {
                reason: "system_process",
                ..
            }
        ));
    }

    #[test]
    fn foreground_classifier_rejects_helper_suffix_for_app_owned_process() {
        let decision = crate::runtime::classify_foreground_pid(
            123,
            |_| {
                Ok(crate::low_level::sys::ProcStatus {
                    name: "org.chromium.chrome".to_string(),
                    uid: 10234,
                })
            },
            |_| Ok("org.chromium.chrome:sandboxed_process0".to_string()),
        );

        assert!(matches!(
            decision,
            crate::runtime::ForegroundClassification::Reject {
                reason: "helper_process",
                ..
            }
        ));
    }

    #[test]
    fn foreground_classifier_ignores_vanished_pid() {
        let decision = crate::runtime::classify_foreground_pid(
            123,
            |_| {
                Err(std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    "vanished",
                ))
            },
            |_| Ok("com.foo.bar".to_string()),
        );

        assert_eq!(decision, crate::runtime::ForegroundClassification::Vanished);
    }

    #[test]
    fn inotify_decode_handles_multiple_packed_events_and_empty_name() {
        use crate::low_level::inotify::{InotifyEvent, decode_events};

        fn push_event(buf: &mut Vec<u8>, wd: i32, mask: u32, name: &[u8]) {
            let event = libc::inotify_event {
                wd,
                mask,
                cookie: 0,
                len: name.len() as u32,
            };
            let bytes = unsafe {
                std::slice::from_raw_parts(
                    (&event as *const libc::inotify_event).cast::<u8>(),
                    std::mem::size_of::<libc::inotify_event>(),
                )
            };
            buf.extend_from_slice(bytes);
            buf.extend_from_slice(name);
        }

        let mut buf = Vec::new();
        push_event(&mut buf, 10, crate::low_level::inotify::MODIFY_MASK, &[]);
        push_event(
            &mut buf,
            11,
            crate::low_level::inotify::PACKAGE_FILE_MASK,
            b"child\0\0\0",
        );

        let events = decode_events(&buf);
        assert_eq!(
            events,
            vec![
                InotifyEvent {
                    wd: 10,
                    mask: crate::low_level::inotify::MODIFY_MASK,
                    name_len: 0,
                },
                InotifyEvent {
                    wd: 11,
                    mask: crate::low_level::inotify::PACKAGE_FILE_MASK,
                    name_len: 8,
                },
            ]
        );
    }

    #[test]
    fn inotify_pid_change_detection_emits_only_on_change() {
        use crate::low_level::inotify::InotifyEvent;
        use crate::low_level::reactor::Fd;
        use crate::runtime::{PreloadInotify, PreloadInotifyEvent};
        use std::os::unix::io::IntoRawFd;
        use std::os::unix::net::UnixStream;

        let (fd, _peer) = UnixStream::pair().unwrap();
        let mut watcher =
            PreloadInotify::new(Fd::new(fd.into_raw_fd(), "test").unwrap(), 10, 11, 12);

        let mut status_reads = 0;
        let mut cmdline_reads = 0;

        let first = watcher.handle_decoded_events_with_procfs(
            &[InotifyEvent {
                wd: 10,
                mask: crate::low_level::inotify::MODIFY_MASK,
                name_len: 0,
            }],
            || Ok("100\n".to_string()),
            |pid| {
                status_reads += 1;
                Ok(crate::low_level::sys::ProcStatus {
                    name: format!("com.example.app{}", pid),
                    uid: 10234,
                })
            },
            |pid| {
                cmdline_reads += 1;
                Ok(format!("com.example.app{}", pid))
            },
        );
        assert_eq!(
            first,
            vec![PreloadInotifyEvent::ForegroundAccepted {
                old_pid: None,
                new_pid: 100,
                uid: 10234,
                package: "com.example.app100".to_string(),
            }]
        );
        assert_eq!(status_reads, 1);
        assert_eq!(cmdline_reads, 1);

        let duplicate = watcher.handle_decoded_events_with_procfs(
            &[InotifyEvent {
                wd: 10,
                mask: crate::low_level::inotify::MODIFY_MASK,
                name_len: 0,
            }],
            || Ok("100\n".to_string()),
            |pid| {
                status_reads += 1;
                Ok(crate::low_level::sys::ProcStatus {
                    name: format!("com.example.app{}", pid),
                    uid: 10234,
                })
            },
            |pid| {
                cmdline_reads += 1;
                Ok(format!("com.example.app{}", pid))
            },
        );
        assert!(duplicate.is_empty());
        assert_eq!(status_reads, 1);
        assert_eq!(cmdline_reads, 1);

        let changed = watcher.handle_decoded_events_with_procfs(
            &[InotifyEvent {
                wd: 10,
                mask: crate::low_level::inotify::MODIFY_MASK,
                name_len: 0,
            }],
            || Ok("200\n".to_string()),
            |pid| {
                status_reads += 1;
                Ok(crate::low_level::sys::ProcStatus {
                    name: format!("com.example.app{}", pid),
                    uid: 10234,
                })
            },
            |pid| {
                cmdline_reads += 1;
                Ok(format!("com.example.app{}", pid))
            },
        );
        assert_eq!(
            changed,
            vec![PreloadInotifyEvent::ForegroundAccepted {
                old_pid: Some(100),
                new_pid: 200,
                uid: 10234,
                package: "com.example.app200".to_string(),
            }]
        );
        assert_eq!(status_reads, 2);
        assert_eq!(cmdline_reads, 2);
    }

    #[test]
    fn inotify_suppresses_repeated_normalized_package() {
        use crate::low_level::inotify::InotifyEvent;
        use crate::low_level::reactor::Fd;
        use crate::runtime::{PreloadInotify, PreloadInotifyEvent};
        use std::os::unix::io::IntoRawFd;
        use std::os::unix::net::UnixStream;

        let (fd, _peer) = UnixStream::pair().unwrap();
        let mut watcher =
            PreloadInotify::new(Fd::new(fd.into_raw_fd(), "test").unwrap(), 10, 11, 12);

        let first = watcher.handle_decoded_events_with_procfs(
            &[InotifyEvent {
                wd: 10,
                mask: crate::low_level::inotify::MODIFY_MASK,
                name_len: 0,
            }],
            || Ok("100\n".to_string()),
            |_| {
                Ok(crate::low_level::sys::ProcStatus {
                    name: "com.foo.bar".to_string(),
                    uid: 10234,
                })
            },
            |_| Ok("com.foo.bar:service".to_string()),
        );
        assert_eq!(
            first,
            vec![PreloadInotifyEvent::ForegroundAccepted {
                old_pid: None,
                new_pid: 100,
                uid: 10234,
                package: "com.foo.bar".to_string(),
            }]
        );

        let repeated_package = watcher.handle_decoded_events_with_procfs(
            &[InotifyEvent {
                wd: 10,
                mask: crate::low_level::inotify::MODIFY_MASK,
                name_len: 0,
            }],
            || Ok("200\n".to_string()),
            |_| {
                Ok(crate::low_level::sys::ProcStatus {
                    name: "com.foo.bar".to_string(),
                    uid: 10234,
                })
            },
            |_| Ok("com.foo.bar:push".to_string()),
        );
        assert!(repeated_package.is_empty());
    }

    #[test]
    fn package_change_marks_dirty_without_reading_cgroup() {
        use crate::low_level::inotify::InotifyEvent;
        use crate::low_level::reactor::Fd;
        use crate::runtime::{PreloadInotify, PreloadInotifyEvent};
        use std::os::unix::io::IntoRawFd;
        use std::os::unix::net::UnixStream;

        let (fd, _peer) = UnixStream::pair().unwrap();
        let mut watcher =
            PreloadInotify::new(Fd::new(fd.into_raw_fd(), "test").unwrap(), 10, 11, 12);
        let mut cgroup_reads = 0;

        let events = watcher.handle_decoded_events(
            &[InotifyEvent {
                wd: 11,
                mask: crate::low_level::inotify::MODIFY_MASK,
                name_len: 0,
            }],
            || {
                cgroup_reads += 1;
                Ok("100\n".to_string())
            },
        );

        assert_eq!(
            events,
            vec![PreloadInotifyEvent::PackagesChanged {
                path: crate::runtime::PACKAGES_XML_PATH
            }]
        );
        assert!(watcher.packages_dirty());
        let status = watcher.status();
        assert_eq!(status.events_seen, 1);
        assert_eq!(status.last_source.as_deref(), Some("packages_xml"));
        assert_eq!(cgroup_reads, 0);
    }

    #[test]
    fn inotify_queue_overflow_marks_state_uncertain() {
        use crate::low_level::inotify::InotifyEvent;
        use crate::low_level::reactor::Fd;
        use crate::runtime::{InotifySource, PreloadInotify, PreloadInotifyEvent};
        use std::os::unix::io::IntoRawFd;
        use std::os::unix::net::UnixStream;

        let (fd, _peer) = UnixStream::pair().unwrap();
        let mut watcher =
            PreloadInotify::new(Fd::new(fd.into_raw_fd(), "test").unwrap(), 10, 11, 12);

        let events = watcher.handle_decoded_events(
            &[InotifyEvent {
                wd: -1,
                mask: crate::low_level::inotify::QUEUE_OVERFLOW_MASK,
                name_len: 0,
            }],
            || Ok("100\n".to_string()),
        );

        assert_eq!(
            events,
            vec![PreloadInotifyEvent::Exceptional {
                source: InotifySource::Unknown,
                description: "queue_overflow",
                mask: crate::low_level::inotify::QUEUE_OVERFLOW_MASK,
            }]
        );
        let status = watcher.status();
        assert_eq!(status.events_seen, 1);
        assert_eq!(status.last_source.as_deref(), Some("unknown"));
        assert_eq!(status.last_exception.as_deref(), Some("queue_overflow"));
        assert!(status.package_cache_dirty);
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
    // Preload status layer-boundary tests
    // -------------------------------------------------------------------------

    /// PreloadSnapshot is a pure policy snapshot: no filesystem probes, no
    /// daemon context, no serialization logic inside the addon.
    #[test]
    fn preload_snapshot_is_pure_policy_state() {
        use crate::high_level::addons::preload::{PreloadAddon, PreloadConfig};

        let config = PreloadConfig {
            enabled: false,
            ..Default::default()
        };
        let addon = PreloadAddon::new(config);
        let snap = addon.status_snapshot();

        // Snapshot contains only addon-owned policy fields.
        assert!(!snap.enabled);
        assert_eq!(snap.last_foreground_pid, -1);
        assert!(snap.last_foreground_package.is_none());
        assert_eq!(snap.package_cache_count, 0);
        assert!(!snap.package_cache_dirty);
        assert_eq!(snap.dedup_cache_count, 0);
        assert_eq!(snap.negative_cache_count, 0);
        assert_eq!(snap.in_flight_count, 0);
        assert_eq!(snap.total_failures, 0);
        assert!(!snap.auto_disabled);
        assert!(snap.last_skip_reason.is_none());
        assert!(snap.last_warmup_result.is_none());
        // Snapshot must NOT contain daemon-level fields (socket_path, mode,
        // enable_preload_path, foreground_path_exists) - those live in
        // DaemonStatusReport assembled by the runtime.
    }

    /// PreloadSnapshot serializes and deserializes correctly via api types.
    #[test]
    fn preload_snapshot_serializes_and_deserializes() {
        use crate::high_level::api::PreloadSnapshot;

        let snap = PreloadSnapshot {
            enabled: true,
            last_foreground_pid: 1234,
            last_foreground_package: Some("com.example.app".to_string()),
            last_transition: Some("none -> com.example.app".to_string()),
            package_cache_count: 3,
            package_cache_dirty: true,
            dedup_cache_count: 1,
            negative_cache_count: 0,
            in_flight_count: 0,
            in_flight_packages: Vec::new(),
            total_failures: 2,
            auto_disabled: false,
            events_seen: 100,
            last_skip_stage: Some("identity_resolution".to_string()),
            last_skip_reason: Some("cooldown".to_string()),
            last_skip_package: None,
            last_discovered_path_count: 0,
            last_warmup_result: Some(
                "package=com.example.app bytes=4096 duration_ms=12".to_string(),
            ),
            last_warmup_package: None,
        };

        let json = serde_json::to_string(&snap).expect("serialization must succeed");
        let decoded: PreloadSnapshot =
            serde_json::from_str(&json).expect("deserialization must succeed");

        assert_eq!(snap, decoded);
        assert!(json.contains("\"enabled\":true"));
        assert!(json.contains("\"last_foreground_pid\":1234"));
        assert!(json.contains("com.example.app"));
        assert!(json.contains("cooldown"));
    }

    /// DaemonStatusReport (assembled by runtime) serializes and deserializes.
    #[test]
    fn daemon_status_report_serializes_and_deserializes() {
        use crate::high_level::api::{DaemonStatusReport, PreloadSnapshot, WatchedPathStatus};

        let report = DaemonStatusReport {
            uptime_secs: 10,
            mode: "preload".to_string(),
            socket_path: "/data/local/tmp/coreshift/coreshift.sock".to_string(),
            active_clients: 1,
            preload_addon_loaded: true,
            enable_preload_file_exists: false,
            enable_preload_path: "/data/local/tmp/coreshift/control/enable_preload".to_string(),
            foreground_path_exists: false,
            watched_paths: vec![
                WatchedPathStatus {
                    path: "/dev/cpuset/top-app/cgroup.procs".to_string(),
                    registered: true,
                },
                WatchedPathStatus {
                    path: "/data/system/packages.xml".to_string(),
                    registered: false,
                },
            ],
            inotify: None,
            preload: Some(PreloadSnapshot {
                enabled: true,
                last_foreground_pid: 42,
                last_foreground_package: Some("com.test".to_string()),
                last_transition: Some("none -> com.test".to_string()),
                package_cache_count: 1,
                package_cache_dirty: false,
                dedup_cache_count: 0,
                negative_cache_count: 0,
                in_flight_count: 0,
                in_flight_packages: Vec::new(),
                total_failures: 0,
                auto_disabled: false,
                events_seen: 50,
                last_skip_stage: None,
                last_skip_reason: None,
                last_skip_package: None,
                last_discovered_path_count: 0,
                last_warmup_result: None,
                last_warmup_package: None,
            }),
        };

        let json = serde_json::to_string(&report).expect("serialization must succeed");
        let decoded: DaemonStatusReport =
            serde_json::from_str(&json).expect("deserialization must succeed");

        assert_eq!(report, decoded);
        assert!(json.contains("\"mode\":\"preload\""));
        assert!(json.contains("com.test"));
    }

    /// Runtime assembler produces a DaemonStatusReport without touching addon
    /// internals beyond the snapshot method.
    #[test]
    fn runtime_assembler_produces_report_from_snapshot() {
        use crate::high_level::addon::Addon;
        use crate::high_level::addons::preload::{PreloadAddon, PreloadConfig};
        use crate::high_level::api::WatchedPathStatus;
        use crate::runtime::assemble_daemon_status;

        let config = PreloadConfig {
            enabled: true,
            ..Default::default()
        };
        let addon = PreloadAddon::new(config);
        let watches = vec![WatchedPathStatus {
            path: "/dev/cpuset/top-app/cgroup.procs".to_string(),
            registered: true,
        }];

        let report = assemble_daemon_status(
            0,
            "preload",
            "/data/local/tmp/coreshift/coreshift.sock",
            0,
            Some(&addon as &dyn Addon),
            &watches,
            None,
        );

        assert_eq!(report.mode, "preload");
        assert_eq!(
            report.socket_path,
            "/data/local/tmp/coreshift/coreshift.sock"
        );
        assert!(report.preload_addon_loaded);
        assert_eq!(report.watched_paths.len(), 1);
        assert!(report.watched_paths[0].registered);
        // Filesystem fields are probed by the runtime, not the addon.
        assert_eq!(
            report.enable_preload_path,
            crate::paths::ENABLE_PRELOAD_PATH
        );
        let snap = report.preload.expect("preload snapshot must be present");
        assert!(snap.enabled);
        assert_eq!(snap.last_foreground_pid, -1);
    }

    /// Runtime assembler with no preload addon produces a report with
    /// preload_addon_loaded=false and preload=None.
    #[test]
    fn runtime_assembler_without_preload_addon() {
        use crate::runtime::assemble_daemon_status;

        let report = assemble_daemon_status(0, "normal", "/tmp/test.sock", 0, None, &[], None);

        assert_eq!(report.mode, "normal");
        assert!(!report.preload_addon_loaded);
        assert!(report.preload.is_none());
        assert!(report.inotify.is_none());
        assert!(report.watched_paths.is_empty());
    }

    /// Snapshot tracks skip reason correctly (policy state, no FS).
    #[test]
    fn preload_snapshot_tracks_skip_reason() {
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

        addon.in_flight.insert("com.first".to_string());

        let _ = addon.on_core_event(
            &state,
            &Event::SystemResponse {
                request_id: 0,
                kind: SystemService::ResolveIdentity,
                payload: "com.second".to_string().into_bytes(),
            },
        );

        let snap = addon.status_snapshot();
        assert_eq!(snap.last_skip_reason.as_deref(), Some("global_budget_full"));
        assert_eq!(snap.last_foreground_package.as_deref(), Some("com.second"));
    }

    /// Snapshot tracks warmup result correctly (policy state, no FS).
    #[test]
    fn preload_snapshot_tracks_warmup_result() {
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

        let snap = addon.status_snapshot();
        let result = snap.last_warmup_result.expect("warmup result must be set");
        assert!(result.contains("com.warmup"));
        assert!(result.contains("bytes=8192"));
        assert!(result.contains("duration_ms=25"));
        assert!(!addon.in_flight.contains("com.warmup"));
    }

    /// IPC layer frames the JSON status string correctly (type byte 5).
    /// The IPC module receives an opaque string; it does not parse preload
    /// internals.
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

        // IPC receives an opaque JSON string; it does not know preload types.
        let status_json = r#"{"mode":"preload","preload_addon_loaded":true}"#.to_string();
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

    /// ensure_preload_control_file creates the file when absent and returns
    /// true; returns false without touching the filesystem when already present.
    #[test]
    fn ensure_preload_control_file_creates_when_absent() {
        use crate::ensure_preload_control_file;
        use crate::low_level::sys::path_exists;

        let tmp = std::env::temp_dir().join("coreshift_test_ensure_preload");
        let path = tmp.to_str().expect("valid temp path");

        // Start clean.
        let _ = std::fs::remove_file(path);
        assert!(!path_exists(path), "precondition: file must not exist");

        // First call: file is absent, must be created, returns true.
        let created = ensure_preload_control_file(path);
        assert!(created, "must return true when file was created");
        assert!(path_exists(path), "file must exist after creation");

        // Second call: file already present, must return false without error.
        let created_again = ensure_preload_control_file(path);
        assert!(!created_again, "must return false when file already exists");
        assert!(path_exists(path), "file must still exist");

        let _ = std::fs::remove_file(path);
    }

    /// Watch registrations are owned by the runtime (daemon_watch_registrations),
    /// not by the addon.  The runtime assembler passes them directly into
    /// DaemonStatusReport.watched_paths; the addon snapshot has no watch fields.
    #[test]
    fn runtime_assembler_watch_registrations_come_from_runtime_not_addon() {
        use crate::high_level::addon::Addon;
        use crate::high_level::addons::preload::{PreloadAddon, PreloadConfig};
        use crate::high_level::api::WatchedPathStatus;
        use crate::runtime::assemble_daemon_status;

        let addon = PreloadAddon::new(PreloadConfig::default());
        let regs = vec![
            WatchedPathStatus {
                path: "/dev/cpuset/top-app/cgroup.procs".to_string(),
                registered: true,
            },
            WatchedPathStatus {
                path: "/data/system/packages.xml".to_string(),
                registered: false,
            },
        ];

        // The runtime passes watch registrations directly; the addon snapshot
        // does not carry them.
        let report = assemble_daemon_status(
            0,
            "preload",
            "/tmp/s.sock",
            0,
            Some(&addon as &dyn Addon),
            &regs,
            None,
        );
        assert_eq!(report.watched_paths, regs);
        assert!(report.watched_paths[0].registered);
        assert!(!report.watched_paths[1].registered);

        // Snapshot has no watch_registrations field - confirmed by the type.
        let snap = report.preload.expect("snapshot must be present");
        // PreloadSnapshot fields are all policy state; no watched_paths field.
        assert_eq!(snap.last_foreground_pid, -1);
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
                    if path != *main_rs {
                        for token in forbidden {
                            assert!(
                                !content.contains(token),
                                "Found forbidden output macro '{}' in {:?}",
                                token,
                                path
                            );
                        }
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
