@echo off
chcp 65001 >nul
title AlgoDeck Installer (Windows)
echo ==============================================================================
echo    ⚡ AlgoDeck (Kolo MAP) - Instalator dla Windows
echo ==============================================================================
echo Uruchamianie instalatora PowerShell...
echo.

powershell -NoProfile -ExecutionPolicy Bypass -File "%~dp0install_win.ps1"

if %ERRORLEVEL% NEQ 0 (
    echo.
    echo [BLAD] Instalator napotkal problem (kod bledu: %ERRORLEVEL%).
    pause
    exit /b %ERRORLEVEL%
)

echo.
echo Nacisnij dowolny klawisz, aby zamknac to okno...
pause >nul
