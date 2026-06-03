@echo off
setlocal

set "CONFIG=%~1"
if "%CONFIG%"=="" set "CONFIG=proxysss.yaml"

echo [proxysss] running with config: %CONFIG%
cargo run --release -- run --config "%CONFIG%"
exit /b %ERRORLEVEL%
