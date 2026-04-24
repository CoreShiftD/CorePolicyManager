# CorePolicyManager

Android policy and performance management platform powered by **CoreShift**.

> [!CAUTION]
> **Project Status: EXPERIMENTAL / TESTING PHASE**
> This software is currently in an active testing phase. It is not production-ready. Use with caution on physical devices as it interacts with low-level system resources.

## Overview

CorePolicyManager is a comprehensive Android management platform designed to optimize system performance and enforce operational policies. The project consists of a high-level Android application (frontend) and **CoreShift**, a high-performance native daemon (backend) written in Rust.

CoreShift acts as the system's execution engine, monitoring environmental changes and dispatching modular "addons" to adjust system behavior in real-time.

## Architecture

The system follows a decoupled, asynchronous architecture:

- **Frontend**: Jetpack Compose-based Android application.
- **Backend (CoreShift)**: Native Rust daemon using a deterministic state-machine core.
- **IPC**: Communication via Unix Domain Sockets with bounded packet sizes and security verification.
- **Lifecycle**: Packaged as an executable ELF PIE payload within the Android APK and extracted to internal storage at runtime.

## CoreShift Engine

The CoreShift engine is the kernel of the platform, providing:

- **System Observability**: High-frequency monitoring of foreground PIDs and package modifications.
- **Deterministic Scheduler**: A priority-aware queue system that manages work budgets and prevents background task starvation.
- **Capability-Based Security**: Strict enforcement of action permissions for internal modules and IPC clients.
- **Structured Logging**: Key-value paired telemetry for core metrics and addon decisions.

## Current Feature: Preload Addon

The **Preload Addon** is the first implemented optimization module. It aims to reduce application launch times by warming critical assets when a foreground change is detected.

**Safety Features:**
- **Disabled by Default**: Requires explicit activation via control file.
- **Intelligent Scheduling**: Debounces rapid foreground switches and suppresses duplicate warmup requests.
- **Resource Constraints**: Enforces global concurrency limits and per-package cooldown windows.
- **Health Protection**: Implements failure backoff and will auto-disable if a global error threshold is reached.

## Runtime Controls

CoreShift is controlled through the Android shell via trigger files located in the centralized runtime directory.

**Base Directory**: `/data/local/tmp/coreshift/`

| Feature | Command |
| :--- | :--- |
| **Enable Preload** | `touch /data/local/tmp/coreshift/control/enable_preload` |
| **Disable Preload** | `rm /data/local/tmp/coreshift/control/enable_preload` |
| **Debug Logs** | `touch /data/local/tmp/coreshift/control/log_debug` |
| **Trace Logs** | `touch /data/local/tmp/coreshift/control/log_trace` |
| **Reset Verbosity** | `rm /data/local/tmp/coreshift/control/log_*` |

**Log Locations:**
- **Core Engine**: `/data/local/tmp/coreshift/core.log`
- **Preload Addon**: `/data/local/tmp/coreshift/addons/addon_102.log`

## Build Instructions

### Prerequisites
- Android NDK (API 28+)
- Rust (Stable) with Android targets

### Supported Targets
- `aarch64-linux-android` (arm64-v8a)
- `armv7-linux-androideabi` (armeabi-v7a)

### Build and Package
Use the unified build script to compile the native engine and package it for the Android app:
```bash
./scripts/build-rust-android.sh
```

The resulting binaries are packaged as `libcoreshift.so` executable payloads within the `jniLibs` directory.

## Testing Focus

Current testing is concentrated on the following areas:
- **System Stability**: Long-running (24h+) soak tests to verify zero file descriptor leaks or memory creep.
- **Battery Impact**: Monitoring idle wakeups and CPU cycles consumed during warmup cycles.
- **Thermal Behavior**: Ensuring I/O heavy operations do not trigger significant thermal throttling.
- **Launch Performance**: Quantifying actual reduction in "cold-start" duration for targeted applications.
- **Compatibility**: Testing across different Android ROMs and kernel versions.

## Roadmap
- [ ] JNI Bridge for direct Android UI integration.
- [ ] Modular policy addon for CPU frequency scaling.
- [ ] Extended metrics dashboard in the Android frontend.
- [ ] Persistent configuration storage.

## Safety Notice
CoreShift performs direct system calls and file I/O that may affect device stability. If the system detects repetitive failures, it is designed to enter a safe "Auto-Disabled" state to protect the host device.

**AI Workflows**: GitHub Actions labeled with "AI" are automated review and report generators. They create isolated branches for manual inspection and **never** push directly to the `main` branch. Detailed setup and configuration can be found in [docs/ci-ai.md](docs/ci-ai.md).

## License
Private / In Development. All rights reserved.
