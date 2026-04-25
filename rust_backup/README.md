# CoreShift Technical Reference

CoreShift is the high-performance native daemon backend for CorePolicyManager, written in Rust.
It acts as the system's execution engine, monitoring environmental changes and dispatching modular addons to adjust system behavior in real time.

This directory contains the definitive, source-of-truth technical documentation for the daemon.

## Deep Reference Guides

- **[Architecture Deep Dive](docs/architecture.md)**: Layering rules, pure vs. impure boundaries, and state machine model.
- **[Runtime Lifecycle](docs/runtime.md)**: Startup sequence, tick loop, and logging.
- **[IPC Protocol](docs/ipc.md)**: Socket framing, message formats, and backpressure limits.
- **[Build & Release](docs/build.md)**: Host builds, Android NDK cross-compilation, and `jniLibs` packaging.
- **[Testing & Validation](docs/testing.md)**: Cargo tests, linting, and offline replay debugging.
- **[Android Integration](docs/android.md)**: Platform services, system properties, and `inotify` hooks.
- **[Development Guide](docs/development.md)**: How to safely extend the daemon (adding new commands, actions, or addons).
- **[Known Limitations](docs/limitations.md)**: Unresolved caveats and experimental surfaces.
