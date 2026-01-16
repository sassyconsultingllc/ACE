# Installing Sassy Browser

## Requirements

- Rust 1.70+ (https://rustup.rs)
- Tailscale (optional, for phone sync)

## Quick Build

### Windows
```batch
build.bat
```

### Linux/Mac
```bash
./build.sh
```

Output goes to `dist/` folder.

## Manual Build

```bash
cargo build --release
```

Binary: `target/release/sassy-browser` (or .exe on Windows)

## Installation Options

### Option A: Portable (Recommended)

Just copy the `dist/` folder anywhere. No installation needed.
Run `sassy-browser` directly.

### Option B: System Install (Linux)

```bash
sudo cp dist/sassy-browser /usr/local/bin/
sudo mkdir -p /usr/share/sassy-browser
sudo cp -r dist/assets dist/config dist/phone-app /usr/share/sassy-browser/
```

### Option C: User Install (Linux)

```bash
mkdir -p ~/.local/bin
cp dist/sassy-browser ~/.local/bin/
# Add ~/.local/bin to PATH if needed
```

## Data Locations

| OS | Config | Data |
|----|--------|------|
| Windows | `%APPDATA%\SassyBrowser` | `%LOCALAPPDATA%\SassyBrowser` |
| Linux | `~/.config/SassyBrowser` | `~/.local/share/SassyBrowser` |
| Mac | `~/Library/Application Support/SassyBrowser` | Same |

## First Run

1. Run `sassy-browser`
2. Enter your name to create a profile
3. You become the admin (can add other users)
4. If Tailscale is detected, phone sync is enabled

## Reset

To start fresh:
```bash
sassy-browser --reset
```

## Uninstall

1. Delete the binary
2. Delete config folder (see table above)
3. Delete data folder (see table above)

## Phone Sync Setup

1. Install Tailscale on both desktop and phone
2. Sign in with same account
3. Run browser - it shows your Tailscale hostname
4. Open phone app, enter that hostname
5. Pick your user profile
