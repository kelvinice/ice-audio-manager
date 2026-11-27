@echo off
rem Pandora: one-click build (prototype).
rem
rem   build            release build
rem   build debug      fast debug build
rem   build check      UI type-check/build + Rust tests & clippy
rem   build run        debug build, then start Pandora

setlocal EnableExtensions
cd /d "%~dp0"
set "MODE=%~1"
if "%MODE%"=="" set "MODE=release"
if /i not "%MODE%"=="release" if /i not "%MODE%"=="debug" if /i not "%MODE%"=="check" if /i not "%MODE%"=="run" (
    echo Unknown option "%MODE%". Use: build [release^|debug^|check^|run]
    goto :fail
)

where cargo >nul 2>nul || set "PATH=%USERPROFILE%\.cargo\bin;%PATH%"
where cargo >nul 2>nul || (echo cargo not found. Install Rust from https://rustup.rs and try again. & goto :fail)
where npm >nul 2>nul || (echo npm not found. Install Node.js 22 and try again. & goto :fail)

echo.
echo === [1/3] Pandora UI packages
rem Always run: about a second when nothing changed, and it picks up packages added since the last build.
call npm --prefix ui install --no-audit --no-fund || goto :fail

echo.
echo === [2/3] Pandora UI build
call npm --prefix ui run build || goto :fail

if /i "%MODE%"=="check" goto :check

set "PROFILE=release"
set "CARGO_FLAGS=--release"
if /i "%MODE%"=="debug" (
    set "PROFILE=debug"
    set "CARGO_FLAGS="
)
if /i "%MODE%"=="run" (
    set "PROFILE=debug"
    set "CARGO_FLAGS="
)
set "EXE=%CD%\src-tauri\target\%PROFILE%\pandora.exe"

powershell -NoProfile -Command "Get-Process pandora -ErrorAction SilentlyContinue | Where-Object { $_.Path -eq $env:EXE } | Stop-Process -Force"

echo.
echo === [3/3] Pandora App (%PROFILE%)
cargo build --manifest-path src-tauri\Cargo.toml %CARGO_FLAGS% || goto :fail

echo.
echo Built: %EXE%
if /i "%MODE%"=="run" (
    start "" "%EXE%"
)
goto :done

:check
echo.
echo === Pandora UI check
call npm --prefix ui run check || goto :fail
echo.
echo === Pandora Rust test
cargo test --manifest-path src-tauri\Cargo.toml || goto :fail
echo.
echo All Pandora checks passed.
goto :done

:fail
echo.
echo *** PANDORA BUILD FAILED ***
endlocal
exit /b 1

:done
endlocal
exit /b 0
