# Rust Daemon Runtime Notes

This document describes the daemon runtime contract, not the Android UI.

## Packaging Strategy
The Rust daemon is packaged as `libcoreshift.so` within the `jniLibs` directory. 
**Note**: This is a packaged **executable binary (ELF PIE)**, not a JNI shared library. 
It is named with a `.so` prefix to ensure the Android Package Manager (PM) extracts it to the application's native library directory upon installation.

## Runtime Execution
Because modern Android versions restrict the execution of binaries directly from the APK or certain storage locations, the Android application must follow these steps to run the daemon:

1.  **Locate the Binary**: Find the extracted `libcoreshift.so` in the app's `nativeLibraryDir`.
2.  **Copy to Internal Storage**: Copy the file from the native library directory to the app's internal files directory (e.g., `/data/data/<package_name>/files/coreshift`).
3.  **Set Permissions**: Apply execute permissions to the copied file:
    ```java
    file.setExecutable(true, true); // Equivalent to chmod 700
    ```
4.  **Execute**: Start the process using `ProcessBuilder` or `Runtime.exec()`.

## Runtime Output Model

- **CLI help mode** (`help`, `--help`, `-h`) writes help text to stdout for normal shell usage.
- **CLI replay/record modes** stay shell-facing entrypoints. They are not daemon lifecycle logging surfaces.
- Replay file-open failures surface as normal CLI errors instead of aborting with a panic.
- **Daemon mode** writes runtime output through structured logging only.
- Structured daemon output is routed through `LogLevel`, `LogEvent`, and the runtime `LogRouter`.
- Core logs default to `/data/local/tmp/coreshift/core.log`; addon logs are written under `/data/local/tmp/coreshift/addons/`.

## Runtime Responsibility Split

- `runtime/logging.rs`: formatting, routing, and ownership-aware log sinks.
- `runtime/control.rs`: signal mapping, exit status translation, and process-control helpers.
- `runtime/effects.rs`: side-effect execution for `core::Effect` values.
- `runtime/system_services.rs`: Android and system-facing service requests.

Pure reducer, scheduler, and state-machine logic must stay in `core`.

## IPC Boundary Behavior

- IPC clients are authenticated with `SO_PEERCRED`.
- Requests are bounded by `MAX_PACKET_SIZE` and `MAX_READ_BUF`.
- Response frames are bounded by `MAX_WRITE_BUF` as a whole frame, not just the existing queue length.
- If queueing a response would exceed the write buffer limit, the daemon logs the overflow and disconnects that client explicitly.
- Disconnects are intentional backpressure, not silent lossy delivery. Once a client crosses the queue limit it must reconnect and retry.

## Spawn Validation

- `ExecContext::new` rejects:
  - empty argv
  - argv entries containing interior NUL bytes
  - env entries containing interior NUL bytes
  - cwd values containing interior NUL bytes
- Invalid exec context construction is reported back as a normal spawn failure rather than being silently rewritten or partially dropped.

## Long-Lived Runtime Safety

- Arena generation `0` is reserved as invalid, so reused slots continue to reject stale handles after wrap.
- Runtime cleanup paths should convert failure into explicit events or structured logs instead of panicking.
- Periodic `core::verify` drift checks in daemon mode now log invariant failures through structured logging instead of panicking the process.
- Verification helpers are still useful for invariant checking and tests, but daemon release paths should prefer recoverable failures.

## Target Architectures
- **arm64-v8a**: `lib/arm64-v8a/libcoreshift.so`
- **armeabi-v7a**: `lib/armeabi-v7a/libcoreshift.so`
