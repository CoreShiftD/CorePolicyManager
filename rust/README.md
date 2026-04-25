# CoreShift Policy

CoreShift Policy is an Android-oriented autonomous daemon for lightweight system policy decisions. It provides a high-performance native execution engine designed to optimize system behavior with minimal overhead.

The executable binary is named `corepolicy`.

## Binary Identity
- **Product Name**: CoreShift Policy
- **Executable**: `corepolicy`
- **Daemon Identity**: CoreShift Policy Daemon (displayed in logs and help text)

## Planned CLI Model
The `corepolicy` CLI is designed for simplicity and directness. 

**NOTE: Only help/placeholder behavior is currently implemented. All other commands are PLANNED.**

```bash
corepolicy           # Placeholder: prints implementation status
corepolicy -p        # (Planned) Preload mode: starts the autonomous loop with preload enabled
corepolicy status    # (Planned) Reads local status from /data/local/tmp/coreshift/status.json
corepolicy help      # Prints help and usage information (Implemented)
```

## Runtime Model (Planned)
CoreShift Policy follows a lightweight, single-process execution model:
- **One Process**: Minimal resource footprint.
- **Main Thread Execution**: Leverages the `low_level` reactor for asynchronous event multiplexing.
- **Tick Scheduler**: A deterministic scheduler drives periodic module tasks.
- **Autonomous Modules**: Feature modules (like preload) observe the system and act independently.

## Status and Logs
Observability is provided through local filesystem artifacts. All paths remain under `/data/local/tmp/coreshift/` for the current development phase.

- **Status File**: `/data/local/tmp/coreshift/status.json` (Planned JSON formatted state snapshot)
- **Core Logs**: `/data/local/tmp/coreshift/core.log` (Engine and module events)
- **Feature Logs**: `/data/local/tmp/coreshift/features/` (Planned module-specific diagnostics)

## Future Feature Flags
Additional compact flags (e.g., `-t`, `-b`) are **reserved** for upcoming policy modules. They are currently proposed and should not be treated as implemented or available in the current version.
