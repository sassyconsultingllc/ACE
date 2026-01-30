#!/bin/bash
# Sassy Browser Demo - Shell Script Sample
# Build and deployment script

set -e

VERSION="2.0.0"
BUILD_DIR="./target/release"
DIST_DIR="./dist"

echo "Building Sassy Browser v$VERSION..."

# Clean previous builds
rm -rf "$DIST_DIR"
mkdir -p "$DIST_DIR"

# Build release binary
cargo build --release

# Copy binary
cp "$BUILD_DIR/sassy-browser" "$DIST_DIR/"

# Copy assets
cp -r ./assets "$DIST_DIR/"

# Create archive
tar -czvf "sassy-browser-$VERSION-linux-x64.tar.gz" -C "$DIST_DIR" .

echo "Build complete!"
echo "Output: sassy-browser-$VERSION-linux-x64.tar.gz"
