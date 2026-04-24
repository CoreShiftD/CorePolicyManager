# Structured Logging

To ensure diagnostic utility on headless platforms, CoreShift strictly avoids ad-hoc `println!` or `dbg!` statements.

## Architecture

- **`core::LogEvent` / `LogLevel`**: Enumerations representing strongly-typed events and severities.
- **`runtime::logging`**: The runtime component that formats events and flushes them to the appropriate output mechanism (file log router, ring buffer).

## Control Triggers

Logging verbosity is dynamically controlled via file triggers mapped into the system's temporary directory (`/data/local/tmp/coreshift/control/`):
- `log_debug`: Sets the daemon log level to Debug.
- `log_trace`: Sets the daemon log level to Trace.

## Output Locations

By default, structured logs are written to runtime files:
- Daemon events: `/data/local/tmp/coreshift/core.log`
- Addon-specific events: e.g., `/data/local/tmp/coreshift/addons/addon_102.log`

## Idle Diagnostics

To maintain a high signal-to-noise ratio, the daemon heavily rate-limits repetitive metrics during idle periods. In `Info` mode, metrics are only logged when meaningful state changes occur (e.g., active clients, queue depth changes). If the daemon is completely idle, it emits a heartbeat (`daemon idle: listening for IPC clients`) roughly every 100 seconds. Setting the verbosity to `Debug` or `Trace` overrides this and forces continuous metric output.
