#!/bin/bash
# This Source Code Form is subject to the terms of the Mozilla Public
# License, v. 2.0. If a copy of the MPL was not distributed with this
# file, You can obtain one at https://mozilla.org/MPL/2.0/

set -e

# Resolve script and project directories
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
RUST_ROOT="$PROJECT_ROOT/rust"
JNI_LIBS_ROOT="$PROJECT_ROOT/app/src/main/jniLibs"

# Ensure Rust targets are installed
echo "Ensuring Rust targets are available..."
rustup target add aarch64-linux-android armv7-linux-androideabi

# Resolve actual target directory from cargo
cd "$RUST_ROOT"
CARGO_TARGET_DIR=$(cargo metadata --format-version 1 | grep -o '"target_directory":"[^"]*"' | head -n 1 | cut -d'"' -f4)
echo "Resolved CARGO_TARGET_DIR: $CARGO_TARGET_DIR"

# Build binaries
echo "Building for aarch64-linux-android..."
cargo build --release --target aarch64-linux-android -j 1

echo "Building for armv7-linux-androideabi..."
cargo build --release --target armv7-linux-androideabi -j 1

# Define expected binary paths
BINARY_ARM64="$CARGO_TARGET_DIR/aarch64-linux-android/release/CoreShift"
BINARY_ARMV7="$CARGO_TARGET_DIR/armv7-linux-androideabi/release/CoreShift"

# Validation
function check_binary() {
    if [ ! -f "$1" ]; then
        echo "ERROR: Binary not found at $1"
        echo "Found the following files in target release directories:"
        find "$CARGO_TARGET_DIR" -maxdepth 4 -type f | grep "release/" | head -n 20
        exit 1
    fi
}

check_binary "$BINARY_ARM64"
check_binary "$BINARY_ARMV7"

# Create jniLibs directories
echo "Preparing jniLibs packaging..."
mkdir -p "$JNI_LIBS_ROOT/arm64-v8a"
mkdir -p "$JNI_LIBS_ROOT/armeabi-v7a"

# Copy and rename binaries
echo "Copying executable payloads to jniLibs..."
cp "$BINARY_ARM64" "$JNI_LIBS_ROOT/arm64-v8a/libcoreshift.so"
cp "$BINARY_ARMV7" "$JNI_LIBS_ROOT/armeabi-v7a/libcoreshift.so"

# Ensure executability
chmod 755 "$JNI_LIBS_ROOT/arm64-v8a/libcoreshift.so"
chmod 755 "$JNI_LIBS_ROOT/armeabi-v7a/libcoreshift.so"

# Print verification info
echo "Verifying packaged binaries:"
file "$JNI_LIBS_ROOT/arm64-v8a/libcoreshift.so"
file "$JNI_LIBS_ROOT/armeabi-v7a/libcoreshift.so"

echo "Rust Android build and packaging successfully complete."
