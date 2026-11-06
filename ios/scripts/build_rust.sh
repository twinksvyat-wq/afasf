#!/bin/bash
# Builds the Rust core for iOS device + simulator and packages FightCore.xcframework.
set -euo pipefail
HERE="$(cd "$(dirname "$0")/.." && pwd)"
CORE="$HERE/../core"
OUT="$HERE/Frameworks"
export PATH="$HOME/.cargo/bin:$PATH"
# Xcode sets SDKROOT etc. for the app target; cargo must not inherit them.
unset SDKROOT IPHONEOS_DEPLOYMENT_TARGET 2>/dev/null || true
export IPHONEOS_DEPLOYMENT_TARGET=16.0

rustup target add aarch64-apple-ios aarch64-apple-ios-sim x86_64-apple-ios >/dev/null

cd "$CORE"
cargo build --release --lib --target aarch64-apple-ios
cargo build --release --lib --target aarch64-apple-ios-sim
cargo build --release --lib --target x86_64-apple-ios

mkdir -p "$CORE/target/universal-sim"
lipo -create \
  "$CORE/target/aarch64-apple-ios-sim/release/libfightcore.a" \
  "$CORE/target/x86_64-apple-ios/release/libfightcore.a" \
  -output "$CORE/target/universal-sim/libfightcore.a"

rm -rf "$OUT/FightCore.xcframework"
mkdir -p "$OUT"
xcodebuild -create-xcframework \
  -library "$CORE/target/aarch64-apple-ios/release/libfightcore.a" -headers "$CORE/include" \
  -library "$CORE/target/universal-sim/libfightcore.a" -headers "$CORE/include" \
  -output "$OUT/FightCore.xcframework"
echo "FightCore.xcframework ready"
