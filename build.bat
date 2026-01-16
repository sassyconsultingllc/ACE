@echo off
echo.
echo  ═══════════════════════════════════════════════════════════════
echo  SASSY BROWSER v2.0.0 - Quick Build
echo  Universal File Viewer ^& Browser - Pure Rust
echo  ═══════════════════════════════════════════════════════════════
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
echo.
cargo build --release
if %ERRORLEVEL% neq 0 (
    echo.
    echo ERROR: Build failed!
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
xcopy /Y /E /I "config" "C:\SassyBrowser\config\"
xcopy /Y /E /I "assets" "C:\SassyBrowser\assets\"
xcopy /Y /E /I "phone-app" "C:\SassyBrowser\phone-app\"

:: Create run scripts
echo @echo off > "C:\SassyBrowser\run.bat"
echo cd /d "%%~dp0" >> "C:\SassyBrowser\run.bat"
echo sassy-browser.exe %%* >> "C:\SassyBrowser\run.bat"

echo @echo off > "C:\SassyBrowser\open-file.bat"
echo cd /d "%%~dp0" >> "C:\SassyBrowser\open-file.bat"
echo if "%%~1"=="" ( >> "C:\SassyBrowser\open-file.bat"
echo   echo Usage: open-file.bat [filepath] >> "C:\SassyBrowser\open-file.bat"
echo   pause >> "C:\SassyBrowser\open-file.bat"
echo   exit /b 1 >> "C:\SassyBrowser\open-file.bat"
echo ) >> "C:\SassyBrowser\open-file.bat"
echo sassy-browser.exe "%%~1" >> "C:\SassyBrowser\open-file.bat"

echo.
echo  ═══════════════════════════════════════════════════════════════
echo  BUILD COMPLETE!
echo  ═══════════════════════════════════════════════════════════════
echo.
echo  Run:   C:\SassyBrowser\sassy-browser.exe
echo  Or:    C:\SassyBrowser\run.bat [URL]
echo  Open:  C:\SassyBrowser\open-file.bat [file.pdb/pdf/docx/...]
echo.
echo  Supported formats: 200+
echo    Images: PNG, JPG, RAW, PSD, SVG, AVIF, HEIC, EXR...
echo    Docs:   PDF, DOCX, ODT, XLSX, CSV...
echo    Code:   200+ languages with syntax highlighting
echo    Chem:   PDB, MOL, SDF, CIF (molecular structures)
echo    3D:     OBJ, STL, GLTF, PLY
echo    Audio:  MP3, FLAC, WAV, OGG
echo    Video:  MP4, MKV, WebM (metadata)
echo.
pause
