# ════════════════════════════════════════════════════════════════════════════
# SASSY BROWSER v2.0.0 - Universal File Viewer & Browser
# Windows Build Script - Run as Administrator
# Pure Rust - No Chrome - No Google - No Paid Dependencies
# ════════════════════════════════════════════════════════════════════════════

$ErrorActionPreference = "Stop"
$Version = "2.0.0"

Write-Host ""
Write-Host "════════════════════════════════════════════════════════════════" -ForegroundColor Cyan
Write-Host "  SASSY BROWSER v$Version - Universal File Viewer & Browser" -ForegroundColor Cyan  
Write-Host "  Pure Rust | 200+ Formats | No Chrome | No Paid Dependencies" -ForegroundColor DarkCyan
Write-Host "════════════════════════════════════════════════════════════════" -ForegroundColor Cyan
Write-Host ""

# ═══════════════════════════════════════════════════════════════════════════
# DEPENDENCY CHECK
# ═══════════════════════════════════════════════════════════════════════════

# Check/Install Rust
Write-Host "[1/5] Checking Rust..." -ForegroundColor Yellow
if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
    Write-Host "  Installing Rust..." -ForegroundColor Yellow
    Invoke-WebRequest -Uri https://win.rustup.rs -OutFile rustup-init.exe
    .\rustup-init.exe -y --default-toolchain stable
    $env:PATH += ";$env:USERPROFILE\.cargo\bin"
    Remove-Item rustup-init.exe
    Write-Host "  Rust installed!" -ForegroundColor Green
} else {
    $rustVersion = (rustc --version) -replace "rustc ", ""
    Write-Host "  Rust $rustVersion found" -ForegroundColor Green
}

# Check WiX (optional)
$HasWix = $false
if (Get-Command candle.exe -ErrorAction SilentlyContinue) {
    $HasWix = $true
    Write-Host "  WiX Toolset found" -ForegroundColor Green
} else {
    Write-Host "  WiX Toolset not found (MSI build skipped)" -ForegroundColor DarkYellow
}

# ═══════════════════════════════════════════════════════════════════════════
# BUILD
# ═══════════════════════════════════════════════════════════════════════════

Write-Host ""
Write-Host "[2/5] Building release binary..." -ForegroundColor Yellow
$buildStart = Get-Date
cargo build --release
$buildTime = (Get-Date) - $buildStart
Write-Host "  Build completed in $([math]::Round($buildTime.TotalSeconds, 1))s" -ForegroundColor Green

# ═══════════════════════════════════════════════════════════════════════════
# PREPARE INSTALLER
# ═══════════════════════════════════════════════════════════════════════════

Write-Host ""
Write-Host "[3/5] Preparing installer files..." -ForegroundColor Yellow
$pkg = "installer\pkg"
New-Item -ItemType Directory -Force -Path $pkg | Out-Null
Copy-Item "target\release\sassy-browser.exe" "$pkg\" -Force
Copy-Item "config\*.toml" "$pkg\" -Force
Copy-Item -Recurse "assets" "$pkg\" -Force
Copy-Item "README.md" "$pkg\" -Force
Copy-Item "LICENSE" "$pkg\" -Force
Write-Host "  Package prepared" -ForegroundColor Green

# ═══════════════════════════════════════════════════════════════════════════
# CREATE PORTABLE ZIP
# ═══════════════════════════════════════════════════════════════════════════

Write-Host ""
Write-Host "[4/5] Creating portable ZIP..." -ForegroundColor Yellow
$zipPath = "installer\SassyBrowser-$Version-portable.zip"
if (Test-Path $zipPath) { Remove-Item $zipPath -Force }
Compress-Archive -Path "$pkg\*" -DestinationPath $zipPath
$zipSize = [math]::Round((Get-Item $zipPath).Length / 1MB, 2)
Write-Host "  Created $zipPath ($zipSize MB)" -ForegroundColor Green

# ═══════════════════════════════════════════════════════════════════════════
# BUILD MSI (if WiX available)
# ═══════════════════════════════════════════════════════════════════════════

if ($HasWix) {
    Write-Host ""
    Write-Host "[5/5] Building MSI installer..." -ForegroundColor Yellow
    Push-Location installer
    try {
        candle.exe sassy.wxs -out sassy.wixobj -dVersion=$Version
        light.exe sassy.wixobj -out "SassyBrowser-$Version-x64.msi" -ext WixUIExtension
        $msiSize = [math]::Round((Get-Item "SassyBrowser-$Version-x64.msi").Length / 1MB, 2)
        Write-Host "  Created SassyBrowser-$Version-x64.msi ($msiSize MB)" -ForegroundColor Green
    } catch {
        Write-Host "  MSI build failed: $_" -ForegroundColor Red
    }
    Pop-Location
} else {
    Write-Host ""
    Write-Host "[5/5] Skipping MSI (WiX not installed)" -ForegroundColor DarkYellow
}

# ═══════════════════════════════════════════════════════════════════════════
# SUMMARY
# ═══════════════════════════════════════════════════════════════════════════

Write-Host ""
Write-Host "════════════════════════════════════════════════════════════════" -ForegroundColor Green
Write-Host "  BUILD COMPLETE!" -ForegroundColor Green
Write-Host "════════════════════════════════════════════════════════════════" -ForegroundColor Green
Write-Host ""
Write-Host "  Outputs:" -ForegroundColor White
Write-Host "    target\release\sassy-browser.exe" -ForegroundColor Gray
Write-Host "    installer\SassyBrowser-$Version-portable.zip" -ForegroundColor Gray
if ($HasWix) {
    Write-Host "    installer\SassyBrowser-$Version-x64.msi" -ForegroundColor Gray
}
Write-Host ""
Write-Host "  Supported Formats: 200+" -ForegroundColor White
Write-Host "    Images    RAW, PSD, EXR, AVIF, HEIC, PNG, JPG, SVG..." -ForegroundColor DarkGray
Write-Host "    Documents PDF, DOCX, ODT, RTF, EPUB..." -ForegroundColor DarkGray
Write-Host "    Science   PDB, MOL, SDF, CIF (molecular structures)" -ForegroundColor DarkGray
Write-Host "    Code      200+ languages with syntax highlighting" -ForegroundColor DarkGray
Write-Host ""
Write-Host "  KILLS paid software:" -ForegroundColor Magenta
Write-Host "    Adobe Creative Cloud   `$504/year" -ForegroundColor DarkMagenta
Write-Host "    Microsoft 365          `$100/year" -ForegroundColor DarkMagenta
Write-Host "    ChemDraw               `$2,600/year" -ForegroundColor DarkMagenta
Write-Host ""
Write-Host "════════════════════════════════════════════════════════════════" -ForegroundColor Green
