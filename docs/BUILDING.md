# Building

Build the Magisk module from the repository root:

```bash
./scripts/build-magisk-module.sh
```

## Requirements

- Rust toolchain.
- Android NDK via `ANDROID_NDK_HOME`, or an SDK install discoverable through
  `ANDROID_HOME` or `ANDROID_SDK_ROOT`.
- `zip`.

## Rust Build Flow

`scripts/build-rust-android.sh` builds:

- `aarch64-linux-android`
- `armv7-linux-androideabi`

It configures NDK clang linkers for API level 28 by default and verifies both
release binaries exist.

## Module Build Flow

`scripts/build-magisk-module.sh`:

1. Validates required packaging files.
2. Builds Android Rust binaries.
3. Creates `dist/magisk/CoreShiftPolicy`.
4. Stages ABI binaries under `bin/arm64-v8a` and `bin/armeabi-v7a`.
5. Copies Magisk scripts and default config.
6. Writes a `dist/CoreShiftPolicy-<version>.zip` named from `module.prop`.
7. Writes a `.sha256` checksum.
