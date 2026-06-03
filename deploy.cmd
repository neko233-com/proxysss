@echo off
setlocal

set "CONFIG=%~1"
if "%CONFIG%"=="" set "CONFIG=proxysss.yaml"

echo [proxysss] checking config: %CONFIG%
cargo run --release -- check-config --config "%CONFIG%"
if errorlevel 1 exit /b %ERRORLEVEL%

echo [proxysss] building release
cargo build --release
if errorlevel 1 exit /b %ERRORLEVEL%

echo [proxysss] installing service
cargo run --release -- service install --config "%CONFIG%"
if errorlevel 1 exit /b %ERRORLEVEL%

echo [proxysss] starting service
cargo run --release -- service start
exit /b %ERRORLEVEL%
