#!/usr/bin/env bash
set -euo pipefail
echo "Ensure Rust (rustup) + stable toolchain are installed: https://rustup.rs"
rustup default stable || true
echo "Formatting check"
cargo fmt --all -- --check
echo "Fast check"
cargo check --workspace --all-targets
echo "Release build"
cargo build --workspace --release
echo "Done — artifacts under target/release/"
