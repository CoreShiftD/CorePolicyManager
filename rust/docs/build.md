# Build, Release, and Cross-Compilation

CoreShift supports building for host development and cross-compiling for Android targets using the NDK.

## Host GNU Build

For local development and testing of the `core` state machine:
```bash
cd rust
cargo build
```

## Android NDK Cross-Compilation

The official packaging uses a shell script to build and place the resulting `libcoreshift.so` executable binaries into the Android application's `jniLibs` directory.

### Targets Built
- `aarch64-linux-android` (arm64-v8a)
- `armv7-linux-androideabi` (armeabi-v7a)

### Build Script

From the repository root, run:
```bash
./scripts/build-rust-android.sh
```
This script resolves the target directories, runs `cargo build --release --target <target> -j 1`, and copies the resulting binary.

## Static vs Dynamic Realities

While packaged as a `.so` file inside `jniLibs` (`libcoreshift.so`) by the build script, it is actually compiled via `cargo build --release` as a standard Rust executable binary. Android's packaging system permits loading raw executables if masqueraded as native shared libraries (due to `extractNativeLibs` packaging behavior), allowing the daemon to be launched out of the app's `lib/` directory via standard `Runtime.getRuntime().exec()` calls from the Java/Kotlin frontend.

### Deployment Compatibility Notes

When launching the masqueraded `.so` executable via `Runtime.getRuntime().exec()`, consider the following caveats:
- **Android Version Caveats**: Starting with Android 10 (API 29), executing files directly from the application's home directory (e.g., `getFilesDir()`) is blocked due to W^X enforcement. However, executing from the `nativeLibraryDir` (`/data/app/.../lib/...`) is explicitly supported for packaged `jniLibs`.
- **SELinux Caveats**: Android's `untrusted_app` SELinux context restricts background processes. The daemon inherits the parent app's SELinux domain unless explicitly launched via a root shell (`su`).
- **`noexec` Mount Caveats**: The `/data/local/tmp` directory may be mounted with the `noexec` flag on some locked-down stock ROMs, preventing manual CLI execution via `adb shell`. If this occurs, binaries must be executed via `app_process` or from within the app's `nativeLibraryDir`.
- **Rooted vs Non-Rooted Behavior**: On non-rooted devices, the daemon runs with standard app privileges and UID. It can only manipulate its own files or access public APIs. On a rooted device (when launched via `su`), the daemon can monitor global cgroups and system packages system-wide (e.g., the Preload addon).
