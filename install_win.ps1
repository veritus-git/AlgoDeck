# ==============================================================================
# AlgoDeck ⚡ Koło MAP - Oficjalny Instalator dla Windows (PowerShell)
# Instalacja One-Liner:
# irm https://raw.githubusercontent.com/veritus-git/AlgoDeck/main/install_win.ps1 | iex
# ==============================================================================

$ErrorActionPreference = "Stop"

Write-Host ""
Write-Host "==============================================================================" -ForegroundColor Cyan
Write-Host "       ⚡ AlgoDeck (Koło MAP) — Oficjalny Instalator dla Windows             " -ForegroundColor Cyan
Write-Host "==============================================================================" -ForegroundColor Cyan
Write-Host ""

# 1. Ścieżki docelowe
$InstallDir = "$env:USERPROFILE\.algodeck"
$BinDir = "$InstallDir\bin"
$VenvDir = "$InstallDir\venv"

Write-Host "[1/6] Przygotowywanie katalogów roboczych ($InstallDir)..." -ForegroundColor Cyan
New-Item -ItemType Directory -Force -Path $InstallDir | Out-Null
New-Item -ItemType Directory -Force -Path $BinDir | Out-Null

# 2. Sprawdzenie Pythona 3
Write-Host "`n[2/6] Sprawdzanie środowiska Python 3..." -ForegroundColor Cyan
$PythonPath = Get-Command python -ErrorAction SilentlyContinue | Select-Object -ExpandProperty Source -ErrorAction SilentlyContinue
if (-not $PythonPath) {
    Write-Host "Python nie został wykryty. Próba instalacji przez winget..." -ForegroundColor Yellow
    winget install -e --id Python.Python.3.12 --accept-package-agreements --accept-source-agreements --silent
    $PythonPath = "python"
}
Write-Host "✓ Wykryto Python: $PythonPath" -ForegroundColor Green

# 3. Sprawdzanie kompilatora C++ (G++ / Clang)
Write-Host "`n[3/6] Sprawdzanie kompilatora C++ (MinGW G++ / Clang)..." -ForegroundColor Cyan
$GppPath = Get-Command g++ -ErrorAction SilentlyContinue
if (-not $GppPath) {
    Write-Host "Kompilator g++ nie został znaleziony w PATH." -ForegroundColor Yellow
    Write-Host "Wskazówka: Zainstaluj MinGW-w64 (np. MSYS2 lub w64devkit) dla pełnego wsparcia C++20." -ForegroundColor Yellow
} else {
    Write-Host "✓ Wykryto kompilator g++." -ForegroundColor Green
}

# 4. Sprawdzanie Visual Studio Code
Write-Host "`n[4/6] Sprawdzanie Visual Studio Code..." -ForegroundColor Cyan
$CodeCmd = Get-Command code -ErrorAction SilentlyContinue
if ($CodeCmd) {
    Write-Host "✓ Visual Studio Code zainstalowany." -ForegroundColor Green
    & code --install-extension ms-vscode.cpptools --force | Out-Null
} else {
    Write-Host "Wskazówka: Pobierz Visual Studio Code z https://code.visualstudio.com/" -ForegroundColor Yellow
}

# 5. Pobranie plików AlgoDeck
Write-Host "`n[5/6] Pobieranie repozytorium AlgoDeck..." -ForegroundColor Cyan
if (Test-Path "$PSScriptRoot\backend\server.py") {
    Copy-Item -Recurse -Force "$PSScriptRoot\*" "$InstallDir\"
} else {
    $ZipUrl = "https://github.com/veritus-git/AlgoDeck/archive/refs/heads/main.zip"
    $ZipFile = "$env:TEMP\algodeck_main.zip"
    Invoke-WebRequest -Uri $ZipUrl -OutFile $ZipFile
    Expand-Archive -Path $ZipFile -DestinationPath "$env:TEMP\algodeck_extracted" -Force
    Copy-Item -Recurse -Force "$env:TEMP\algodeck_extracted\AlgoDeck-main\*" "$InstallDir\"
    Remove-Item -Force $ZipFile
    Remove-Item -Recurse -Force "$env:TEMP\algodeck_extracted"
}

# Konfiguracja venv
if (-not (Test-Path "$VenvDir")) {
    & python -m venv "$VenvDir"
}
$PipExe = "$VenvDir\Scripts\pip.exe"
$PythonVenv = "$VenvDir\Scripts\python.exe"

& $PipExe install --upgrade pip --quiet
& $PipExe install -r "$InstallDir\requirements.txt" --quiet
& $PipExe install websockets --quiet

# 6. Oficjalny plugin Elgato Stream Deck na Windows
Write-Host "`n[6/6] Konfiguracja Elgato Stream Deck (Oficjalny plugin Windows)..." -ForegroundColor Cyan
$ElgatoPluginsDir = "$env:APPDATA\Elgato\StreamDeck\Plugins"
$PluginDest = "$ElgatoPluginsDir\com.algodeck.streamdeck.sdPlugin"

if (Test-Path "$InstallDir\streamdeck_plugin\com.algodeck.streamdeck.sdPlugin") {
    New-Item -ItemType Directory -Force -Path $ElgatoPluginsDir | Out-Null
    Copy-Item -Recurse -Force "$InstallDir\streamdeck_plugin\com.algodeck.streamdeck.sdPlugin" "$PluginDest"
    Write-Host "✓ Zainstalowano plugin Elgato Stream Deck w: $PluginDest" -ForegroundColor Green
}

# 6b. Pobranie binarnego GUI w Rust (algodeck-gui.exe)
$GuiExeDest = "$BinDir\algodeck-gui.exe"
if (-not (Test-Path "$GuiExeDest")) {
    Write-Host "`nPobieranie skompilowanej binarki algodeck-gui.exe z GitHub Releases..." -ForegroundColor Cyan
    try {
        Invoke-WebRequest -Uri "https://github.com/veritus-git/AlgoDeck/releases/latest/download/algodeck-gui-windows-x86_64.exe" -OutFile "$GuiExeDest"
        Write-Host "✓ Pobrano algodeck-gui.exe z GitHub Releases!" -ForegroundColor Green
    } catch {
        Write-Host "Nie udalo sie pobrac gotowej binarki GUI (opcjonalne)." -ForegroundColor Yellow
    }
}

# Tworzenie pliku algodeck.cmd w BinDir
$CmdContent = @"
@echo off
set "INSTALL_DIR=$InstallDir"
set "PYTHON_EXE=$VenvDir\Scripts\python.exe"

if "%~1"=="stop" (
    taskkill /F /IM python.exe /FI "WINDOWTITLE eq AlgoDeckDaemon*" >nul 2>&1
    echo [AlgoDeck] Zatrzymano srodowisko.
    exit /b 0
)

if "%~1"=="status" (
    powershell -NoProfile -Command "try { `$r = Invoke-RestMethod 'http://127.0.0.1:8080/api/status' -TimeoutSec 1; Write-Host 'AlgoDeck jest ONLINE' -ForegroundColor Green } catch { Write-Host 'AlgoDeck jest OFFLINE' -ForegroundColor Red }"
    exit /b 0
)

:: Sprawdz czy dziala
powershell -NoProfile -Command "try { `$r = Invoke-RestMethod 'http://127.0.0.1:8080/api/status' -TimeoutSec 1; exit 0 } catch { exit 1 }" >nul 2>&1
if %ERRORLEVEL% equ 0 (
    echo [AlgoDeck] Program juz dziala w tle. Aktywuje ekran zachety...
    powershell -NoProfile -Command "Invoke-RestMethod -Method Post 'http://127.0.0.1:8080/api/launch' | Out-Null" >nul 2>&1
    exit /b 0
)

:: Uruchom w tle (ciche okno ukryte)
start "" /B "%PYTHON_EXE%" -m uvicorn backend.server:app --app-dir "%INSTALL_DIR%" --host 127.0.0.1 --port 8080
timeout /t 1 /nobreak >nul
powershell -NoProfile -Command "Invoke-RestMethod -Method Post 'http://127.0.0.1:8080/api/launch' | Out-Null" >nul 2>&1

echo [AlgoDeck] Program zostal pomyslnie uruchomiony w tle! Ekran zachety aktywny.
exit /b 0
"@

Set-Content -Path "$BinDir\algodeck.cmd" -Value $CmdContent

# Dodaj BinDir do PATH użytkownika
$UserPath = [Environment]::GetEnvironmentVariable("Path", [EnvironmentVariableTarget]::User)
if ($UserPath -notlike "*$BinDir*") {
    [Environment]::SetEnvironmentVariable("Path", "$UserPath;$BinDir", [EnvironmentVariableTarget]::User)
    $env:Path += ";$BinDir"
}

Write-Host ""
Write-Host "==============================================================================" -ForegroundColor Green
Write-Host "       🎉 INSTALACJA ALGODECK NA WINDOWS ZAKOŃCZONA SUKCESEM!               " -ForegroundColor Green
Write-Host "==============================================================================" -ForegroundColor Green
Write-Host ""
Write-Host "🚀 Jak korzystać z AlgoDeck na Windows:" -ForegroundColor Cyan
Write-Host "  1. Wpisz w PowerShell/CMD: algodeck (uruchamia cichy proces w tle)"
Write-Host "  2. Otwórz oficjalną aplikację Elgato Stream Deck - akcje AlgoDeck są już dostępne!"
Write-Host "  3. Zainstaluj rozszerzenie w Chrome/Brave z folderu: $InstallDir\extension"
Write-Host ""
