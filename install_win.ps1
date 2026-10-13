# ==============================================================================
# AlgoDeck ⚡ Koło MAP - Oficjalny Instalator dla Windows (PowerShell)
# 100% Rust Architecture — Zero Python, Zero StreamController, Zero Elgato App
# ==============================================================================

$ErrorActionPreference = "Stop"

Write-Host "`n⚡ AlgoDeck (Koło MAP) - 100% Rust Installer dla Windows`n" -ForegroundColor Cyan

# 1. Sprawdzenie kompilatora C++ (MinGW / MSVC)
if (-not (Get-Command g++ -ErrorAction SilentlyContinue) -and -not (Get-Command cl -ErrorAction SilentlyContinue)) {
    Write-Host "Instalacja MinGW-w64 przez winget..." -ForegroundColor Yellow
    winget install -e --id MSYS2.MSYS2 --accept-source-agreements --accept-package-agreements
}

# 2. Rust & Cargo
if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
    Write-Host "Instalacja Rust (rustup)..." -ForegroundColor Yellow
    winget install -e --id Rustlang.Rustup --accept-source-agreements --accept-package-agreements
    $env:Path = [System.Environment]::GetEnvironmentVariable("Path","User") + ";" + [System.Environment]::GetEnvironmentVariable("Path","Machine")
}

# 3. Kompilacja i instalacja binarki algodeck
$RepoDir = Split-Path -Parent $MyInvocation.MyCommand.Path
Set-Location $RepoDir

Write-Host "Kompilacja natywnej binarki AlgoDeck w Rust..." -ForegroundColor Cyan
cargo build --release

$InstallDir = "$env:LOCALAPPDATA\AlgoDeck"
if (-not (Test-Path $InstallDir)) {
    New-Item -ItemType Directory -Path $InstallDir | Out-Null
}

Copy-Item "$RepoDir\target\release\algodeck.exe" "$InstallDir\algodeck.exe" -Force

# Dodaj do PATH użytkownika
$UserPath = [Environment]::GetEnvironmentVariable("Path", "User")
if ($UserPath -notlike "*$InstallDir*") {
    [Environment]::SetEnvironmentVariable("Path", "$UserPath;$InstallDir", "User")
}

# 4. Rejestracja Native Messaging Host w Rejestrze Windows dla Chrome
$ManifestPath = "$InstallDir\com.algodeck.launcher.json"
$ManifestJson = @"
{
  "name": "com.algodeck.launcher",
  "description": "AlgoDeck 100% Rust Native Messaging Host for Windows",
  "path": "$($InstallDir.Replace('\', '\\'))\\algodeck.exe",
  "type": "stdio",
  "allowed_origins": [
    "chrome-extension://cdgdnkfkkbadljdgjjjbaefefcbelplf/",
    "chrome-extension://jnfimdneicefaomnlikmogffogcadabf/"
  ]
}
"@
Set-Content -Path $ManifestPath -Value $ManifestJson -Encoding UTF8

$RegKey = "HKCU:\Software\Google\Chrome\NativeMessagingHosts\com.algodeck.launcher"
New-Item -Path $RegKey -Force | Out-Null
Set-ItemProperty -Path $RegKey -Name "(Default)" -Value $ManifestPath

Write-Host "`n✓ AlgoDeck v2.0.0 (100% Rust) został pomyślnie zainstalowany!" -ForegroundColor Green
Write-Host "• Uruchom 'algodeck' w PowerShellu"
Write-Host "• Stream Deck: Bezpośrednia obsługa USB HID bez oficjalnej aplikacji Elgato"
