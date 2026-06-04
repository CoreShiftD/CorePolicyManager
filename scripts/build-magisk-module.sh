#!/bin/bash
set -euo pipefail

PROJECT_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$PROJECT_ROOT"

VERSION="$(sed -n 's/^version=//p' packaging/magisk/module.prop)"
ZIP_NAME="CoreShiftPolicy-${VERSION}.zip"
DIST_ROOT="dist"
MODULE_DIR="$DIST_ROOT/magisk/CoreShiftPolicy"
RUST_TARGET_DIR="${CARGO_TARGET_DIR:-rust/target}"

require_file() {
    if [ ! -f "$1" ]; then
        echo "Missing required file: $1" >&2
        exit 1
    fi
}

require_file packaging/magisk/module.prop
require_file packaging/magisk/service.sh
require_file packaging/magisk/customize.sh
require_file packaging/magisk/uninstall.sh
require_file packaging/magisk/corepolicy.conf
require_file packaging/magisk/utensil.conf
require_file packaging/magisk/gamelist.txt

./scripts/build-rust-android.sh

rm -rf "$MODULE_DIR"
mkdir -p "$MODULE_DIR/bin/arm64-v8a"
mkdir -p "$MODULE_DIR/bin/armeabi-v7a"
mkdir -p "$MODULE_DIR/system/bin"
mkdir -p "$DIST_ROOT/binaries"

cp packaging/magisk/module.prop "$MODULE_DIR/"
cp packaging/magisk/service.sh "$MODULE_DIR/"
cp packaging/magisk/customize.sh "$MODULE_DIR/"
cp packaging/magisk/uninstall.sh "$MODULE_DIR/"
cp packaging/magisk/corepolicy.conf "$MODULE_DIR/"
cp packaging/magisk/utensil.conf "$MODULE_DIR/"
cp packaging/magisk/gamelist.txt "$MODULE_DIR/"

cp "$RUST_TARGET_DIR/aarch64-linux-android/release/corepolicy" "$MODULE_DIR/bin/arm64-v8a/corepolicy"
cp "$RUST_TARGET_DIR/aarch64-linux-android/release/inoi_refresh_idle" "$MODULE_DIR/bin/arm64-v8a/inoi_refresh_idle"
cp "$RUST_TARGET_DIR/aarch64-linux-android/release/utensil-poker" "$MODULE_DIR/bin/arm64-v8a/utensil-poker"
cp "$RUST_TARGET_DIR/aarch64-linux-android/release/utensil-webui" "$MODULE_DIR/bin/arm64-v8a/utensil-webui"

cp "$RUST_TARGET_DIR/armv7-linux-androideabi/release/corepolicy" "$MODULE_DIR/bin/armeabi-v7a/corepolicy"
cp "$RUST_TARGET_DIR/armv7-linux-androideabi/release/inoi_refresh_idle" "$MODULE_DIR/bin/armeabi-v7a/inoi_refresh_idle"
cp "$RUST_TARGET_DIR/armv7-linux-androideabi/release/utensil-poker" "$MODULE_DIR/bin/armeabi-v7a/utensil-poker"
cp "$RUST_TARGET_DIR/armv7-linux-androideabi/release/utensil-webui" "$MODULE_DIR/bin/armeabi-v7a/utensil-webui"

cp "$RUST_TARGET_DIR/aarch64-linux-android/release/corepolicy" "$DIST_ROOT/binaries/corepolicy-aarch64-linux-android"
cp "$RUST_TARGET_DIR/aarch64-linux-android/release/inoi_refresh_idle" "$DIST_ROOT/binaries/inoi_refresh_idle-aarch64-linux-android"
cp "$RUST_TARGET_DIR/aarch64-linux-android/release/utensil-poker" "$DIST_ROOT/binaries/utensil-poker-aarch64-linux-android"
cp "$RUST_TARGET_DIR/aarch64-linux-android/release/utensil-webui" "$DIST_ROOT/binaries/utensil-webui-aarch64-linux-android"
cp "$RUST_TARGET_DIR/armv7-linux-androideabi/release/corepolicy" "$DIST_ROOT/binaries/corepolicy-armv7-linux-androideabi"
cp "$RUST_TARGET_DIR/armv7-linux-androideabi/release/inoi_refresh_idle" "$DIST_ROOT/binaries/inoi_refresh_idle-armv7-linux-androideabi"
cp "$RUST_TARGET_DIR/armv7-linux-androideabi/release/utensil-poker" "$DIST_ROOT/binaries/utensil-poker-armv7-linux-androideabi"
cp "$RUST_TARGET_DIR/armv7-linux-androideabi/release/utensil-webui" "$DIST_ROOT/binaries/utensil-webui-armv7-linux-androideabi"

chmod 0644 "$MODULE_DIR/module.prop" "$MODULE_DIR/corepolicy.conf" "$MODULE_DIR/utensil.conf" "$MODULE_DIR/gamelist.txt"
chmod 0755 "$MODULE_DIR/service.sh" "$MODULE_DIR/customize.sh" "$MODULE_DIR/uninstall.sh"
chmod 0755 "$MODULE_DIR/bin/arm64-v8a/corepolicy" "$MODULE_DIR/bin/arm64-v8a/inoi_refresh_idle"
chmod 0755 "$MODULE_DIR/bin/arm64-v8a/utensil-poker"
chmod 0755 "$MODULE_DIR/bin/arm64-v8a/utensil-webui"
chmod 0755 "$MODULE_DIR/bin/armeabi-v7a/corepolicy" "$MODULE_DIR/bin/armeabi-v7a/inoi_refresh_idle"
chmod 0755 "$MODULE_DIR/bin/armeabi-v7a/utensil-poker"
chmod 0755 "$MODULE_DIR/bin/armeabi-v7a/utensil-webui"
chmod 0755 "$DIST_ROOT"/binaries/*

rm -f "$DIST_ROOT/$ZIP_NAME" "$DIST_ROOT/$ZIP_NAME.sha256"

(
    cd "$MODULE_DIR"
    zip -qr "../../$ZIP_NAME" .
)

sha256sum "$DIST_ROOT/$ZIP_NAME" > "$DIST_ROOT/$ZIP_NAME.sha256"
sha256sum "$DIST_ROOT"/binaries/* > "$DIST_ROOT/binaries.sha256"

echo "Built $DIST_ROOT/$ZIP_NAME"
echo "Built standalone binaries in $DIST_ROOT/binaries"
