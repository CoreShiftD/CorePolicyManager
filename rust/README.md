# CoreShift Policy

CoreShift Policy is an Android-oriented autonomous daemon for lightweight system policy decisions. It provides a high-performance native execution engine designed to optimize system behavior with minimal overhead.

This project uses a two-binary model to separate production daemon logic from diagnostic tools.

## Binary Model
- **`corepolicy`**: The production daemon and primary CLI. This is the only binary required for normal operation.
- **`coredebug`**: An optional diagnostics binary for on-device substrate probes and manual verification.

## Build Commands
| Target | Command |
| :--- | :--- |
| **Production** | `cargo build --bin corepolicy` |
| **Debug Tool** | `cargo build --features debug-cli --bin coredebug` |

## Validation Model
- **`cargo test`**: The primary source of truth for development validation. All logic should be verified here using unit and integration tests.
- **`coredebug`**: Specialized for on-device diagnostics and manual probes that cannot be easily represented in a standard cargo test environment (e.g., specific Android kernel behaviors). It is **not** a replacement for `cargo test`.

## low_level API stability
The `coreshift-lowlevel` crate is the stable, frozen substrate of CoreShift Policy. 

- **Stability Guarantee**: Higher layers (daemon runtime, feature modules) build on this API. Breaking changes to the low-level substrate are avoided in favor of additive improvements.
- **Scope**: Owns syscalls, FFI, resource ownership (FDs/PIDs), and non-blocking I/O multiplexing.
- **Policy Neutral**: No business logic or daemon-level policy resides in the low-level crate.
- **Validation**: Public APIs should be covered by cargo tests before stabilization.


## Binary Identity (Production)
- **Product Name**: CoreShift Policy
- **Executable**: `corepolicy`
- **Daemon Identity**: CoreShift Policy Daemon (displayed in logs and help text)

## Planned CLI Model (`corepolicy`)
The `corepolicy` CLI is designed for simplicity and directness. 

**NOTE: Only help/placeholder behavior is currently implemented. All other commands are PLANNED.**

```bash
corepolicy           # Placeholder: prints implementation status
corepolicy -p        # (Planned) Preload mode: starts the autonomous loop with preload enabled
corepolicy status    # (Planned) Reads local status from /data/local/tmp/coreshift/status.json
corepolicy help      # Prints help and usage information (Implemented)
```

## Diagnostics CLI (`coredebug`)
The diagnostics binary is used for low-level substrate probes on Android devices.

```bash
# Build with: cargo build --features debug-cli --bin coredebug
coredebug probe paths       # Probe path existence/visibility (Implemented)
coredebug probe procfs      # Probe procfs helper behavior (Planned)
coredebug probe inotify     # Probe inotify substrate (Planned)
coredebug probe spawn       # Probe process spawning primitives (Planned)
```

## Runtime Model (Planned)
CoreShift Policy follows a lightweight, single-process execution model:
- **One Process**: Minimal resource footprint.
- **Main Thread Execution**: Leverages the `coreshift-lowlevel` reactor for asynchronous event multiplexing.
- **Tick Scheduler**: A deterministic scheduler drives periodic module tasks.
- **Autonomous Modules**: Feature modules (like preload) observe the system and act independently.

## Status and Logs
Observability is provided through local filesystem artifacts. All paths remain under `/data/local/tmp/coreshift/` for the current development phase.

- **Status File**: `/data/local/tmp/coreshift/status.json` (Planned JSON formatted state snapshot)
- **Core Logs**: `/data/local/tmp/coreshift/core.log` (Engine and module events)
- **Feature Logs**: `/data/local/tmp/coreshift/features/` (Planned module-specific diagnostics)

## Future Feature Flags
Additional compact flags (e.g., `-t`, `-b`) are **reserved** for upcoming policy modules. They are currently proposed and should not be treated as implemented or available in the current version.
