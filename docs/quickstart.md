# CoreShift Quickstart

## What is CoreShift?
CoreShift is a high-performance native daemon written in Rust. It serves as the system execution engine, monitoring environment changes and safely dispatching system optimizations via modular addons.

## What is CorePolicyManager?
CorePolicyManager is the Android application frontend that interacts with the CoreShift daemon. It provides a user interface and configuration layer over the native daemon.

## Prerequisites
- Android NDK (API 28+)
- Rust (stable) with Android targets (`aarch64-linux-android`, `armv7-linux-androideabi`)

## Build Instructions

To compile the CoreShift native engine and package it into the Android project's `jniLibs` folder, run the following from the **repository root**:

```bash
./scripts/build-rust-android.sh
```

For purely building the Rust daemon for your host architecture, run this from the **rust** directory:
```bash
cd rust
cargo build
```

## Running the Daemon
Once built and deployed to a device, you can run the daemon via an adb shell:

```bash
adb shell
cd /data/local/tmp/coreshift
./coreshift preload
```

For more details on CLI options, control triggers, and troubleshooting, see the [Daemon Usage](daemon-usage.md) guide.
