# Sassy Browser - Windows Build Script
# Run in PowerShell as Administrator

$ErrorActionPreference = "Stop"

Write-Host "========================================" -ForegroundColor Cyan
Write-Host "  Sassy Browser v1.0.0 - Windows Build" -ForegroundColor Cyan  
Write-Host "========================================" -ForegroundColor Cyan

# Check Rust
if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
    Write-Host "Installing Rust..." -ForegroundColor Yellow
    Invoke-WebRequest -Uri https://win.rustup.rs -OutFile rustup-init.exe
    .\rustup-init.exe -y
    $env:PATH += ";$env:USERPROFILE\.cargo\bin"
    Remove-Item rustup-init.exe
}

# Check WiX
if (-not (Get-Command candle.exe -ErrorAction SilentlyContinue)) {
    Write-Host "Installing WiX Toolset..." -ForegroundColor Yellow
    winget install WixToolset.WixToolset --accept-package-agreements --accept-source-agreements
    $env:PATH += ";C:\Program Files (x86)\WiX Toolset v3.14\bin"
}

# Build release
Write-Host "`nBuilding release binary..." -ForegroundColor Green
cargo build --release

# Prep installer files
Write-Host "`nPreparing installer..." -ForegroundColor Green
$pkg = "installer\pkg"
New-Item -ItemType Directory -Force -Path $pkg | Out-Null
Copy-Item "target\release\sassy-browser.exe" "$pkg\"
Copy-Item "config\*.toml" "$pkg\"
Copy-Item -Recurse "assets" "$pkg\"

# Build MSI
Write-Host "`nBuilding MSI..." -ForegroundColor Green
Set-Location installer
candle.exe sassy.wxs -out sassy.wixobj
light.exe sassy.wixobj -out SassyBrowser-1.0.0-x64.msi -ext WixUIExtension

Write-Host "`n========================================" -ForegroundColor Green
Write-Host "  SUCCESS! MSI created:" -ForegroundColor Green
Write-Host "  installer\SassyBrowser-1.0.0-x64.msi" -ForegroundColor White
Write-Host "========================================" -ForegroundColor Green
