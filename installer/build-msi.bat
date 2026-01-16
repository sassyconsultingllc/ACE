@echo off
echo Building MSI installer...
echo.

REM Requires WiX Toolset installed
REM Download from: https://wixtoolset.org/releases/

REM Check for WiX
where candle >nul 2>nul
if %ERRORLEVEL% neq 0 (
    echo [ERROR] WiX Toolset not found in PATH
    echo Download from: https://wixtoolset.org/releases/
    exit /b 1
)

REM Build the browser first
echo [1/4] Building release binary...
cd ..
cargo build --release
if %ERRORLEVEL% neq 0 exit /b 1

REM Convert SVG to ICO if needed
echo [2/4] Preparing assets...
if not exist assets\icons\icon.ico (
    echo [WARN] icon.ico not found - using placeholder
    REM You'd use ImageMagick or similar to convert SVG to ICO
)

REM Compile WiX
echo [3/4] Compiling installer...
cd installer
candle -nologo sassy.wxs -out sassy.wixobj
if %ERRORLEVEL% neq 0 exit /b 1

REM Link to MSI
echo [4/4] Linking MSI...
light -nologo -ext WixUIExtension sassy.wixobj -out sassy-browser-0.4.0.msi
if %ERRORLEVEL% neq 0 exit /b 1

echo.
echo [OK] Created: sassy-browser-0.4.0.msi
echo.
