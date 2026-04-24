# Rust Daemon Runtime Notes

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

## Target Architectures
- **arm64-v8a**: `lib/arm64-v8a/libcoreshift.so`
- **armeabi-v7a**: `lib/armeabi-v7a/libcoreshift.so`
