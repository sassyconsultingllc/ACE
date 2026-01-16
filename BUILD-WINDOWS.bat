@echo off
echo ========================================
echo   Sassy Browser v1.0.0 - Windows Build
echo ========================================
echo.

REM Build
echo Building release...
cargo build --release
if errorlevel 1 goto :error

REM Prep
echo Preparing installer files...
mkdir installer\pkg 2>nul
copy target\release\sassy-browser.exe installer\pkg\
copy config\*.toml installer\pkg\
xcopy assets installer\pkg\assets\ /E /I /Y

REM MSI
echo Building MSI...
cd installer
candle sassy.wxs -out sassy.wixobj
if errorlevel 1 goto :error
light sassy.wixobj -out SassyBrowser-1.0.0-x64.msi -ext WixUIExtension
if errorlevel 1 goto :error

echo.
echo ========================================
echo   SUCCESS! 
echo   installer\SassyBrowser-1.0.0-x64.msi
echo ========================================
goto :eof

:error
echo BUILD FAILED
exit /b 1
