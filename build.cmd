@echo off
rem Ice Audio Manager: one-click build.
rem
rem   build            release build of the app (double-click does this)
rem   build debug      fast debug build
rem   build check      everything CI runs: UI check/test/build, fmt, clippy -D warnings, all tests
rem   build run        release build, then start the app
rem
rem The first release build takes several minutes at the last step (full link-time optimisation on one core).

setlocal EnableExtensions
cd /d "%~dp0"
set "MODE=%~1"
if "%MODE%"=="" set "MODE=release"
if /i not "%MODE%"=="release" if /i not "%MODE%"=="debug" if /i not "%MODE%"=="check" if /i not "%MODE%"=="run" (
    echo Unknown option "%MODE%". Use: build [release^|debug^|check^|run]
    goto :fail
)

rem Terminals opened before Rust was installed don't have cargo on PATH yet.
where cargo >nul 2>nul || set "PATH=%USERPROFILE%\.cargo\bin;%PATH%"
where cargo >nul 2>nul || (echo cargo not found. Install Rust from https://rustup.rs and try again. & goto :fail)
where npm >nul 2>nul || (echo npm not found. Install Node.js 22 and try again. & goto :fail)

echo.
echo === [1/3] UI packages
if exist "app\ui\node_modules\.package-lock.json" (
    echo already installed
) else (
    call npm --prefix app/ui ci --no-audit --no-fund || goto :fail
)

echo.
echo === [2/3] UI build
call npm --prefix app/ui run build || goto :fail

if /i "%MODE%"=="check" goto :check

set "PROFILE=release"
set "CARGO_FLAGS=--release"
if /i "%MODE%"=="debug" (
    set "PROFILE=debug"
    set "CARGO_FLAGS="
)
set "EXE=%CD%\target\%PROFILE%\ice-audio-manager.exe"

rem A running copy of this exact build locks the file and the link fails. Close only that copy.
powershell -NoProfile -Command "Get-Process ice-audio-manager -ErrorAction SilentlyContinue | Where-Object { $_.Path -eq $env:EXE } | ForEach-Object { Write-Host ('closing the running copy (pid ' + $_.Id + ')'); $_ | Stop-Process -Force }"

echo.
echo === [3/3] App (%PROFILE%)
cargo build %CARGO_FLAGS% -p ice-audio-manager || goto :fail

echo.
echo Built: %EXE%
if /i "%MODE%"=="run" (
    echo Starting it. It lives in the tray; a second launch just exits.
    start "" "%EXE%"
)
goto :done

:check
echo.
echo === UI type-check and tests
call npm --prefix app/ui run check || goto :fail
call npm --prefix app/ui test || goto :fail
echo.
echo === cargo fmt --check
cargo fmt --all -- --check || goto :fail
echo.
echo === cargo clippy -D warnings
cargo clippy --workspace --all-targets -- -D warnings || goto :fail
echo.
echo === cargo test
cargo test --workspace || goto :fail
cargo test -p ice-core --all-features || goto :fail
echo.
echo All checks passed.
goto :done

:fail
echo.
echo *** BUILD FAILED (see the messages above) ***
call :pause_if_clicked
endlocal
exit /b 1

:done
call :pause_if_clicked
endlocal
exit /b 0

rem Keep the window open when started by double-click (cmd /c), not when run from a terminal.
:pause_if_clicked
echo %CMDCMDLINE% | find /i "/c" >nul && pause
exit /b 0
