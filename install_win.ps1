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

# 3. Sprawdzanie i instalacja kompilatora C++ (G++ / MinGW)
Write-Host "`n[3/6] Sprawdzanie kompilatora C++ (G++ / MinGW dla C++20)..." -ForegroundColor Cyan
$GppPath = Get-Command g++ -ErrorAction SilentlyContinue
if (-not $GppPath) {
    Write-Host "Kompilator g++ nie zostal wykryty. Automatyczna instalacja MinGW-w64..." -ForegroundColor Yellow
    $Installed = $false
    try {
        winget install -e --id BrechtSanders.winlibs.posix.ucrt.gcc --accept-package-agreements --accept-source-agreements --silent
        $Installed = $true
    } catch {}

    if (-not $Installed) {
        Write-Host "Pobieranie w64devkit (GCC/G++ 14 dla C++20)..." -ForegroundColor Cyan
        try {
            $W64Zip = "$env:TEMP\w64devkit.zip"
            Invoke-WebRequest -Uri "https://github.com/skeeto/w64devkit/releases/download/v2.0.0/w64devkit-2.0.0.zip" -OutFile $W64Zip
            Expand-Archive -Path $W64Zip -DestinationPath "$InstallDir" -Force
            Remove-Item -Force $W64Zip
            $MingwBin = "$InstallDir\w64devkit\bin"
            $UserPath = [Environment]::GetEnvironmentVariable("Path", "User")
            if ($UserPath -notlike "*$MingwBin*") {
                [Environment]::SetEnvironmentVariable("Path", "$UserPath;$MingwBin", "User")
                $env:Path += ";$MingwBin"
            }
            Write-Host "✓ Zainstalowano w64devkit G++ w $MingwBin" -ForegroundColor Green
        } catch {
            Write-Host "Ostrzezenie: Nie udalo sie automatycznie pobrac MinGW. Zainstaluj MinGW recznie." -ForegroundColor Yellow
        }
    }
} else {
    Write-Host "✓ Wykryto kompilator g++." -ForegroundColor Green
}

# 4. Sprawdzanie i instalacja Visual Studio Code
Write-Host "`n[4/6] Sprawdzanie Visual Studio Code..." -ForegroundColor Cyan
$CodeCmd = Get-Command code -ErrorAction SilentlyContinue
if (-not $CodeCmd) {
    Write-Host "Visual Studio Code nie zostal wykryty. Instalacja przez winget..." -ForegroundColor Yellow
    try {
        winget install -e --id Microsoft.VisualStudioCode --accept-package-agreements --accept-source-agreements --silent
    } catch {}
}

$CodeCmd = Get-Command code -ErrorAction SilentlyContinue
if ($CodeCmd) {
    Write-Host "✓ Visual Studio Code zainstalowany." -ForegroundColor Green
    & code --install-extension ms-vscode.cpptools --force | Out-Null
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

& $PythonVenv -m pip install --upgrade pip --quiet
& $PythonVenv -m pip install -r "$InstallDir\requirements.txt" --quiet
& $PythonVenv -m pip install websockets --quiet

# 6. Oficjalny plugin Elgato Stream Deck na Windows
Write-Host "`n[6/6] Konfiguracja Elgato Stream Deck (Program i plugin Windows)..." -ForegroundColor Cyan
$StreamDeckAppExe = "$env:ProgramFiles\Elgato\StreamDeck\StreamDeck.exe"
if (-not (Test-Path "$StreamDeckAppExe")) {
    Write-Host "Oficjalna aplikacja Elgato Stream Deck nie zostala wykryta. Instalacja przez winget..." -ForegroundColor Yellow
    try {
        winget install -e --id Corsair.StreamDeck --accept-package-agreements --accept-source-agreements --silent
    } catch {}
}

$ElgatoPluginsDir = "$env:APPDATA\Elgato\StreamDeck\Plugins"
$PluginDest = "$ElgatoPluginsDir\com.algodeck.streamdeck.sdPlugin"

if (Test-Path "$InstallDir\streamdeck_plugin\com.algodeck.streamdeck.sdPlugin") {
    New-Item -ItemType Directory -Force -Path $ElgatoPluginsDir | Out-Null
    Copy-Item -Recurse -Force "$InstallDir\streamdeck_plugin\com.algodeck.streamdeck.sdPlugin" "$PluginDest"
    Write-Host "✓ Zainstalowano plugin Elgato Stream Deck w: $PluginDest" -ForegroundColor Green
}

# Rejestracja Native Messaging Host w Windows Registry
$NmJsonPath = "$InstallDir\com.algodeck.launcher.json"
$NmContent = @"
{
  "name": "com.algodeck.launcher",
  "description": "AlgoDeck Native Messaging Host for browser launching",
  "path": "$BinDir\algodeck-native-host.bat",
  "type": "stdio",
  "allowed_origins": [
    "chrome-extension://cdgdnkfkkbadljdgjjjbaefefcbelplf/"
  ]
}
"@
Set-Content -Path $NmJsonPath -Value $NmContent

$RegKeys = @(
    "HKCU:\Software\Google\Chrome\NativeMessagingHosts\com.algodeck.launcher",
    "HKCU:\Software\BraveSoftware\Brave-Browser\NativeMessagingHosts\com.algodeck.launcher",
    "HKCU:\Software\Chromium\NativeMessagingHosts\com.algodeck.launcher"
)
foreach ($reg in $RegKeys) {
    New-Item -Path $reg -Force | Out-Null
    Set-ItemProperty -Path $reg -Name "(Default)" -Value $NmJsonPath
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
set "BIN_DIR=$BinDir"
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

if "%~1"=="gui" (
    if exist "$BinDir\algodeck-gui.exe" (
        start "" "$BinDir\algodeck-gui.exe" %2 %3 %4
    ) else (
        echo [AlgoDeck] algodeck-gui.exe nie zostal znaleziony w $BinDir
    )
    exit /b 0
)

:: Sprawdz czy dziala
powershell -NoProfile -Command "try { `$r = Invoke-RestMethod 'http://127.0.0.1:8080/api/status' -TimeoutSec 1; exit 0 } catch { exit 1 }" >nul 2>&1
if %ERRORLEVEL% equ 0 (
    echo [AlgoDeck] Program juz dziala w tle. Aktywuje ekran zachety...
    powershell -NoProfile -Command "Invoke-RestMethod -Method Post 'http://127.0.0.1:8080/api/launch' | Out-Null" >nul 2>&1
    exit /b 0
)

:: Uruchom w tle (ciche okno ukryte z tytulem AlgoDeckDaemon)
start "AlgoDeckDaemon" /B "%PYTHON_EXE%" -m uvicorn backend.server:app --app-dir "%INSTALL_DIR%" --host 127.0.0.1 --port 8080
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
