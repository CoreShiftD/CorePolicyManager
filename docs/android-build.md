# Building Rust Daemon for Android

This project supports cross-compilation for Android ARM targets using a dedicated build script.

## Prerequisites

1.  **Android NDK**: Ensure you have the Android NDK installed.
2.  **Environment Variables**: Set `ANDROID_NDK_HOME` to your NDK path.
    Also, ensure the NDK LLVM binaries are in your `PATH`:
    ```bash
    export PATH=$ANDROID_NDK_HOME/toolchains/llvm/prebuilt/linux-x86_64/bin:$PATH
    ```

## Build and Package

Use the provided script to build for all supported targets and package them for the Android app:

```bash
./scripts/build-rust-android.sh
```

### Script Behavior
- **`CARGO_TARGET_DIR`**: The script respects your `CARGO_TARGET_DIR` if set. In CI or if unset, it defaults to a repo-local directory: `rust/target`.
- **Packaging**: It creates the required `jniLibs` structure in the `app` module and copies the binaries as `libcoreshift.so`.
- **Permissions**: It applies `chmod 755` to the packaged binaries.

### Supported Targets
- **ARM64 (arm64-v8a)**: `aarch64-linux-android`
- **ARMv7 (armeabi-v7a)**: `armv7-linux-androideabi`

## Output Artifacts

The final packaged payloads are located at:
- `app/src/main/jniLibs/arm64-v8a/libcoreshift.so`
- `app/src/main/jniLibs/armeabi-v7a/libcoreshift.so`

**Important**: These are **executable ELF PIE payloads**, not JNI shared libraries. They are named with `.so` to ensure the Android Package Manager extracts them upon installation.

## CI Configuration
The CI environment automatically installs the required Rust targets and uses the repo-local `rust/target` directory for deterministic builds.
