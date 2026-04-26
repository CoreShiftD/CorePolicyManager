#!/bin/bash
set -e

# Change to project root
cd "$(dirname "$0")/.."
PROJECT_ROOT=$(pwd)
RUST_ROOT="$PROJECT_ROOT/rust"
ASSET_DIR="$PROJECT_ROOT/app/src/main/assets/coreshift"
PROFILES_ASSET="$ASSET_DIR/profiles_category.json"
BLACKLIST_ASSET="$ASSET_DIR/foreground_blacklist.json"
PROFILE_RULES_ASSET="$ASSET_DIR/profile_rules.json"

if [ ! -f "$PROFILES_ASSET" ]; then
    echo "Missing required asset: $PROFILES_ASSET" >&2
    exit 1
fi

if [ ! -f "$BLACKLIST_ASSET" ]; then
    echo "Missing required asset: $BLACKLIST_ASSET" >&2
    exit 1
fi

if [ ! -f "$PROFILE_RULES_ASSET" ]; then
    echo "Missing required asset: $PROFILE_RULES_ASSET" >&2
    exit 1
fi

echo "Ensuring Rust targets are available..."
rustup target add aarch64-linux-android armv7-linux-androideabi

cd "$RUST_ROOT"
CARGO_TARGET_DIR=$(cargo metadata --format-version 1 | grep -o '"target_directory":"[^"]*"' | head -n 1 | cut -d'"' -f4)

echo "Building release binary for arm64-v8a..."
cargo build --release --target aarch64-linux-android -j 1

echo "Building release binary for armeabi-v7a..."
cargo build --release --target armv7-linux-androideabi -j 1
cd "$PROJECT_ROOT"

echo "Preparing packaging directory..."
DIST_DIR="dist/magisk/CoreShiftPolicy"
rm -rf "$DIST_DIR"
mkdir -p "$DIST_DIR/system/bin"
mkdir -p "$DIST_DIR/bin/arm64-v8a"
mkdir -p "$DIST_DIR/bin/armeabi-v7a"

echo "Copying module files..."
cp packaging/magisk/module.prop "$DIST_DIR/"
cp packaging/magisk/service.sh "$DIST_DIR/"
cp packaging/magisk/customize.sh "$DIST_DIR/"
cp packaging/magisk/uninstall.sh "$DIST_DIR/"

echo "Copying default JSON assets..."
cp "$PROFILES_ASSET" "$DIST_DIR/profiles_category.json"
cp "$BLACKLIST_ASSET" "$DIST_DIR/foreground_blacklist.json"
cp "$PROFILE_RULES_ASSET" "$DIST_DIR/profile_rules.json"

echo "Copying binaries..."
TARGET_DIR=${CARGO_TARGET_DIR:-rust/target}
cp "$TARGET_DIR/aarch64-linux-android/release/corepolicy" "$DIST_DIR/bin/arm64-v8a/corepolicy"
cp "$TARGET_DIR/armv7-linux-androideabi/release/corepolicy" "$DIST_DIR/bin/armeabi-v7a/corepolicy"

echo "Setting permissions..."
chmod 0755 "$DIST_DIR/service.sh"
chmod 0755 "$DIST_DIR/customize.sh"
chmod 0755 "$DIST_DIR/uninstall.sh"
chmod 0755 "$DIST_DIR/bin/arm64-v8a/corepolicy"
chmod 0755 "$DIST_DIR/bin/armeabi-v7a/corepolicy"
chmod 0644 "$DIST_DIR/profiles_category.json"
chmod 0644 "$DIST_DIR/foreground_blacklist.json"
chmod 0644 "$DIST_DIR/profile_rules.json"

echo "Zipping module..."
mkdir -p dist
cd "$DIST_DIR"
rm -f "../../CoreShiftPolicy-v0.1.0-preview.2.zip"
zip -r "../../CoreShiftPolicy-v0.1.0-preview.2.zip" .
cd ../../..

echo "Done: dist/CoreShiftPolicy-v0.1.0-preview.2.zip"

echo "Package contents:"
unzip -l dist/CoreShiftPolicy-v0.1.0-preview.2.zip
