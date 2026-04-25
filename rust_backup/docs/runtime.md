# Runtime Lifecycle

The `runtime` module and `run_daemon` orchestrate the execution loop, bridging pure state transitions with system side effects.

## Daemon Startup Sequence

Tracing the initialization in `run_daemon`:
1. **Directory Creation**: Ensures `/data/local/tmp/coreshift` (and `control`, `addons`) exist using `paths::ensure_dirs()`.
2. **PID Management**: Writes the current process ID to `/data/local/tmp/coreshift/coreshift.pid`.
3. **Reactor Bind**: Binds a non-blocking `AF_UNIX` socket to `/data/local/tmp/coreshift/coreshift.sock` and adds it to the asynchronous `Reactor` multiplexer.
4. **Addon Loading**: Instantiates modules. Currently: `NoOpAddon` (100), `EchoAddon` (101), and optionally `PreloadAddon` (102) if started with the `preload` command. Starting with `preload` automatically writes the `enable_preload` control file if it is missing.
5. **Initial Logging**: Emits the daemon start event.
   - Source log: `"daemon start version=0.1.0 git=a472b4f log_schema=structured_v2"`.
6. **Inotify Setup**: If preload is enabled, watches `/dev/cpuset/top-app/cgroup.procs`, `/data/system/packages.xml`, and `/data/system/packages.list`.

## Foreground Filtering

Preload foreground handling is intentionally cheap. A cgroup event first reads
`/dev/cpuset/top-app/cgroup.procs` and ignores duplicate PIDs. For a new PID,
the runtime reads `/proc/<pid>/status` and parses only `Name` and `Uid`.
Processes that vanished, have UID below `10000`, use obvious Android system
package names, or have no dot in `Name` do not trigger preload. Only surviving
package-like candidates read `/proc/<pid>/cmdline`. Multiprocess names are
normalized at the first `:` and usually accepted as the base package; known
helper suffixes such as `sandboxed_process`, `renderer`, `webview`, `gpu`,
`isolated`, and `privileged_process` are skipped. Package database files remain
cache invalidation signals only and are not parsed in this hot path.

## The Event Loop

The daemon operates in a continuous tick loop (`TICK_MS = 16` milliseconds):
- **Time Computation**: It measures elapsed milliseconds and processes logical `Event::TimeAdvanced(16)` sys_events.
- **Reactor Wait**: The loop is **not a busy poll**. It blocks via `epoll/kqueue` (in `low_level::reactor`) until `compute_reactor_timeout_ms` specifies a deadline, effectively sleeping until I/O events, inotify changes, or the next tick limit. This minimizes idle battery drain.
- **Action Budgets**: Bounded by `MAX_ACTIONS_PER_TICK` (10,000) to prevent CPU saturation in tight loops.

## Shutdown & Restart

- Uses atomic flags and intercepts `SIGTERM` and `SIGINT` signals to break out of the `while RUNNING.load()` loop.
- Reactor failure limits: If the reactor fails 10 consecutive times, the daemon intentionally exits.

## Structured Logging

- Logs are strictly routed as `LogEvent` items emitted through the `core::Effect::Log`.
- Daemon events target `/data/local/tmp/coreshift/core.log`.
- Addons write to `addons/addon_<ID>.log`.
- **Fallback**: As verified in `runtime/logging.rs`, if the log file path cannot be opened (e.g., due to permissions), the `FileSink` emits a warning to `stderr` and silently degrades to writing to `/dev/null` instead of crashing. There is no `stdout` fallback for daemon log events.
