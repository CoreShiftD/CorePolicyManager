use coreshift_policy::api::{
    DaemonBuilder, DaemonContext, DaemonError, DaemonFeature, FeatureThreadContext,
    ThreadedFeature, json, resolver,
};
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use std::time::{Duration, Instant};

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

struct InlineFeature;

impl DaemonFeature for InlineFeature {
    fn name(&self) -> &'static str {
        "inline_feature"
    }

    fn on_start(&mut self, _ctx: &DaemonContext) -> Result<(), DaemonError> {
        Ok(())
    }
}

struct TestWorker {
    started: Arc<AtomicBool>,
}

impl ThreadedFeature for TestWorker {
    fn name(&self) -> &'static str {
        "test_worker"
    }

    fn run(self: Box<Self>, _ctx: FeatureThreadContext) -> Result<(), DaemonError> {
        self.started.store(true, Ordering::SeqCst);
        Ok(())
    }
}

#[test]
fn test_builder_registers_inline_feature() {
    let daemon = DaemonBuilder::new()
        .with_feature(Box::new(InlineFeature))
        .build()
        .expect("Failed to build daemon");

    assert!(daemon.status().daemon.alive);
}

#[test]
fn test_builder_registers_threaded_feature() {
    let started = Arc::new(AtomicBool::new(false));

    let daemon = DaemonBuilder::new()
        .with_threaded_feature(Box::new(TestWorker {
            started: Arc::clone(&started),
        }))
        .build()
        .expect("Failed to build daemon");

    let deadline = Instant::now() + Duration::from_millis(250);
    while Instant::now() < deadline && !started.load(Ordering::SeqCst) {
        std::thread::sleep(Duration::from_millis(10));
    }

    assert!(daemon.status().daemon.alive);
    assert!(started.load(Ordering::SeqCst));
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
    let ctx = FeatureThreadContext {
        work_dir: std::path::PathBuf::from("/tmp"),
        shutdown_receiver: rx,
        foreground_receiver: std::sync::mpsc::channel().1,
    };

    assert!(!ctx.shutdown_requested());

    tx.send(()).unwrap();
    assert!(ctx.shutdown_requested());
    assert!(!ctx.shutdown_requested());

    drop(tx);
    assert!(ctx.shutdown_requested());
    assert!(ctx.shutdown_requested());
}
