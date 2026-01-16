@echo off
echo.
echo  =======================================
echo  Building Sassy Browser v1.0.0
echo  =======================================
echo.

:: Check for Rust
where cargo >nul 2>nul
if %ERRORLEVEL% neq 0 (
    echo ERROR: Rust not found. Install from https://rustup.rs
    pause
    exit /b 1
)

:: Build release
echo Building release binary...
cargo build --release
if %ERRORLEVEL% neq 0 (
    echo ERROR: Build failed
    pause
    exit /b 1
)

:: Create output directory
if not exist "C:\SassyBrowser" mkdir "C:\SassyBrowser"

:: Copy files
echo.
echo Copying files to C:\SassyBrowser...
copy /Y "target\release\sassy-browser.exe" "C:\SassyBrowser\"
copy /Y "README.md" "C:\SassyBrowser\"
copy /Y "LICENSE" "C:\SassyBrowser\"
xcopy /Y /E "config" "C:\SassyBrowser\config\"
xcopy /Y /E "assets" "C:\SassyBrowser\assets\"
xcopy /Y /E "phone-app" "C:\SassyBrowser\phone-app\"

:: Create run script
echo @echo off > "C:\SassyBrowser\run.bat"
echo cd /d "%%~dp0" >> "C:\SassyBrowser\run.bat"
echo sassy-browser.exe %%* >> "C:\SassyBrowser\run.bat"

echo.
echo  =======================================
echo  Build complete!
echo  =======================================
echo.
echo  Run: C:\SassyBrowser\sassy-browser.exe
echo  Or:  C:\SassyBrowser\run.bat [URL]
echo.
pause
