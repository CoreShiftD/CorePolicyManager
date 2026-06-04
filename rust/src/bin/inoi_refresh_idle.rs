use std::env;
use std::fs::{File, OpenOptions};
use std::io::{self, Read};
use std::mem::{size_of, MaybeUninit};
use std::os::fd::{AsRawFd, RawFd};
use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::Duration;

const PROP_DYNAMIC: &str = "persist.inoi.refresh.dynamic";
const PROP_LOW: &str = "persist.inoi.refresh.low";
const PROP_HIGH: &str = "persist.inoi.refresh.high";

const DEFAULT_IDLE_SECONDS: u64 = 5;
const DEFAULT_LOW_RATE: &str = "60";
const DEFAULT_HIGH_RATE: &str = "120";
const DEFAULT_INPUT_DEVICES: &str = "/dev/input/event2";

const EV_SYN: u16 = 0x00;
const SYN_REPORT: u16 = 0x00;

static RUNNING: AtomicBool = AtomicBool::new(true);

#[repr(C)]
#[derive(Clone, Copy)]
struct InputEvent {
    tv_sec: libc::time_t,
    tv_usec: libc::suseconds_t,
    type_: u16,
    code: u16,
    value: i32,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Mode {
    Unknown,
    Low,
    High,
}

#[derive(Clone, PartialEq, Eq)]
struct PropState {
    dynamic: String,
    low: String,
    high: String,
}

struct Config {
    idle_ms: i32,
    default_low: String,
    default_high: String,
    input_devices: Vec<String>,
}

struct InputDev {
    path: String,
    file: File,
}

struct Daemon {
    config: Config,
    low_rate: String,
    high_rate: String,
    mode: Mode,
}

fn log_msg(message: &str) {
    coreshift_core::alog_info!("inoi_refresh_idle", "{}", message);
}

unsafe extern "C" fn signal_handler(_: libc::c_int) {
    RUNNING.store(false, Ordering::SeqCst);
}

fn install_signal_handlers() {
    unsafe {
        let handler = signal_handler as *const () as libc::sighandler_t;

        libc::signal(libc::SIGTERM, handler);
        libc::signal(libc::SIGINT, handler);
        libc::signal(libc::SIGHUP, handler);
    }
}

fn prop_get(name: &str) -> String {
    coreshift_core::android_property::android_property_get(name).unwrap_or_default()
}

fn prop_set(name: &str, value: &str) -> bool {
    coreshift_core::android_property::android_property_set(name, value).is_ok()
}

fn prop_state() -> PropState {
    PropState {
        dynamic: prop_get(PROP_DYNAMIC),
        low: prop_get(PROP_LOW),
        high: prop_get(PROP_HIGH),
    }
}

fn dynamic_enabled() -> bool {
    matches!(
        prop_get(PROP_DYNAMIC).as_str(),
        "1" | "true" | "TRUE" | "yes" | "YES" | "on" | "ON"
    )
}

fn valid_rate(value: &str) -> bool {
    !value.is_empty() && value.chars().all(|c| c.is_ascii_digit() || c == '.')
}

fn rate_int(value: &str) -> Option<i64> {
    value.split('.').next()?.parse::<i64>().ok()
}

fn rate_gt(a: &str, b: &str) -> bool {
    match (rate_int(a), rate_int(b)) {
        (Some(a), Some(b)) => a > b,
        _ => false,
    }
}

fn same_rate(a: &str, b: &str) -> bool {
    match (rate_int(a), rate_int(b)) {
        (Some(a), Some(b)) => a == b,
        _ => false,
    }
}

fn read_prop_rate(name: &str) -> Option<String> {
    let value = prop_get(name);

    if valid_rate(&value) {
        Some(value)
    } else {
        None
    }
}

fn command_output(program: &str, args: &[&str]) -> Option<String> {
    let output = Command::new(program).args(args).output().ok()?;

    if !output.status.success() {
        return None;
    }

    Some(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn read_rate_setting(key: &str) -> String {
    let value = command_output("/system/bin/cmd", &["settings", "get", "system", key])
        .or_else(|| command_output("/system/bin/cmd", &["settings", "get", "system", key]))
        .unwrap_or_default();

    match value.as_str() {
        "" | "null" => "0".to_string(),
        _ => value,
    }
}

fn put_rate_setting(key: &str, value: &str) -> bool {
    let current = read_rate_setting(key);

    if same_rate(&current, value) {
        return true;
    }

    Command::new("/system/bin/cmd")
        .args(["settings", "put", "system", key, value])
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
        || Command::new("/system/bin/settings")
            .args(["put", "system", key, value])
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
}

fn parse_input_devices(value: &str) -> Vec<String> {
    value
        .split(|c: char| c == ',' || c.is_ascii_whitespace())
        .filter(|s| !s.is_empty())
        .map(ToString::to_string)
        .collect()
}

fn read_env_rate(name: &str) -> Option<String> {
    let value = env::var(name).ok()?;

    if valid_rate(&value) {
        Some(value)
    } else {
        None
    }
}

fn load_config() -> Config {
    let idle_seconds = env::var("INOI_IDLE_SECONDS")
        .ok()
        .and_then(|v| v.parse::<u64>().ok())
        .filter(|v| *v > 0)
        .unwrap_or(DEFAULT_IDLE_SECONDS);

    let default_low = env::var("INOI_DEFAULT_LOW_RATE")
        .ok()
        .filter(|v| valid_rate(v))
        .unwrap_or_else(|| DEFAULT_LOW_RATE.to_string());

    let default_high = env::var("INOI_DEFAULT_HIGH_RATE")
        .ok()
        .filter(|v| valid_rate(v))
        .unwrap_or_else(|| DEFAULT_HIGH_RATE.to_string());

    let input_devices = env::var("INOI_INPUT_DEVICES")
        .ok()
        .filter(|v| !v.trim().is_empty())
        .unwrap_or_else(|| DEFAULT_INPUT_DEVICES.to_string());

    Config {
        idle_ms: idle_seconds.saturating_mul(1000).min(i32::MAX as u64) as i32,
        default_low,
        default_high,
        input_devices: parse_input_devices(&input_devices),
    }
}

fn monotonic_ms() -> i64 {
    let mut ts = libc::timespec {
        tv_sec: 0,
        tv_nsec: 0,
    };

    unsafe {
        libc::clock_gettime(libc::CLOCK_MONOTONIC, &mut ts);
    }

    ts.tv_sec as i64 * 1000 + ts.tv_nsec as i64 / 1_000_000
}

fn poll_timeout_from_deadline(deadline: Option<i64>) -> i32 {
    let Some(deadline) = deadline else {
        return -1;
    };

    let left = deadline - monotonic_ms();

    if left <= 0 {
        0
    } else if left > i32::MAX as i64 {
        i32::MAX
    } else {
        left as i32
    }
}

impl Daemon {
    fn new(config: Config) -> Self {
        Self {
            config,
            low_rate: String::new(),
            high_rate: String::new(),
            mode: Mode::Unknown,
        }
    }

    fn load_rates(&mut self) -> bool {
        if let (Some(low), Some(high)) =
            (read_env_rate("INOI_LOW_RATE"), read_env_rate("INOI_HIGH_RATE"))
        {
            if rate_gt(&high, &low) {
                self.low_rate = low;
                self.high_rate = high;
                return true;
            }
        }

        let prop_low = read_prop_rate(PROP_LOW);
        let prop_high = read_prop_rate(PROP_HIGH);

        if prop_low.is_some() || prop_high.is_some() {
            let low = prop_low.unwrap_or_else(|| self.config.default_low.clone());
            let high = prop_high.unwrap_or_else(|| self.config.default_high.clone());

            if rate_gt(&high, &low) {
                self.low_rate = low;
                self.high_rate = high;
                return true;
            }
        }

        let current_min = read_rate_setting("min_refresh_rate");
        let current_peak = read_rate_setting("peak_refresh_rate");

        if rate_gt(&current_peak, &current_min) {
            self.low_rate = current_min;
            self.high_rate = current_peak;
            return true;
        }

        if rate_gt(&self.config.default_high, &self.config.default_low) {
            self.low_rate = self.config.default_low.clone();
            self.high_rate = self.config.default_high.clone();
            return true;
        }

        false
    }

    fn apply_mode(&mut self, new_mode: Mode) -> bool {
        if self.mode == new_mode {
            return true;
        }

        if !dynamic_enabled() {
            return false;
        }

        if !self.load_rates() {
            log_msg("dynamic enabled but no valid refresh range");
            return false;
        }

        let min_rate = match new_mode {
            Mode::High => self.high_rate.clone(),
            Mode::Low => self.low_rate.clone(),
            Mode::Unknown => return false,
        };

        if !put_rate_setting("peak_refresh_rate", &self.high_rate) {
            log_msg(&format!("failed setting peak_refresh_rate={}", self.high_rate));
            return false;
        }

        if !put_rate_setting("min_refresh_rate", &min_rate) {
            log_msg(&format!("failed setting min_refresh_rate={min_rate}"));
            return false;
        }

        self.mode = new_mode;

        let mode_name = match new_mode {
            Mode::Low => "low",
            Mode::High => "high",
            Mode::Unknown => "unknown",
        };

        log_msg(&format!(
            "mode={mode_name} min={min_rate} peak={}",
            self.high_rate
        ));

        true
    }

    fn set_low(&mut self) -> bool {
        self.apply_mode(Mode::Low)
    }

    fn set_high(&mut self) -> bool {
        self.apply_mode(Mode::High)
    }
}

fn set_nonblocking(fd: RawFd) -> io::Result<()> {
    let flags = unsafe { libc::fcntl(fd, libc::F_GETFL) };

    if flags < 0 {
        return Err(io::Error::last_os_error());
    }

    let rc = unsafe { libc::fcntl(fd, libc::F_SETFL, flags | libc::O_NONBLOCK) };

    if rc < 0 {
        Err(io::Error::last_os_error())
    } else {
        Ok(())
    }
}

fn open_inputs(paths: &[String]) -> Vec<InputDev> {
    let mut out = Vec::new();

    for path in paths {
        match OpenOptions::new().read(true).open(path) {
            Ok(file) => {
                if let Err(e) = set_nonblocking(file.as_raw_fd()) {
                    log_msg(&format!("failed setting nonblock on {path}: {e}"));
                    continue;
                }

                out.push(InputDev {
                    path: path.clone(),
                    file,
                });
            }
            Err(e) => log_msg(&format!("failed opening {path}: {e}")),
        }
    }

    out
}

fn read_input_event(file: &mut File) -> io::Result<InputEvent> {
    let mut event = MaybeUninit::<InputEvent>::uninit();

    let buf = unsafe {
        std::slice::from_raw_parts_mut(event.as_mut_ptr() as *mut u8, size_of::<InputEvent>())
    };

    file.read_exact(buf)?;
    Ok(unsafe { event.assume_init() })
}

fn drain_input_events(input: &mut InputDev) -> io::Result<bool> {
    let mut saw_syn_report = false;

    loop {
        match read_input_event(&mut input.file) {
            Ok(event) => {
                if event.type_ == EV_SYN && event.code == SYN_REPORT {
                    saw_syn_report = true;
                }
            }
            Err(e) if e.kind() == io::ErrorKind::WouldBlock => {
                return Ok(saw_syn_report);
            }
            Err(e) if e.kind() == io::ErrorKind::Interrupted => {
                continue;
            }
            Err(e) => {
                return Err(e);
            }
        }
    }
}

fn make_pipe() -> io::Result<(RawFd, RawFd)> {
    let mut fds = [0; 2];

    let rc = unsafe { libc::pipe(fds.as_mut_ptr()) };

    if rc != 0 {
        return Err(io::Error::last_os_error());
    }

    set_nonblocking(fds[0])?;
    let _ = set_nonblocking(fds[1]);

    Ok((fds[0], fds[1]))
}

fn spawn_property_watcher(write_fd: RawFd) {
    thread::spawn(move || {
        let props = [PROP_DYNAMIC, PROP_LOW, PROP_HIGH];
        let mut watched = props
            .iter()
            .map(|name| WatchedProperty::new(name))
            .collect::<Vec<_>>();

        while RUNNING.load(Ordering::SeqCst) {
            let mut changed = false;
            for property in &mut watched {
                if property.wait_changed(Duration::from_millis(250)) {
                    changed = true;
                }
            }

            if changed {
                notify_property_pipe(write_fd);
            }
        }

        unsafe {
            libc::close(write_fd);
        }
    });
}

struct WatchedProperty {
    name: &'static str,
    info: Option<coreshift_core::android_property::AndroidPropertyInfo>,
    serial: u32,
    last_value: String,
}

impl WatchedProperty {
    fn new(name: &'static str) -> Self {
        let info = coreshift_core::android_property::android_property_find(name);
        let (serial, last_value) = if let Some(info) = info {
            coreshift_core::android_property::android_property_read(info)
                .map(|value| (value.serial, value.value))
                .unwrap_or_else(|_| (0, prop_get(name)))
        } else {
            (0, prop_get(name))
        };
        Self {
            name,
            info,
            serial,
            last_value,
        }
    }

    fn wait_changed(&mut self, timeout: Duration) -> bool {
        if self.info.is_none() {
            self.info = coreshift_core::android_property::android_property_find(self.name);
        }

        if let Some(info) = self.info {
            match coreshift_core::android_property::android_property_wait(
                info,
                self.serial,
                Some(timeout),
            ) {
                Ok(Some(serial)) => {
                    self.serial = serial;
                    if let Ok(value) = coreshift_core::android_property::android_property_read(info)
                    {
                        self.last_value = value.value;
                        self.serial = value.serial;
                    }
                    return true;
                }
                Ok(None) => return false,
                Err(_) => {
                    self.info = None;
                }
            }
        }

        let value = prop_get(self.name);
        if value != self.last_value {
            self.last_value = value;
            true
        } else {
            thread::sleep(timeout);
            false
        }
    }
}

fn notify_property_pipe(write_fd: RawFd) {
    let byte = [1u8];
    unsafe {
        libc::write(write_fd, byte.as_ptr() as *const libc::c_void, 1);
    }
}

fn drain_fd(fd: RawFd) {
    let mut buf = [0u8; 128];

    loop {
        let rc = unsafe { libc::read(fd, buf.as_mut_ptr() as *mut libc::c_void, buf.len()) };

        if rc > 0 {
            continue;
        }

        if rc < 0 {
            let err = io::Error::last_os_error();

            if err.kind() == io::ErrorKind::WouldBlock {
                break;
            }
        }

        break;
    }
}

fn print_status(mut daemon: Daemon) {
    println!("prop={PROP_DYNAMIC}={}", prop_get(PROP_DYNAMIC));
    println!("prop_low={PROP_LOW}={}", prop_get(PROP_LOW));
    println!("prop_high={PROP_HIGH}={}", prop_get(PROP_HIGH));
    println!("idle_ms={}", daemon.config.idle_ms);
    println!("input_devices={}", daemon.config.input_devices.join(" "));
    println!("current_min_refresh_rate={}",
        read_rate_setting("min_refresh_rate")
    );
    println!("current_peak_refresh_rate={}",
        read_rate_setting("peak_refresh_rate")
    );

    if daemon.load_rates() {
        println!("chosen_low_rate={}", daemon.low_rate);
        println!("chosen_high_rate={}", daemon.high_rate);
    } else {
        println!("chosen_low_rate=none");
        println!("chosen_high_rate=none");
    }
}

fn handle_cli() -> bool {
    let args: Vec<String> = env::args().collect();

    if args.len() <= 1 {
        return false;
    }

    let config = load_config();

    match args[1].as_str() {
        "--status" => {
            print_status(Daemon::new(config));
            true
        }
        "--enable" => {
            if prop_set(PROP_DYNAMIC, "1") {
                println!("{PROP_DYNAMIC}=1");
                true
            } else {
                eprintln!("failed setting {PROP_DYNAMIC}");
                std::process::exit(1);
            }
        }
        "--disable" => {
            if prop_set(PROP_DYNAMIC, "0") {
                println!("{PROP_DYNAMIC}=0");
                true
            } else {
                eprintln!("failed setting {PROP_DYNAMIC}");
                std::process::exit(1);
            }
        }
        "--set" => {
            if args.len() != 4
                || !valid_rate(&args[2])
                || !valid_rate(&args[3])
                || !rate_gt(&args[3], &args[2])
            {
                eprintln!("usage: inoi_refresh_idle --set LOW HIGH");
                std::process::exit(2);
            }

            let ok = prop_set(PROP_LOW, &args[2])
                && prop_set(PROP_HIGH, &args[3])
                && prop_set(PROP_DYNAMIC, "1");

            if ok {
                println!("{PROP_LOW}={}", args[2]);
                println!("{PROP_HIGH}={}", args[3]);
                println!("{PROP_DYNAMIC}=1");
                true
            } else {
                eprintln!("failed setting refresh props");
                std::process::exit(1);
            }
        }
        _ => false,
    }
}

fn run_daemon() -> io::Result<()> {

    install_signal_handlers();

    let config = load_config();
    let mut daemon = Daemon::new(config);
    let mut inputs = open_inputs(&daemon.config.input_devices);

    let (prop_read_fd, prop_write_fd) = make_pipe()?;
    spawn_property_watcher(prop_write_fd);

    let mut last_props = prop_state();
    let mut idle_deadline: Option<i64> = None;

    log_msg(&format!(
        "daemon started idle={}ms input={}",
        daemon.config.idle_ms,
        daemon.config.input_devices.join(" ")
    ));

    if dynamic_enabled() {
        daemon.set_low();
    } else {
        log_msg("dynamic disabled; waiting for property change");
    }

    while RUNNING.load(Ordering::SeqCst) {
        if inputs.is_empty() {
            inputs = open_inputs(&daemon.config.input_devices);
        }

        let mut pollfds = Vec::<libc::pollfd>::new();

        pollfds.push(libc::pollfd {
            fd: prop_read_fd,
            events: libc::POLLIN,
            revents: 0,
        });

        for input in &inputs {
            pollfds.push(libc::pollfd {
                fd: input.file.as_raw_fd(),
                events: libc::POLLIN,
                revents: 0,
            });
        }

        let timeout = if daemon.mode == Mode::High {
            poll_timeout_from_deadline(idle_deadline)
        } else {
            -1
        };

        let rc = unsafe {
            libc::poll(
                pollfds.as_mut_ptr(),
                pollfds.len() as libc::nfds_t,
                timeout,
            )
        };

        if rc == 0 {
            if daemon.mode == Mode::High {
                daemon.set_low();
                idle_deadline = None;
            }
            continue;
        }

        if rc < 0 {
            let err = io::Error::last_os_error();

            if err.raw_os_error() == Some(libc::EINTR) {
                continue;
            }

            log_msg(&format!("poll failed: {err}"));
            thread::sleep(std::time::Duration::from_secs(1));
            continue;
        }

        if pollfds[0].revents & libc::POLLIN != 0 {
            drain_fd(prop_read_fd);

            let now_props = prop_state();

            if now_props != last_props {
                last_props = now_props;

                if dynamic_enabled() {
                    if daemon.mode == Mode::Unknown {
                        daemon.set_low();
                        idle_deadline = None;
                    } else {
                        let current_mode = daemon.mode;
                        daemon.mode = Mode::Unknown;
                        daemon.apply_mode(current_mode);
                    }
                } else {
                    if daemon.mode != Mode::Unknown {
                        log_msg("dynamic disabled");
                    }

                    daemon.mode = Mode::Unknown;
                    idle_deadline = None;
                }
            }
        }

        let mut broken_paths = Vec::<String>::new();
        let mut saw_input = false;

        for (index, pfd) in pollfds.iter().enumerate().skip(1) {
            let input_index = index - 1;

            if pfd.revents & (libc::POLLERR | libc::POLLHUP | libc::POLLNVAL) != 0 {
                broken_paths.push(inputs[input_index].path.clone());
                continue;
            }

            if pfd.revents & libc::POLLIN == 0 {
                continue;
            }

            match drain_input_events(&mut inputs[input_index]) {
                Ok(true) => {
                    saw_input = true;
                }
                Ok(false) => {}
                Err(e) => {
                    log_msg(&format!(
                        "input read failed on {}: {e}",
                        inputs[input_index].path
                    ));
                    broken_paths.push(inputs[input_index].path.clone());
                }
            }
        }

        if saw_input && dynamic_enabled() {
            daemon.set_high();
            idle_deadline = Some(monotonic_ms() + daemon.config.idle_ms as i64);
        }

        if !broken_paths.is_empty() {
            inputs.retain(|input| !broken_paths.iter().any(|p| p == &input.path));
        }
    }

    if dynamic_enabled() {
        daemon.mode = Mode::Unknown;
        daemon.set_low();
    }

    unsafe {
        libc::close(prop_read_fd);
    }

    log_msg("daemon stopped");
    Ok(())
}

fn main() {
    if handle_cli() {
        return;
    }

    if let Err(e) = run_daemon() {
        log_msg(&format!("fatal: {e}"));
        std::process::exit(1);
    }
}
