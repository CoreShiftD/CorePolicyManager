# CoreShift Policy Rust Daemon

This `rust/` directory is the active native daemon source tree.

## Source of Truth

- Production daemon crate: [Cargo.toml](/work/CorePolicyManager/rust/Cargo.toml)
- Binary entrypoint: [src/main.rs](/work/CorePolicyManager/rust/src/main.rs)
- Runtime orchestration: [src/runtime/daemon.rs](/work/CorePolicyManager/rust/src/runtime/daemon.rs)
- Policy feature modules: [src/features/](/work/CorePolicyManager/rust/src/features)

## Boundaries

- `coreshift-lowlevel` owns low-level OS primitives such as reactor, inotify, procfs helpers, and shutdown signal installation.
- This crate owns daemon/runtime policy, foreground filtering, and preload path discovery.

## Build

```bash
cargo build
```

The Android packaging script at [../scripts/build-rust-android.sh](/work/CorePolicyManager/scripts/build-rust-android.sh) builds this crate and copies the resulting `corepolicy` executable into the app's `jniLibs` payload.
