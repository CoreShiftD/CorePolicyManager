#!/bin/bash
set -euo pipefail

PROJECT_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
RUST_ROOT="$PROJECT_ROOT/rust"

if [ -z "${ANDROID_NDK_HOME:-}" ]; then
    if [ -n "${ANDROID_HOME:-}" ] && [ -d "$ANDROID_HOME/ndk" ]; then
        ANDROID_NDK_HOME="$(find "$ANDROID_HOME/ndk" -mindepth 1 -maxdepth 1 -type d | sort -V | tail -n 1)"
    elif [ -n "${ANDROID_SDK_ROOT:-}" ] && [ -d "$ANDROID_SDK_ROOT/ndk" ]; then
        ANDROID_NDK_HOME="$(find "$ANDROID_SDK_ROOT/ndk" -mindepth 1 -maxdepth 1 -type d | sort -V | tail -n 1)"
    fi
fi

if [ -z "${ANDROID_NDK_HOME:-}" ] || [ ! -d "$ANDROID_NDK_HOME" ]; then
    echo "ANDROID_NDK_HOME must point to an Android NDK install" >&2
    exit 1
fi

HOST_TAG="linux-x86_64"
TOOLCHAIN="$ANDROID_NDK_HOME/toolchains/llvm/prebuilt/$HOST_TAG/bin"
API_LEVEL="${ANDROID_API_LEVEL:-28}"

export CARGO_TARGET_AARCH64_LINUX_ANDROID_LINKER="$TOOLCHAIN/aarch64-linux-android${API_LEVEL}-clang"
export CARGO_TARGET_ARMV7_LINUX_ANDROIDEABI_LINKER="$TOOLCHAIN/armv7a-linux-androideabi${API_LEVEL}-clang"

if [ ! -x "$CARGO_TARGET_AARCH64_LINUX_ANDROID_LINKER" ]; then
    echo "Missing linker: $CARGO_TARGET_AARCH64_LINUX_ANDROID_LINKER" >&2
    exit 1
fi

if [ ! -x "$CARGO_TARGET_ARMV7_LINUX_ANDROIDEABI_LINKER" ]; then
    echo "Missing linker: $CARGO_TARGET_ARMV7_LINUX_ANDROIDEABI_LINKER" >&2
    exit 1
fi

rustup target add aarch64-linux-android armv7-linux-androideabi

cd "$RUST_ROOT"
cargo build --release --target aarch64-linux-android
cargo build --release --target armv7-linux-androideabi

test -x target/aarch64-linux-android/release/corepolicy
test -x target/armv7-linux-androideabi/release/corepolicy
