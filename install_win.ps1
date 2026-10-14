# ==============================================================================
# AlgoDeck ⚡ Koło MAP - Oficjalny Instalator dla Windows (PowerShell)
# 100% Rust Architecture — Zero Python, Zero StreamController, Zero Elgato App
# ==============================================================================

$ErrorActionPreference = "Stop"

try {
    [Console]::OutputEncoding = [System.Text.Encoding]::UTF8
} catch {}

$banner = @"

  █████╗ ██╗      ██████╗  ██████╗ ██████╗ ███████╗ ██████╗██╗  ██╗
 ██╔══██╗██║     ██╔════╝ ██╔═══██╗██╔══██╗██╔════╝██╔════╝██║ ██╔╝
 ███████║██║     ██║  ███╗██║   ██║██║  ██║█████╗  ██║     █████═╝ 
 ██╔══██║██║     ██║   ██║██║   ██║██║  ██║██╔══╝  ██║     ██╔═██╗ 
 ██║  ██║███████╗╚██████╔╝╚██████╔╝██████╔╝███████╗╚██████╗██║  ██╗
 ╚═╝  ╚═╝╚══════╝ ╚═════╝  ╚═════╝ ╚═════╝ ╚══════╝ ╚═════╝╚═╝  ╚═╝

       ⚡  K O Ł O   M A P   -   O L I M P I C K I E   C + +  ⚡
       ⚡       100% RUST ENGINE  |  STREAM DECK HID        ⚡

"@
Write-Host $banner -ForegroundColor Cyan

# Katalog instalacyjny
$InstallDir = "$env:LOCALAPPDATA\AlgoDeck"
$IconsDir = "$InstallDir\icons"
if (-not (Test-Path $InstallDir)) {
    New-Item -ItemType Directory -Path $InstallDir -Force | Out-Null
}
if (-not (Test-Path $IconsDir)) {
    New-Item -ItemType Directory -Path $IconsDir -Force | Out-Null
}

# 1. Sprawdzenie kompilatora C++ (MinGW / MSVC)
Write-Host "`n[1/4] Sprawdzanie środowiska kompilatora C++..." -ForegroundColor Yellow
$hasCompiler = (Get-Command g++ -ErrorAction SilentlyContinue) -or (Get-Command cl -ErrorAction SilentlyContinue)
if (-not $hasCompiler) {
    Write-Host "Brak wykrytego kompilatora C++. Próba instalacji MinGW-w64 (MSYS2)..." -ForegroundColor Yellow
    if (Get-Command winget -ErrorAction SilentlyContinue) {
        try {
            winget install -e --id MSYS2.MSYS2 --accept-source-agreements --accept-package-agreements
        } catch {
            Write-Host "Nie udało się automatycznie zainstalować MSYS2 przez winget." -ForegroundColor DarkYellow
        }
    } else {
        Write-Host "Zainstaluj MinGW lub Visual Studio C++, aby móc lokalnie kompilować i testować zadania olimpijskie." -ForegroundColor DarkYellow
    }
} else {
    Write-Host "✓ Wykryto kompilator C++ w systemie." -ForegroundColor Green
}

# 2. Pobieranie / Instalacja binarki AlgoDeck
Write-Host "`n[2/4] Instalacja silnika AlgoDeck..." -ForegroundColor Yellow
$BinaryPath = "$InstallDir\algodeck.exe"
$installed = $false

# Sprawdź czy skrypt jest uruchomiony z lokalnego sklonowanego repozytorium
if ($MyInvocation.MyCommand -and $MyInvocation.MyCommand.Path) {
    $LocalRepo = Split-Path -Parent $MyInvocation.MyCommand.Path
    if (Test-Path "$LocalRepo\target\release\algodeck.exe") {
        Write-Host "Kopiowanie lokalnie skompilowanej binarki z $LocalRepo..." -ForegroundColor Cyan
        Copy-Item "$LocalRepo\target\release\algodeck.exe" $BinaryPath -Force
        $installed = $true
    }
}

if (-not $installed) {
    Write-Host "Pobieranie natywnej binarki z GitHub Releases..." -ForegroundColor Cyan
    $ReleaseUrl = "https://github.com/veritus-git/AlgoDeck/releases/latest/download/algodeck-windows-x86_64.exe"
    try {
        [Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12
        Invoke-WebRequest -Uri $ReleaseUrl -OutFile $BinaryPath -UseBasicParsing
        Write-Host "✓ Pomyślnie pobrano algodeck.exe." -ForegroundColor Green
        $installed = $true
    } catch {
        Write-Host "Wydanie GitHub Releases nie jest jeszcze opublikowane lub brak połączenia ($($_.Exception.Message))." -ForegroundColor Yellow
        if (Get-Command cargo -ErrorAction SilentlyContinue) {
            Write-Host "Wykryto Cargo w systemie. Kompilacja ze źródeł..." -ForegroundColor Cyan
            if ($MyInvocation.MyCommand -and $MyInvocation.MyCommand.Path) {
                $RepoDir = Split-Path -Parent $MyInvocation.MyCommand.Path
                Push-Location $RepoDir
                cargo build --release
                Pop-Location
                Copy-Item "$RepoDir\target\release\algodeck.exe" $BinaryPath -Force
                $installed = $true
            }
        }
    }
}

# 3. Konfiguracja zestawu ikon dla Stream Decka
Write-Host "`n[3/4] Konfiguracja grafik i ikon dla Stream Decka..." -ForegroundColor Yellow
$copiedIcons = $false

if ($MyInvocation.MyCommand -and $MyInvocation.MyCommand.Path) {
    $LocalIcons = Join-Path (Split-Path -Parent $MyInvocation.MyCommand.Path) "frontend\icons"
    if (Test-Path $LocalIcons) {
        Copy-Item "$LocalIcons\*" $IconsDir -Recurse -Force
        Write-Host "✓ Skopiowano ikony z lokalnego repozytorium do $IconsDir" -ForegroundColor Green
        $copiedIcons = $true
    }
}

if (-not $copiedIcons) {
    Write-Host "Pobieranie grafik Stream Decka z repozytorium..." -ForegroundColor Cyan
    $ZipPath = "$env:TEMP\algodeck_icons_$PID.zip"
    $TempExtract = "$env:TEMP\algodeck_extract_$PID"
    try {
        Invoke-WebRequest -Uri "https://github.com/veritus-git/AlgoDeck/archive/refs/heads/main.zip" -OutFile $ZipPath -UseBasicParsing
        if (Test-Path $TempExtract) { Remove-Item $TempExtract -Recurse -Force }
        Expand-Archive -Path $ZipPath -DestinationPath $TempExtract -Force
        $ExtractedIcons = Get-ChildItem -Path $TempExtract -Recurse -Directory -Filter "icons" | Where-Object { $_.FullName -like "*frontend*icons*" } | Select-Object -First 1
        if ($ExtractedIcons) {
            Copy-Item "$($ExtractedIcons.FullName)\*" $IconsDir -Recurse -Force
            Write-Host "✓ Ikony zainstalowane w $IconsDir" -ForegroundColor Green
        }
        Remove-Item $ZipPath -Force -ErrorAction SilentlyContinue
        Remove-Item $TempExtract -Recurse -Force -ErrorAction SilentlyContinue
    } catch {
        Write-Host "Ostrzeżenie: Nie udało się pobrać paczki ikon: $($_.Exception.Message)" -ForegroundColor DarkYellow
    }
}

# 4. Dodaj do PATH użytkownika
$UserPath = [Environment]::GetEnvironmentVariable("Path", "User")
if ($UserPath -notlike "*$InstallDir*") {
    [Environment]::SetEnvironmentVariable("Path", "$UserPath;$InstallDir", "User")
    $env:Path = "$env:Path;$InstallDir"
    Write-Host "✓ Dodano $InstallDir do zmiennej środowiskowej PATH." -ForegroundColor Green
}

# 5. Rejestracja Native Messaging Host w Rejestrze Windows dla Chrome
Write-Host "`n[4/4] Rejestracja Native Messaging Host dla Google Chrome..." -ForegroundColor Yellow
$ManifestPath = "$InstallDir\com.algodeck.launcher.json"
$ManifestJson = @"
{
  "name": "com.algodeck.launcher",
  "description": "AlgoDeck 100% Rust Native Messaging Host for Windows",
  "path": "$($BinaryPath.Replace('\', '\\'))",
  "type": "stdio",
  "allowed_origins": [
    "chrome-extension://cdgdnkfkkbadljdgjjjbaefefcbelplf/",
    "chrome-extension://jnfimdneicefaomnlikmogffogcadabf/"
  ]
}
"@
[System.IO.File]::WriteAllText($ManifestPath, $ManifestJson, [System.Text.Encoding]::UTF8)

$RegKey = "HKCU:\Software\Google\Chrome\NativeMessagingHosts\com.algodeck.launcher"
New-Item -Path $RegKey -Force | Out-Null
Set-ItemProperty -Path $RegKey -Name "(Default)" -Value $ManifestPath

Write-Host "`n==================================================================" -ForegroundColor Cyan
Write-Host "✓ AlgoDeck v2.0.0 (100% Rust) został pomyślnie zainstalowany!" -ForegroundColor Green
Write-Host "• Uruchom 'algodeck' w PowerShellu lub Command Prompcie" -ForegroundColor White
Write-Host "• Stream Deck: Bezpośrednia obsługa USB HID bez oficjalnej aplikacji Elgato" -ForegroundColor White
Write-Host "• Konfiguracja i ikony: $InstallDir" -ForegroundColor White
Write-Host "==================================================================`n" -ForegroundColor Cyan
