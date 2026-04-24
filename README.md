# CorePolicyManager

Android policy and performance management platform powered by **CoreShift**.

> [!CAUTION]
> **Project Status: EXPERIMENTAL / TESTING PHASE**
> This software is currently in an active testing phase. It is not production-ready. Use with caution on physical devices as it interacts with low-level system resources.

## Overview

CorePolicyManager is a comprehensive Android management platform designed to optimize system performance and enforce operational policies. 

The project is split into two halves:
1. **CorePolicyManager (App)**: A high-level Android application frontend providing configuration and user interfaces.
2. **CoreShift (Daemon)**: A high-performance native backend written in Rust. It serves as the system's execution engine, monitoring environment changes, spawning tasks, and dispatching modular addons (like process preloading) to adjust system behavior in real time.

## Deployment Model (Planned)

The intended deployment model is for the Android app to extract the CoreShift daemon from its `jniLibs` folder and launch it dynamically as an isolated process. Communication between the frontend App and the daemon will occur over a secure UNIX domain socket (`coreshift.sock`). Currently, the daemon must be launched manually via an `adb shell`.

## Documentation

- **[Daemon Usage Guide](docs/daemon-usage.md)**: How to run the daemon, debug failures, socket paths, and logging.
- **[Developer Quickstart](docs/quickstart.md)**: Prerequisites, building, and running.
- **[CoreShift Architecture](rust/README.md)**: Definitive technical reference and deep dive into the Rust engine internals.

## License
This project is licensed under the Mozilla Public License, v. 2.0. See the [LICENSE](LICENSE) file for the full license text.
