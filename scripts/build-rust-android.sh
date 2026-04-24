#!/bin/bash
set -e

# Resolve script and project directories
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
RUST_ROOT="$PROJECT_ROOT/rust"
JNI_LIBS_ROOT="$PROJECT_ROOT/app/src/main/jniLibs"

# Set deterministic target directory
export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-$RUST_ROOT/target}"
echo "Using CARGO_TARGET_DIR: $CARGO_TARGET_DIR"

# Ensure Rust targets are installed
echo "Ensuring Rust targets are available..."
rustup target add aarch64-linux-android armv7-linux-androideabi

# Build binaries
cd "$RUST_ROOT"

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
# These are executable payloads packaged with .so extension to force Android PM extraction.
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
