#!/bin/bash
# ════════════════════════════════════════════════════════════════════════════
# SASSY BROWSER v2.0.0 - Universal File Viewer & Browser
# macOS Build Script - Creates .app bundle and .dmg installer
# Pure Rust - No Chrome - No Google - No Paid Dependencies
# ════════════════════════════════════════════════════════════════════════════

set -e

VERSION="2.0.0"
APP_NAME="Sassy Browser"
BUNDLE_ID="com.sassyconsulting.sassy-browser"

echo ""
echo "════════════════════════════════════════════════════════════════"
echo "  SASSY BROWSER v${VERSION} - macOS Build"
echo "  Pure Rust | 200+ Formats | No Chrome | No Paid Dependencies"
echo "════════════════════════════════════════════════════════════════"
echo ""

# ═══════════════════════════════════════════════════════════════════════════
# DEPENDENCY CHECK
# ═══════════════════════════════════════════════════════════════════════════

echo "[1/6] Checking dependencies..."

if ! command -v cargo &> /dev/null; then
    echo "  ERROR: Rust not found. Install from https://rustup.rs"
    exit 1
fi
RUST_VERSION=$(rustc --version | sed 's/rustc //')
echo "  Rust ${RUST_VERSION} found"

# Detect architecture
ARCH=$(uname -m)
echo "  Architecture: ${ARCH}"

# ═══════════════════════════════════════════════════════════════════════════
# BUILD
# ═══════════════════════════════════════════════════════════════════════════

echo ""
echo "[2/6] Building release binary..."
BUILD_START=$(date +%s)

# Check if we can build universal binary
UNIVERSAL=false
if rustup target list --installed | grep -q "aarch64-apple-darwin" && \
   rustup target list --installed | grep -q "x86_64-apple-darwin"; then
    UNIVERSAL=true
    echo "  Building universal binary (Intel + Apple Silicon)..."
    cargo build --release --target aarch64-apple-darwin
    cargo build --release --target x86_64-apple-darwin
    mkdir -p target/universal-apple-darwin/release
    lipo -create \
        target/aarch64-apple-darwin/release/sassy-browser \
        target/x86_64-apple-darwin/release/sassy-browser \
        -output target/universal-apple-darwin/release/sassy-browser
    BINARY="target/universal-apple-darwin/release/sassy-browser"
    ARCH_LABEL="universal"
else
    echo "  Building for current architecture (${ARCH})..."
    echo "  TIP: For universal binary, install both targets:"
    echo "    rustup target add aarch64-apple-darwin x86_64-apple-darwin"
    cargo build --release
    BINARY="target/release/sassy-browser"
    ARCH_LABEL="${ARCH}"
fi

chmod +x "$BINARY"
BUILD_END=$(date +%s)
BUILD_TIME=$((BUILD_END - BUILD_START))
BINARY_SIZE=$(du -h "$BINARY" | cut -f1)
echo "  Build completed in ${BUILD_TIME}s (${BINARY_SIZE})"

# ═══════════════════════════════════════════════════════════════════════════
# CREATE APP BUNDLE
# ═══════════════════════════════════════════════════════════════════════════

echo ""
echo "[3/6] Creating app bundle..."

APP_DIR="${APP_NAME}.app/Contents"
rm -rf "${APP_NAME}.app"
mkdir -p "${APP_DIR}/MacOS"
mkdir -p "${APP_DIR}/Resources"

# Copy binary
cp "$BINARY" "${APP_DIR}/MacOS/sassy-browser"
chmod +x "${APP_DIR}/MacOS/sassy-browser"

# Copy resources
cp -r assets "${APP_DIR}/Resources/" 2>/dev/null || true
cp -r config "${APP_DIR}/Resources/" 2>/dev/null || true

# Create icon from PNG
if [ -f "assets/icons/android-chrome-512x512.png" ]; then
    echo "  Creating app icon..."
    mkdir -p SassyBrowser.iconset
    sips -z 16 16     "assets/icons/android-chrome-512x512.png" --out SassyBrowser.iconset/icon_16x16.png 2>/dev/null
    sips -z 32 32     "assets/icons/android-chrome-512x512.png" --out SassyBrowser.iconset/icon_16x16@2x.png 2>/dev/null
    sips -z 32 32     "assets/icons/android-chrome-512x512.png" --out SassyBrowser.iconset/icon_32x32.png 2>/dev/null
    sips -z 64 64     "assets/icons/android-chrome-512x512.png" --out SassyBrowser.iconset/icon_32x32@2x.png 2>/dev/null
    sips -z 128 128   "assets/icons/android-chrome-512x512.png" --out SassyBrowser.iconset/icon_128x128.png 2>/dev/null
    sips -z 256 256   "assets/icons/android-chrome-512x512.png" --out SassyBrowser.iconset/icon_128x128@2x.png 2>/dev/null
    sips -z 256 256   "assets/icons/android-chrome-512x512.png" --out SassyBrowser.iconset/icon_256x256.png 2>/dev/null
    sips -z 512 512   "assets/icons/android-chrome-512x512.png" --out SassyBrowser.iconset/icon_256x256@2x.png 2>/dev/null
    sips -z 512 512   "assets/icons/android-chrome-512x512.png" --out SassyBrowser.iconset/icon_512x512.png 2>/dev/null
    cp "assets/icons/android-chrome-512x512.png" SassyBrowser.iconset/icon_512x512@2x.png
    iconutil -c icns SassyBrowser.iconset -o "${APP_DIR}/Resources/AppIcon.icns"
    rm -rf SassyBrowser.iconset
    echo "  App icon created"
fi

# Create Info.plist
cat > "${APP_DIR}/Info.plist" << PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleName</key>
    <string>Sassy Browser</string>
    <key>CFBundleDisplayName</key>
    <string>Sassy Browser</string>
    <key>CFBundleIdentifier</key>
    <string>${BUNDLE_ID}</string>
    <key>CFBundleVersion</key>
    <string>${VERSION}</string>
    <key>CFBundleShortVersionString</key>
    <string>${VERSION}</string>
    <key>CFBundleExecutable</key>
    <string>sassy-browser</string>
    <key>CFBundleIconFile</key>
    <string>AppIcon</string>
    <key>CFBundlePackageType</key>
    <string>APPL</string>
    <key>CFBundleSignature</key>
    <string>SASY</string>
    <key>LSMinimumSystemVersion</key>
    <string>11.0</string>
    <key>NSHighResolutionCapable</key>
    <true/>
    <key>NSSupportsAutomaticGraphicsSwitching</key>
    <true/>
    <key>CFBundleDocumentTypes</key>
    <array>
        <dict>
            <key>CFBundleTypeName</key>
            <string>Web Document</string>
            <key>CFBundleTypeRole</key>
            <string>Viewer</string>
            <key>LSItemContentTypes</key>
            <array>
                <string>public.html</string>
                <string>public.url</string>
            </array>
        </dict>
        <dict>
            <key>CFBundleTypeName</key>
            <string>Document</string>
            <key>CFBundleTypeRole</key>
            <string>Viewer</string>
            <key>LSItemContentTypes</key>
            <array>
                <string>public.data</string>
                <string>public.content</string>
            </array>
        </dict>
    </array>
    <key>CFBundleURLTypes</key>
    <array>
        <dict>
            <key>CFBundleURLName</key>
            <string>Web URL</string>
            <key>CFBundleURLSchemes</key>
            <array>
                <string>http</string>
                <string>https</string>
            </array>
        </dict>
    </array>
</dict>
</plist>
PLIST

echo "  App bundle created: ${APP_NAME}.app"

# ═══════════════════════════════════════════════════════════════════════════
# CODE SIGNING (optional)
# ═══════════════════════════════════════════════════════════════════════════

echo ""
echo "[4/6] Code signing..."

if security find-identity -v -p codesigning 2>/dev/null | grep -q "Developer ID"; then
    IDENTITY=$(security find-identity -v -p codesigning | grep "Developer ID" | head -1 | awk -F'"' '{print $2}')
    echo "  Signing with: ${IDENTITY}"
    codesign --force --deep --sign "${IDENTITY}" "${APP_NAME}.app"
    echo "  App bundle signed"
else
    echo "  No Developer ID certificate found (skipping)"
    echo "  TIP: For distribution, sign with: codesign --force --deep --sign 'Developer ID' '${APP_NAME}.app'"
    # Ad-hoc sign for local use
    codesign --force --deep --sign - "${APP_NAME}.app" 2>/dev/null || true
    echo "  Ad-hoc signed for local use"
fi

# ═══════════════════════════════════════════════════════════════════════════
# CREATE DMG
# ═══════════════════════════════════════════════════════════════════════════

echo ""
echo "[5/6] Creating DMG installer..."

DMG_NAME="SassyBrowser-${VERSION}-macos-${ARCH_LABEL}.dmg"
VOLUME_NAME="Sassy Browser ${VERSION}"

# Clean up previous
rm -f "$DMG_NAME"
rm -rf dmg-staging

# Create staging directory
mkdir -p dmg-staging
cp -r "${APP_NAME}.app" dmg-staging/
ln -s /Applications dmg-staging/Applications

# Create DMG
hdiutil create -volname "$VOLUME_NAME" \
    -srcfolder dmg-staging \
    -ov -format UDZO \
    "$DMG_NAME"

rm -rf dmg-staging

DMG_SIZE=$(du -h "$DMG_NAME" | cut -f1)
echo "  Created ${DMG_NAME} (${DMG_SIZE})"

# ═══════════════════════════════════════════════════════════════════════════
# SUMMARY
# ═══════════════════════════════════════════════════════════════════════════

echo ""
echo "[6/6] Creating portable tar.gz..."
TAR_NAME="SassyBrowser-${VERSION}-macos-${ARCH_LABEL}.tar.gz"
tar -czf "$TAR_NAME" "${APP_NAME}.app"
TAR_SIZE=$(du -h "$TAR_NAME" | cut -f1)
echo "  Created ${TAR_NAME} (${TAR_SIZE})"

echo ""
echo "════════════════════════════════════════════════════════════════"
echo "  BUILD COMPLETE!"
echo "════════════════════════════════════════════════════════════════"
echo ""
echo "  Outputs:"
echo "    ${DMG_NAME}  (${DMG_SIZE})"
echo "    ${TAR_NAME}  (${TAR_SIZE})"
echo "    ${APP_NAME}.app"
echo ""
echo "  Architecture: ${ARCH_LABEL}"
echo "  Build time: ${BUILD_TIME}s"
echo ""
echo "  To install:"
echo "    1. Open ${DMG_NAME}"
echo "    2. Drag 'Sassy Browser' to Applications"
echo "    3. Launch from Applications or Spotlight"
echo ""
echo "  Supported Formats: 200+"
echo "    Images    RAW, PSD, EXR, AVIF, HEIC, PNG, JPG, SVG..."
echo "    Documents PDF, DOCX, ODT, RTF, EPUB..."
echo "    Science   PDB, MOL, SDF, CIF (molecular structures)"
echo "    Code      200+ languages with syntax highlighting"
echo ""
echo "════════════════════════════════════════════════════════════════"
