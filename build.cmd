@echo off
rem Loads the VS 2019 Build Tools C++ toolchain when present (some machines have a newer VS without the full C++ libs).
rem Without it, cargo uses whatever MSVC toolchain it finds on its own.
set "VCVARS=C:\Program Files (x86)\Microsoft Visual Studio\2019\BuildTools\VC\Auxiliary\Build\vcvars64.bat"
if exist "%VCVARS%" call "%VCVARS%" >nul
set "PATH=%USERPROFILE%\.cargo\bin;%PATH%"
cd /d "%~dp0"
if not exist node_modules call npm install --no-fund --no-audit
if "%1"=="dev" (
    npm run dev
) else if "%1"=="check" (
    cd src-tauri && cargo build
) else (
    npm run build
)
