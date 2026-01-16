# Building the MSI Installer

## Prerequisites (Windows)
1. Install Rust: https://rustup.rs
2. Install WiX Toolset v3.11+: `winget install WixToolset.WixToolset`
3. Add WiX to PATH: `C:\Program Files (x86)\WiX Toolset v3.11\bin`

## Build Steps

```powershell
# 1. Build the release binary
cd sassy-browser
cargo build --release

# 2. Create installer directory structure
mkdir installer\pkg
copy target\release\sassy-browser.exe installer\pkg\
copy config\*.toml installer\pkg\
xcopy assets installer\pkg\assets\ /E

# 3. Build MSI
cd installer
candle sassy.wxs -out sassy.wixobj
light sassy.wixobj -out SassyBrowser-1.0.0-x64.msi -ext WixUIExtension

# 4. Done!
# Output: installer/SassyBrowser-1.0.0-x64.msi
```

## Files included:
- icon.ico ✓ (generated)
- LICENSE.rtf ✓ (generated)
- sassy.wxs ✓ (WiX config)
