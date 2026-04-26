use coreshift_policy::api::{daemon, json, resolver};
use std::time::Duration;

#[test]
fn test_json_api() {
    let test_file = "test_api.json";
    #[derive(serde::Serialize, serde::Deserialize, PartialEq, Debug)]
    struct TestData {
        name: String,
        value: i32,
    }
    let data = TestData {
        name: "test".to_string(),
        value: 42,
    };

    json::write(test_file, &data).expect("Failed to write JSON");

    let content = std::fs::read_to_string(test_file).expect("Failed to read file");
    assert!(json::parse::<TestData>(&content).is_ok());

    let loaded: TestData = json::read(test_file).expect("Failed to load JSON");
    assert_eq!(data, loaded);

    let pretty = json::pretty(&data).expect("Failed to serialize");
    assert!(pretty.contains("\"name\": \"test\""));

    std::fs::remove_file(test_file).unwrap();
}

struct TestFeature {
    start_called: bool,
}

impl daemon::DaemonFeature for TestFeature {
    fn name(&self) -> &'static str {
        "test_feature"
    }
    fn on_start(&mut self, _ctx: &daemon::DaemonContext) -> Result<(), daemon::DaemonError> {
        self.start_called = true;
        Ok(())
    }
}

struct TestWorker;
impl daemon::ThreadedFeature for TestWorker {
    fn name(&self) -> &'static str {
        "test_worker"
    }
    fn run(self: Box<Self>, ctx: daemon::FeatureThreadContext) -> Result<(), daemon::DaemonError> {
        while !ctx.shutdown_requested() {
            std::thread::sleep(Duration::from_millis(10));
        }
        Ok(())
    }
}

#[test]
fn test_builder_and_plugin_system() {
    let daemon = daemon::DaemonBuilder::new()
        .with_feature(Box::new(TestFeature {
            start_called: false,
        }))
        .with_threaded_feature(Box::new(TestWorker))
        .build()
        .expect("Failed to build daemon");

    assert!(daemon.status().daemon.alive);
}

#[test]
fn test_resolver_api() {
    let snapshot = resolver::ForegroundSnapshot {
        pid: Some(1234),
        package: Some("com.test.app".to_string()),
        last_skip_reason: None,
    };
    let event = resolver::ForegroundEvent {
        previous: resolver::ForegroundSnapshot::default(),
        current: snapshot,
    };
    assert_eq!(event.current.pid, Some(1234));
}

#[test]
fn test_shutdown_requested_logic() {
    let (tx, rx) = std::sync::mpsc::channel();
    let ctx = daemon::FeatureThreadContext {
        work_dir: std::path::PathBuf::from("/tmp"),
        shutdown_receiver: rx,
        foreground_receiver: std::sync::mpsc::channel().1,
    };

    assert!(!ctx.shutdown_requested());
    tx.send(()).unwrap();
    assert!(ctx.shutdown_requested()); // returns true, consumes message

    // Now it should be false because message was consumed and tx is still alive
    assert!(!ctx.shutdown_requested());

    drop(tx); // Disconnect
    assert!(ctx.shutdown_requested()); // returns true because disconnected
    assert!(ctx.shutdown_requested()); // still true
}
