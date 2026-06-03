@echo off
setlocal

echo [proxysss] building release binary
cargo build --release
if errorlevel 1 exit /b %ERRORLEVEL%

echo [proxysss] build complete: target\release\proxysss.exe
exit /b 0
