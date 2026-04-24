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

        let hash1 = run_replay(path);
        let hash2 = run_replay(path);
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
}
