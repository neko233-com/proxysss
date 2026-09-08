@echo off
setlocal
cd /d "%~dp0"
set "TEMP=%~dp0.tmp\toolchain"
set "TMP=%TEMP%"
set "CARGO_HOME=%~dp0.cache\cargo"
set "CARGO_TARGET_DIR=%~dp0target"
if not exist "%TEMP%" mkdir "%TEMP%"

set PROFILE=%1
if "%PROFILE%"=="" set PROFILE=release-fast
if "%PROFILE%"=="--release" set PROFILE=release

echo [proxysss] building %PROFILE% binary
set CARGO_INCREMENTAL=1
cargo build --profile %PROFILE%
if errorlevel 1 exit /b %ERRORLEVEL%

echo [proxysss] build complete: target\%PROFILE%\proxysss.exe
exit /b 0
