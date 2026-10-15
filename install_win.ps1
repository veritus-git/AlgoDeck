# ==============================================================================
# AlgoDeck ⚡ Koło MAP - Oficjalny Instalator dla Windows (PowerShell)
# 100% Rust Architecture — Zero Python, Zero StreamController, Zero Elgato App
# Odporny na edge-case'y: MinGW C++20, VS Code, ukryty demon VBS, autostart, Registry
# ==============================================================================

$ErrorActionPreference = "Continue"

try {
    [Console]::OutputEncoding = [System.Text.Encoding]::UTF8
    $Host.UI.RawUI.WindowTitle = "AlgoDeck ⚡ Instalator dla Windows"
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
Write-Host "Oficjalny Instalator AlgoDeck v2.0.0 dla Windows 10 / 11 (64-bit)`n" -ForegroundColor White

# Katalogi instalacyjne
$InstallDir   = "$env:LOCALAPPDATA\AlgoDeck"
$FrontendDir  = "$InstallDir\frontend"
$IconsDir     = "$InstallDir\icons"
$ExtensionDir = "$InstallDir\extension"

foreach ($dir in @($InstallDir, $FrontendDir, $IconsDir, $ExtensionDir)) {
    if (-not (Test-Path $dir)) {
        New-Item -ItemType Directory -Path $dir -Force | Out-Null
    }
}

# ------------------------------------------------------------------------------
# KROK 1: Weryfikacja kompilatora C++ (MinGW-w64 / MSVC)
# ------------------------------------------------------------------------------
Write-Host "[1/7] Weryfikacja środowiska kompilatora C++ (C++20)..." -ForegroundColor Yellow

$hasGpp = Get-Command g++ -ErrorAction SilentlyContinue
$hasClang = Get-Command clang++ -ErrorAction SilentlyContinue
$hasCl = Get-Command cl -ErrorAction SilentlyContinue

if ($hasGpp) {
    $ver = (g++ --version | Select-Object -First 1)
    Write-Host "✓ Wykryto kompilator GNU C++: $ver" -ForegroundColor Green
} elseif ($hasClang) {
    $ver = (clang++ --version | Select-Object -First 1)
    Write-Host "✓ Wykryto kompilator Clang C++: $ver" -ForegroundColor Green
} elseif ($hasCl) {
    Write-Host "✓ Wykryto kompilator MSVC (cl.exe)." -ForegroundColor Green
} else {
    Write-Host "Brak wykrytego kompilatora C++ w PATH." -ForegroundColor Yellow
    if (Get-Command winget -ErrorAction SilentlyContinue) {
        Write-Host "Próba automatycznej instalacji MinGW-w64 UCRT (C++20) przez winget..." -ForegroundColor Cyan
        try {
            winget install -e --id BrechtSanders.winlibs-mingw-ucrt --accept-package-agreements --accept-source-agreements --silent
            # Odśwież zmienną PATH w bieżącej sesji
            $env:Path = [Environment]::GetEnvironmentVariable("Path", "Machine") + ";" + [Environment]::GetEnvironmentVariable("Path", "User")
            if (Get-Command g++ -ErrorAction SilentlyContinue) {
                Write-Host "✓ Pomyślnie zainstalowano MinGW C++20!" -ForegroundColor Green
            }
        } catch {
            Write-Host "Ostrzeżenie: Nie udało się zainstalować MinGW automatycznie. Możesz zainstalować go ręcznie z winlibs.com." -ForegroundColor DarkYellow
        }
    } else {
        Write-Host "Zainstaluj MinGW-w64 (https://winlibs.com) lub Visual Studio C++, aby móc lokalnie kompilować zadania olimpijskie." -ForegroundColor DarkYellow
    }
}

# ------------------------------------------------------------------------------
# KROK 2: Weryfikacja edytora Visual Studio Code
# ------------------------------------------------------------------------------
Write-Host "`n[2/7] Weryfikacja edytora Visual Studio Code..." -ForegroundColor Yellow

$hasVscode = Get-Command code -ErrorAction SilentlyContinue
if (-not $hasVscode) {
    # Sprawdź domyślne ścieżki instalacji VS Code
    $vsCodeDefault = "$env:LOCALAPPDATA\Programs\Microsoft VS Code\bin\code.cmd"
    if (Test-Path $vsCodeDefault) {
        $hasVscode = $true
        $env:Path = "$env:Path;$env:LOCALAPPDATA\Programs\Microsoft VS Code\bin"
    }
}

if ($hasVscode) {
    Write-Host "✓ Wykryto edytor Visual Studio Code." -ForegroundColor Green
    Write-Host "Instalacja / weryfikacja wtyczki C/C++ (ms-vscode.cpptools)..." -ForegroundColor Cyan
    try {
        & code --install-extension ms-vscode.cpptools --force 2>$null | Out-Null
        Write-Host "✓ Rozszerzenie C/C++ dla VS Code jest zainstalowane." -ForegroundColor Green
    } catch {}
} else {
    Write-Host "Visual Studio Code nie został znaleziony w PATH." -ForegroundColor Yellow
    if (Get-Command winget -ErrorAction SilentlyContinue) {
        Write-Host "Czy chcesz zainstalować VS Code automatycznie przez winget? (Próba cichej instalacji...)" -ForegroundColor Cyan
        try {
            winget install -e --id Microsoft.VisualStudioCode --accept-package-agreements --accept-source-agreements --silent
        } catch {}
    } else {
        Write-Host "Zainstaluj VS Code z oficjalnej strony: https://code.visualstudio.com/" -ForegroundColor DarkYellow
    }
}

# ------------------------------------------------------------------------------
# KROK 3: Instalacja silnika AlgoDeck (algodeck.exe)
# ------------------------------------------------------------------------------
Write-Host "`n[3/7] Instalacja natywnego silnika AlgoDeck (100% Rust)..." -ForegroundColor Yellow

# Zatrzymaj działający proces jeśli istnieje, aby nie blokować pliku
Get-Process algodeck -ErrorAction SilentlyContinue | Stop-Process -Force -ErrorAction SilentlyContinue
Start-Sleep -Milliseconds 300

$BinaryPath = "$InstallDir\algodeck.exe"
$installed = $false

# 3a. Czy skrypt uruchomiono w sklonowanym repozytorium lub wypakowanej paczce?
$ScriptDir = if ($PSScriptRoot) { $PSScriptRoot } elseif ($MyInvocation.MyCommand.Path) { Split-Path -Parent $MyInvocation.MyCommand.Path } else { (Get-Location).Path }

if (Test-Path "$ScriptDir\algodeck.exe") {
    Write-Host "Kopiowanie prekompilowanej binarki z bieżącego folderu..." -ForegroundColor Cyan
    Copy-Item "$ScriptDir\algodeck.exe" $BinaryPath -Force
    $installed = $true
} elseif (Test-Path "$ScriptDir\target\release\algodeck.exe") {
    Write-Host "Kopiowanie lokalnie skompilowanej binarki release z $ScriptDir..." -ForegroundColor Cyan
    Copy-Item "$ScriptDir\target\release\algodeck.exe" $BinaryPath -Force
    $installed = $true
} elseif ((Test-Path "$ScriptDir\Cargo.toml") -and (Get-Command cargo -ErrorAction SilentlyContinue)) {
    Write-Host "Kompilacja ze źródeł lokalnych (cargo build --release)..." -ForegroundColor Cyan
    Push-Location $ScriptDir
    try {
        cargo build --release
        Copy-Item "$ScriptDir\target\release\algodeck.exe" $BinaryPath -Force
        $installed = $true
    } finally {
        Pop-Location
    }
}

# 3b. Pobranie z GitHub Releases
if (-not $installed) {
    Write-Host "Pobieranie paczki AlgoDeck z GitHub Releases..." -ForegroundColor Cyan
    $ZipReleaseUrl = "https://github.com/veritus-git/AlgoDeck/releases/latest/download/algodeck-windows-x86_64.zip"
    $TempZip = "$env:TEMP\algodeck_win_$PID.zip"
    $TempExtract = "$env:TEMP\algodeck_extract_$PID"

    try {
        [Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12
        Invoke-WebRequest -Uri $ZipReleaseUrl -OutFile $TempZip -UseBasicParsing
        if (Test-Path $TempExtract) { Remove-Item $TempExtract -Recurse -Force }
        Expand-Archive -Path $TempZip -DestinationPath $TempExtract -Force

        if (Test-Path "$TempExtract\algodeck.exe") {
            Copy-Item "$TempExtract\algodeck.exe" $BinaryPath -Force
            $installed = $true
        }
        if (Test-Path "$TempExtract\frontend") {
            Copy-Item "$TempExtract\frontend\*" $FrontendDir -Recurse -Force
        }
        if (Test-Path "$TempExtract\extension") {
            Copy-Item "$TempExtract\extension\*" $ExtensionDir -Recurse -Force
        }
        if (Test-Path "$TempExtract\vscode_extension") {
            $VsCodeExtDir = "$env:USERPROFILE\.vscode\extensions\algodeck-vscode-bridge"
            if (-not (Test-Path $VsCodeExtDir)) { New-Item -ItemType Directory -Path $VsCodeExtDir -Force | Out-Null }
            Copy-Item "$TempExtract\vscode_extension\*" $VsCodeExtDir -Recurse -Force
        }
        Remove-Item $TempZip -Force -ErrorAction SilentlyContinue
        Remove-Item $TempExtract -Recurse -Force -ErrorAction SilentlyContinue
        Write-Host "✓ Pomyślnie zainstalowano paczkę z GitHub Releases." -ForegroundColor Green
    } catch {
        Write-Host "Paczka release nie jest jeszcze dostępna. Pobieranie surowej binarki..." -ForegroundColor Yellow
        $RawExeUrl = "https://github.com/veritus-git/AlgoDeck/releases/latest/download/algodeck-windows-x86_64.exe"
        try {
            Invoke-WebRequest -Uri $RawExeUrl -OutFile $BinaryPath -UseBasicParsing
            $installed = $true
            Write-Host "✓ Pomyślnie pobrano algodeck.exe." -ForegroundColor Green
        } catch {
            Write-Host "Błąd pobierania binarki: $($_.Exception.Message)" -ForegroundColor Red
        }
    }
}

if (-not (Test-Path $BinaryPath)) {
    Write-Host "BŁĄD: Nie udało się zainstalować algodeck.exe! Sprawdź połączenie z internetem." -ForegroundColor Red
    exit 1
}

# ------------------------------------------------------------------------------
# KROK 4: Kopiowanie kompletnego frontendu, ikon i rozszerzenia
# ------------------------------------------------------------------------------
Write-Host "`n[4/7] Instalacja plików Web Hubu, grafik i rozszerzenia..." -ForegroundColor Yellow

$copiedAssets = $false

if ((Test-Path "$ScriptDir\frontend") -and (Test-Path "$ScriptDir\extension")) {
    Copy-Item "$ScriptDir\frontend\*" $FrontendDir -Recurse -Force
    Copy-Item "$ScriptDir\extension\*" $ExtensionDir -Recurse -Force
    if (Test-Path "$ScriptDir\frontend\icons") {
        Copy-Item "$ScriptDir\frontend\icons\*" $IconsDir -Recurse -Force
    }
    if (Test-Path "$ScriptDir\vscode_extension") {
        $VsCodeExtDir = "$env:USERPROFILE\.vscode\extensions\algodeck-vscode-bridge"
        if (-not (Test-Path $VsCodeExtDir)) { New-Item -ItemType Directory -Path $VsCodeExtDir -Force | Out-Null }
        Copy-Item "$ScriptDir\vscode_extension\*" $VsCodeExtDir -Recurse -Force
    }
    Write-Host "✓ Skopiowano zasoby z lokalnego katalogu." -ForegroundColor Green
    $copiedAssets = $true
}

if (-not $copiedAssets) {
    Write-Host "Pobieranie najnowszych zasobów z repozytorium GitHub..." -ForegroundColor Cyan
    $ZipAssets = "$env:TEMP\algodeck_assets_$PID.zip"
    $TempExtract = "$env:TEMP\algodeck_assets_extract_$PID"

    try {
        [Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12
        Invoke-WebRequest -Uri "https://github.com/veritus-git/AlgoDeck/archive/refs/heads/main.zip" -OutFile $ZipAssets -UseBasicParsing
        if (Test-Path $TempExtract) { Remove-Item $TempExtract -Recurse -Force }
        Expand-Archive -Path $ZipAssets -DestinationPath $TempExtract -Force

        $RepoRoot = Get-ChildItem -Path $TempExtract -Directory | Select-Object -First 1
        if ($RepoRoot) {
            if (Test-Path "$($RepoRoot.FullName)\frontend") {
                Copy-Item "$($RepoRoot.FullName)\frontend\*" $FrontendDir -Recurse -Force
            }
            if (Test-Path "$($RepoRoot.FullName)\extension") {
                Copy-Item "$($RepoRoot.FullName)\extension\*" $ExtensionDir -Recurse -Force
            }
            if (Test-Path "$($RepoRoot.FullName)\frontend\icons") {
                Copy-Item "$($RepoRoot.FullName)\frontend\icons\*" $IconsDir -Recurse -Force
            }
            if (Test-Path "$($RepoRoot.FullName)\vscode_extension") {
                $VsCodeExtDir = "$env:USERPROFILE\.vscode\extensions\algodeck-vscode-bridge"
                if (-not (Test-Path $VsCodeExtDir)) { New-Item -ItemType Directory -Path $VsCodeExtDir -Force | Out-Null }
                Copy-Item "$($RepoRoot.FullName)\vscode_extension\*" $VsCodeExtDir -Recurse -Force
            }
            Write-Host "✓ Zasoby Web Hub i rozszerzenia pomyślnie zsynchronizowane." -ForegroundColor Green
        }
        Remove-Item $ZipAssets -Force -ErrorAction SilentlyContinue
        Remove-Item $TempExtract -Recurse -Force -ErrorAction SilentlyContinue
    } catch {
        Write-Host "Ostrzeżenie: Nie udało się zaktualizować paczki zasobów ($($_.Exception.Message))." -ForegroundColor DarkYellow
    }
}

# ------------------------------------------------------------------------------
# KROK 5: Weryfikacja konfliktów Stream Decka (oficjalna aplikacja Elgato)
# ------------------------------------------------------------------------------
Write-Host "`n[5/7] Weryfikacja kontrolera Stream Deck i konfliktów USB HID..." -ForegroundColor Yellow

$elgatoProc = Get-Process StreamDeck -ErrorAction SilentlyContinue
if ($elgatoProc) {
    Write-Host "==================================================================" -ForegroundColor Red
    Write-Host "⚠️  UWAGA: WYKRYTO URUCHOMIONĄ OFICJALNĄ APLIKACJĘ ELGATO STREAM DECK!" -ForegroundColor Yellow
    Write-Host "Oficjalny program Elgato blokuje bezpośredni port USB HID Stream Decka." -ForegroundColor White
    Write-Host "Aby AlgoDeck mógł sterować przyciskami:" -ForegroundColor White
    Write-Host "  1. Kliknij prawym przyciskiem myszy na ikonę Stream Deck w zasobniku systemowym (obok zegara)" -ForegroundColor Cyan
    Write-Host "  2. Wybierz 'Zakończ' / 'Quit'" -ForegroundColor Cyan
    Write-Host "==================================================================" -ForegroundColor Red
} else {
    Write-Host "✓ Brak konfliktujących aplikacji Elgato. Port USB HID jest wolny dla AlgoDeck." -ForegroundColor Green
}

# ------------------------------------------------------------------------------
# KROK 6: Rejestracja Native Messaging Host w Rejestrze Windows
# ------------------------------------------------------------------------------
Write-Host "`n[6/7] Rejestracja Native Messaging Host dla przeglądarek Chromium..." -ForegroundColor Yellow

# Oblicz unikalny identyfikator rozszerzenia Chrome dla lokalnego folderu rozszerzenia
$LocalExtId = ""
try {
    $sha256 = [System.Security.Cryptography.SHA256]::Create()
    $bytes = [System.Text.Encoding]::UTF8.GetBytes($ExtensionDir)
    $hash = $sha256.ComputeHash($bytes)
    $chars = @()
    for ($i = 0; $i -lt 16; $i++) {
        $chars += [char](97 + [int]($hash[$i] -shr 4))
        $chars += [char](97 + [int]($hash[$i] -band 15))
    }
    $LocalExtId = -join $chars
} catch {}

$AllowedOrigins = @(
    "chrome-extension://cdgdnkfkkbadljdgjjjbaefefcbelplf/",
    "chrome-extension://jnfimdneicefaomnlikmogffogcadabf/"
)
if ($LocalExtId) {
    $AllowedOrigins += "chrome-extension://$LocalExtId/"
}
$AllowedOriginsJson = ($AllowedOrigins | ForEach-Object { "    `"$_`"" }) -join ",`n"

$ManifestPath = "$InstallDir\com.algodeck.launcher.json"
$ManifestJson = @"
{
  "name": "com.algodeck.launcher",
  "description": "AlgoDeck 100% Rust Native Messaging Host for Windows",
  "path": "$($BinaryPath.Replace('\', '\\'))",
  "type": "stdio",
  "allowed_origins": [
$AllowedOriginsJson
  ]
}
"@
[System.IO.File]::WriteAllText($ManifestPath, $ManifestJson, [System.Text.Encoding]::UTF8)

$RegTargets = @(
    "HKCU:\Software\Google\Chrome\NativeMessagingHosts\com.algodeck.launcher",
    "HKCU:\Software\Microsoft\Edge\NativeMessagingHosts\com.algodeck.launcher",
    "HKCU:\Software\BraveSoftware\Brave-Browser\NativeMessagingHosts\com.algodeck.launcher",
    "HKCU:\Software\Chromium\NativeMessagingHosts\com.algodeck.launcher",
    "HKCU:\Software\Vivaldi\NativeMessagingHosts\com.algodeck.launcher",
    "HKCU:\Software\Opera Software\Opera Stable\NativeMessagingHosts\com.algodeck.launcher"
)

foreach ($reg in $RegTargets) {
    try {
        if (-not (Test-Path $reg)) {
            New-Item -Path $reg -Force | Out-Null
        }
        Set-ItemProperty -Path $reg -Name "(Default)" -Value $ManifestPath -Force
    } catch {}
}
Write-Host "✓ Mostek Native Messaging zarejestrowany w Rejestrze dla Chrome, Edge, Brave i innych." -ForegroundColor Green

# ------------------------------------------------------------------------------
# KROK 7: Konfiguracja ukrytego demona w tle, skrótów i autostartu
# ------------------------------------------------------------------------------
Write-Host "`n[7/7] Konfiguracja cichej usługi w tle (Zero okien konsoli)..." -ForegroundColor Yellow

# Skrypt VBS do bezokienkowego uruchamiania w tle
$VbsPath = "$InstallDir\algodeck_daemon.vbs"
$VbsContent = @"
Set WshShell = CreateObject("WScript.Shell")
WshShell.Run Chr(34) & "$($BinaryPath.Replace('\', '\\'))" & Chr(34) & " daemon", 0, False
"@
[System.IO.File]::WriteAllText($VbsPath, $VbsContent, [System.Text.Encoding]::ASCII)

# Kompatybilność z funkcją dynamicznego przełączania autostartu w Rust (src/server.rs)
Copy-Item $VbsPath "$InstallDir\AlgoDeck.vbs" -Force

# Skrypty pomocnicze CMD w folderze instalacyjnym
$StartCmd = "$InstallDir\algodeck-start.cmd"
[System.IO.File]::WriteAllText($StartCmd, "@echo off`r`nwscript.exe `"$VbsPath`"`r`n", [System.Text.Encoding]::ASCII)

$StopCmd = "$InstallDir\algodeck-stop.cmd"
[System.IO.File]::WriteAllText($StopCmd, "@echo off`r`n`"$BinaryPath`" stop`r`n", [System.Text.Encoding]::ASCII)

# Autostart w folderze Startup użytkownika
try {
    $StartupDir = [Environment]::GetFolderPath("Startup")
    $StartupVbs = Join-Path $StartupDir "AlgoDeck.vbs"
    Copy-Item $VbsPath $StartupVbs -Force
    Write-Host "✓ Skrypt autostartu dodany do folderu Autostart Windows." -ForegroundColor Green
} catch {}

# Skrót na Pulpicie (Desktop)
try {
    $DesktopDir = [Environment]::GetFolderPath("Desktop")
    $WshShell = New-Object -ComObject WScript.Shell
    $Shortcut = $WshShell.CreateShortcut("$DesktopDir\AlgoDeck.lnk")
    $Shortcut.TargetPath = "wscript.exe"
    $Shortcut.Arguments = "`"$VbsPath`""
    $Shortcut.WorkingDirectory = $InstallDir
    $Shortcut.Description = "AlgoDeck ⚡ Koło MAP (Uruchom w tle)"
    if (Test-Path "$IconsDir\icon128.png") {
        $Shortcut.IconLocation = "$BinaryPath,0"
    }
    $Shortcut.Save()
    Write-Host "✓ Skrót Pulpitu utworzony: AlgoDeck.lnk" -ForegroundColor Green
} catch {}

# Dodaj do zmiennej środowiskowej PATH użytkownika
$UserPath = [Environment]::GetEnvironmentVariable("Path", "User")
if ($UserPath -notlike "*$InstallDir*") {
    [Environment]::SetEnvironmentVariable("Path", "$UserPath;$InstallDir", "User")
    $env:Path = "$env:Path;$InstallDir"
    Write-Host "✓ Dodano $InstallDir do zmiennej środowiskowej PATH." -ForegroundColor Green
} else {
    Write-Host "✓ Ścieżka $InstallDir znajduje się w PATH." -ForegroundColor Green
}

# Uruchom demona natychmiastowo w tle
try {
    Start-Process "wscript.exe" -ArgumentList "`"$VbsPath`""
    Write-Host "✓ Demon AlgoDeck został uruchomiony w tle na porcie 8080." -ForegroundColor Green
} catch {}

# ------------------------------------------------------------------------------
# PODSUMOWANIE INSTALACJI
# ------------------------------------------------------------------------------
Write-Host "`n==================================================================" -ForegroundColor Cyan
Write-Host "🎉 AlgoDeck v2.0.0 (100% Rust) został pomyślnie zainstalowany!" -ForegroundColor Green
Write-Host "==================================================================" -ForegroundColor Cyan
Write-Host "• Panel Web Hub:   http://127.0.0.1:8080/ lub przez rozszerzenie" -ForegroundColor White
Write-Host "• Kontrola CLI:    algodeck status  |  algodeck test <kod>  |  algodeck stop" -ForegroundColor White
Write-Host "• Skróty CMD:      algodeck-start  |  algodeck-stop" -ForegroundColor White
Write-Host "• Folder programu: $InstallDir" -ForegroundColor White
Write-Host "==================================================================" -ForegroundColor Cyan

Write-Host "`n🧩 JAK ZAŁADOWAĆ ROZSZERZENIE DO PRZEGLĄDARKI (Chrome / Brave / Edge):" -ForegroundColor Yellow
Write-Host "1. Otwórz w przeglądarce: chrome://extensions  (lub brave://extensions / edge://extensions)" -ForegroundColor White
Write-Host "2. Włącz w prawym górnym rogu 'Tryb programisty' (Developer mode)" -ForegroundColor White
Write-Host "3. Kliknij w lewym górnym rogu przycisk 'Załaduj rozpakowane' (Load unpacked)" -ForegroundColor White
Write-Host "4. W oknie wyboru wskaż DOKŁADNIE ten folder:" -ForegroundColor White
Write-Host "   $ExtensionDir" -ForegroundColor Green
Write-Host "5. Gotowe! Przypnij ikonę ⚡ AlgoDeck na pasku narzędzi przeglądarki.`n" -ForegroundColor Cyan
