@echo off
echo ════════════════════════════════════════════════════════════════
echo   SASSY BROWSER v2.0.0 - Universal File Viewer ^& Browser
echo   Pure Rust - No Chrome - No Google - No Paid Dependencies
echo ════════════════════════════════════════════════════════════════
echo.

REM Check Rust
where cargo >nul 2>nul
if %ERRORLEVEL% neq 0 (
    echo ERROR: Rust not found. Install from https://rustup.rs
    pause
    exit /b 1
)

REM Build release
echo [1/4] Building release binary...
cargo build --release
if errorlevel 1 goto :error

REM Prep installer files
echo [2/4] Preparing installer files...
mkdir installer\pkg 2>nul
copy /Y target\release\sassy-browser.exe installer\pkg\
copy /Y config\*.toml installer\pkg\
xcopy /Y /E /I assets installer\pkg\assets\

REM Check for WiX Toolset
where candle >nul 2>nul
if %ERRORLEVEL% neq 0 (
    echo.
    echo NOTE: WiX Toolset not found. Skipping MSI creation.
    echo       Install WiX 3.x from https://wixtoolset.org/ for MSI builds.
    echo.
    echo BUILD COMPLETE!
    echo Binary: target\release\sassy-browser.exe
    goto :eof
)

REM Build MSI
echo [3/4] Building MSI installer...
cd installer
candle sassy.wxs -out sassy.wixobj -dVersion=2.0.0
if errorlevel 1 goto :error
light sassy.wixobj -out SassyBrowser-2.0.0-x64.msi -ext WixUIExtension
if errorlevel 1 goto :error
cd ..

echo [4/4] Creating portable ZIP...
powershell -Command "Compress-Archive -Path 'installer\pkg\*' -DestinationPath 'installer\SassyBrowser-2.0.0-portable.zip' -Force"

echo.
echo ════════════════════════════════════════════════════════════════
echo   BUILD SUCCESS!
echo ════════════════════════════════════════════════════════════════
echo   Outputs:
echo     installer\SassyBrowser-2.0.0-x64.msi
echo     installer\SassyBrowser-2.0.0-portable.zip
echo     target\release\sassy-browser.exe
echo.
echo   Supported formats: 200+ file types
echo   Kills: Adobe CC ($504/yr), MS 365 ($100/yr), ChemDraw ($2600/yr)
echo ════════════════════════════════════════════════════════════════
goto :eof

:error
echo.
echo ════════════════════════════════════════════════════════════════
echo   BUILD FAILED
echo ════════════════════════════════════════════════════════════════
exit /b 1
