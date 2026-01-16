#!/bin/bash
set -e

echo ""
echo "  Building Sassy Browser"
echo "  ======================"
echo ""

# Check for Rust
if ! command -v cargo &> /dev/null; then
    echo "  [ERROR] Rust not found. Install from https://rustup.rs"
    exit 1
fi

# Build release
echo "  [1/3] Building release binary..."
cargo build --release

# Create dist folder
echo "  [2/3] Creating distribution folder..."
mkdir -p dist
cp target/release/sassy-browser dist/
cp README.md LICENSE dist/
cp -r config dist/
cp -r assets dist/
cp -r phone-app dist/

# Make executable
chmod +x dist/sassy-browser

# Done
echo "  [3/3] Done!"
echo ""
echo "  Output: dist/sassy-browser"
echo ""
echo "  To install:"
echo "    Option A: Copy dist/ to /opt/sassy-browser"
echo "    Option B: Add dist/ to your PATH"
echo ""
