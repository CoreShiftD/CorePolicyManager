# CoreShift Policy

CoreShift Policy is an Android-oriented autonomous daemon for lightweight system policy decisions. It provides a high-performance native execution engine designed to optimize system behavior with minimal overhead.

The executable binary is named `corepolicy`.

## Binary Identity
- **Product Name**: CoreShift Policy
- **Executable**: `corepolicy`
- **Daemon Identity**: CoreShift Policy Daemon (displayed in logs and help text)

## CLI Usage
The `corepolicy` CLI is designed for simplicity and directness:

```bash
corepolicy           # Default mode: runs all stable enabled daemon features
corepolicy -p        # Preload mode: starts the autonomous loop with preload enabled
corepolicy status    # Reads local status from /data/local/tmp/coreshift/status.json
corepolicy help      # Prints help and usage information
```

## Runtime Model
CoreShift Policy follows a lightweight, single-process execution model:
- **One Process**: Minimal resource footprint.
- **Main Thread Execution**: Leverages the `low_level` reactor for asynchronous event multiplexing.
- **Tick Scheduler**: A deterministic scheduler drives periodic module tasks.
- **Autonomous Modules**: Feature modules (like preload) observe the system and act independently.

## Current Feature: Preload
The **Preload** feature (`-p`) is the primary active module. It monitors process transitions and performs autonomous warmup of critical resources to reduce application launch latency.

## Status and Logs
Observability is provided through local filesystem artifacts. All paths remain under `/data/local/tmp/coreshift/` for the current development phase.

- **Status File**: `/data/local/tmp/coreshift/status.json` (JSON formatted state snapshot)
- **Core Logs**: `/data/local/tmp/coreshift/core.log` (Engine and module events)
- **Addon Logs**: `/data/local/tmp/coreshift/addons/` (Detailed module-specific diagnostics)

## Future Feature Flags
Additional compact flags (e.g., `-t`, `-b`) are **reserved** for upcoming policy modules. They are currently proposed and should not be treated as implemented or available in the current version.
