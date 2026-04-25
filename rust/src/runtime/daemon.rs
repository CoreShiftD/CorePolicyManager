use crate::features::preload::PreloadFeature;
use crate::paths::CPUSET_TOP_APP;
use crate::runtime::foreground::ForegroundResolver;
use crate::runtime::logging;
use crate::runtime::scheduler::Scheduler;
use crate::runtime::signals::SHUTDOWN;
use crate::runtime::status::DaemonStatus;
use coreshift_lowlevel::inotify::{MODIFY_MASK, add_watch};
use coreshift_lowlevel::reactor::{Event, Reactor, Token};
use std::path::Path;
use std::sync::atomic::Ordering;
use std::time::{Duration, Instant};

pub struct Daemon {
    reactor: Reactor,
    scheduler: Scheduler,
    status: DaemonStatus,
    last_written_status: DaemonStatus,
    last_status_write: Instant,
    start_time: Instant,
    foreground: Option<ForegroundResolver>,
    preload: Option<PreloadFeature>,
    event_buffer: Vec<Event>,
    inotify_token: Option<Token>,
}

impl Daemon {
    pub fn new(preload_only: bool) -> Self {
        let mut reactor = Reactor::new().expect("Failed to create reactor");
        let scheduler = Scheduler::new(Duration::from_secs(1));
        let status = DaemonStatus {
            daemon_alive: true,
            mode: if preload_only {
                "preload".to_string()
            } else {
                "default".to_string()
            },
            enabled_features: vec!["preload".to_string()],
            ..Default::default()
        };

        // Startup Diagnostics
        if !Path::new(CPUSET_TOP_APP).exists() {
            logging::warn(&format!("{} unavailable. Degrading.", CPUSET_TOP_APP));
        }

        let mut inotify_token = None;
        let foreground = match reactor.setup_inotify() {
            Ok(fd) => {
                inotify_token = reactor.inotify_token;
                if let Err(e) = add_watch(&fd, CPUSET_TOP_APP, MODIFY_MASK) {
                    logging::error(&format!("Failed to add watch on {}: {}", CPUSET_TOP_APP, e));
                    None
                } else {
                    Some(ForegroundResolver::new(fd))
                }
            }
            Err(e) => {
                logging::error(&format!("Failed to setup inotify: {}", e));
                None
            }
        };

        let preload = if foreground.is_some() {
            Some(PreloadFeature::new())
        } else {
            None
        };

        let mut d = Self {
            reactor,
            scheduler,
            last_written_status: status.clone(),
            status,
            last_status_write: Instant::now(),
            start_time: Instant::now(),
            foreground,
            preload,
            event_buffer: Vec::with_capacity(16),
            inotify_token,
        };

        if !Path::new(CPUSET_TOP_APP).exists() {
            d.status
                .warnings
                .push(format!("{} unavailable", CPUSET_TOP_APP));
        }
        if d.foreground.is_none() {
            d.status
                .warnings
                .push("Foreground tracking disabled (setup failure)".to_string());
        }
        if d.preload.is_none() {
            d.status
                .warnings
                .push("Preload feature disabled (setup failure)".to_string());
        }

        d
    }

    fn write_status_if_needed(&mut self, force: bool) {
        let now = Instant::now();
        let changed = self.status != self.last_written_status;
        let heartbeat = now.duration_since(self.last_status_write) >= Duration::from_secs(10);

        if force || changed || heartbeat {
            if let Err(e) = self.status.write() {
                logging::error(&format!("Status write failed: {}", e));
            } else {
                self.last_written_status = self.status.clone();
                self.last_status_write = now;
            }
        }
    }

    pub fn run(&mut self) {
        logging::info("CoreShift Policy Daemon started.");
        self.write_status_if_needed(true);

        while !SHUTDOWN.load(Ordering::SeqCst) {
            let timeout_ms = self.scheduler.timeout_until_next_tick_ms();

            // 1. Poll reactor
            self.event_buffer.clear();
            match self.reactor.wait(&mut self.event_buffer, 16, timeout_ms) {
                Ok(_) => {
                    // 2. Drain and dispatch OS events
                    let mut inotify_ready = false;
                    for event in &self.event_buffer {
                        if Some(event.token) == self.inotify_token {
                            inotify_ready = true;
                        }
                    }

                    if inotify_ready
                        && let Some(foreground) = &mut self.foreground
                        && let Some(snapshot) = foreground.handle_event()
                    {
                        self.status.apply_foreground_snapshot(&snapshot);

                        if let Some(pkg) = snapshot.package.as_deref()
                            && let Some(preload) = &mut self.preload
                        {
                            preload.on_foreground_package(pkg, &mut self.status);
                        }
                    }
                }
                Err(e) => {
                    if std::io::Error::last_os_error().kind() != std::io::ErrorKind::Interrupted {
                        logging::error(&format!("Reactor wait error: {}", e));
                    }
                }
            }

            // 3. Run due feature ticks
            if self.scheduler.should_tick() {
                self.scheduler.tick();
                self.status.ticks = self.scheduler.ticks;
                self.status.uptime_secs = self.start_time.elapsed().as_secs();
                self.write_status_if_needed(false);
            }
        }

        logging::info("Shutdown requested. Cleaning up...");
        self.status.daemon_alive = false;
        self.write_status_if_needed(true);
        logging::info("CoreShift Policy Daemon stopped.");
    }
}
