#!/bin/bash
# One-shot setup on a Mac: Rust iOS targets -> FightCore.xcframework -> Xcode project.
set -euo pipefail
cd "$(dirname "$0")"
command -v cargo >/dev/null || { echo "Install Rust: https://rustup.rs"; exit 1; }
command -v xcodegen >/dev/null || { echo "Install XcodeGen: brew install xcodegen"; exit 1; }
./scripts/build_rust.sh
xcodegen generate
echo "Done. Open WhiteoutFC.xcodeproj, pick your Team in Signing & Capabilities, run on iPhone."
open WhiteoutFC.xcodeproj || true
