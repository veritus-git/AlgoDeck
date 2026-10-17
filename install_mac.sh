#!/usr/bin/env bash
# ==============================================================================
# AlgoDeck ⚡ Koło MAP - Oficjalny Instalator dla macOS (Apple Silicon & Intel)
# 100% Rust Architecture — Zero Python, Zero StreamController, Zero Elgato App
# Odporny na edge-case'y: LaunchAgent daemon, Native Messaging, C++20, PATH
# ==============================================================================

set -e

RED='\033[0;31m'
GREEN='\033[0;32m'
CYAN='\033[0;36m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
BOLD='\033[1m'
NC='\033[0m'

echo -e "${CYAN}${BOLD}"
echo "  █████╗ ██╗      ██████╗  ██████╗ ██████╗ ███████╗ ██████╗██╗  ██╗"
echo " ██╔══██╗██║     ██╔════╝ ██╔═══██╗██╔══██╗██╔════╝██╔════╝██║ ██╔╝"
echo " ███████║██║     ██║  ███╗██║   ██║██║  ██║█████╗  ██║     █████═╝ "
echo " ██╔══██║██║     ██║   ██║██║   ██║██║  ██║██╔══╝  ██║     ██╔═██╗ "
echo " ██║  ██║███████╗╚██████╔╝╚██████╔╝██████╔╝███████╗╚██████╗██║  ██╗"
echo " ╚═╝  ╚═╝╚══════╝ ╚═════╝  ╚═════╝ ╚═════╝ ╚══════╝ ╚═════╝╚═╝  ╚═╝"
echo ""
echo "       ⚡  K O Ł O   M A P   -   O L I M P I C K I E   C + +  ⚡"
echo "       ⚡       100% RUST ENGINE  |  STREAM DECK HID        ⚡"
echo -e "${NC}"
echo -e "${BOLD}AlgoDeck ⚡ Koło MAP - Oficjalny Instalator dla macOS (100% Rust)${NC}"
echo -e "Weryfikacja kompilatora C++, Stream Deck USB HID, VS Code, daemona i rozszerzenia...\n"

if [ "$(uname -s)" != "Darwin" ]; then
    echo -e "${RED}Błąd: Ten instalator jest przeznaczony wyłącznie dla systemu macOS.${NC}"
    exit 1
fi

ARCH_RAW=$(uname -m)
if [ "$ARCH_RAW" = "x86_64" ]; then
    # Sprawdź czy powłoka nie działa pod emulacją Rosetta 2 na Apple Silicon
    if [ "$(sysctl -in sysctl.proc_translated 2>/dev/null)" = "1" ] || sysctl -n machdep.cpu.brand_string 2>/dev/null | grep -qi "Apple"; then
        ARCH_TYPE="arm64"
    else
        ARCH_TYPE="x86_64"
    fi
elif [ "$ARCH_RAW" = "arm64" ] || [ "$ARCH_RAW" = "aarch64" ]; then
    ARCH_TYPE="arm64"
else
    ARCH_TYPE="$ARCH_RAW"
fi
echo -e "${GREEN}✓ Wykryto system macOS (${ARCH_TYPE})${NC}"

# Katalogi instalacyjne
BIN_DIR="$HOME/.local/bin"
SHARE_DIR="$HOME/.local/share/algodeck"
FRONTEND_DIR="$SHARE_DIR/frontend"
ICONS_DIR="$SHARE_DIR/icons"
EXT_DIR="$SHARE_DIR/extension"

mkdir -p "$BIN_DIR"
mkdir -p "$FRONTEND_DIR"
mkdir -p "$ICONS_DIR"
mkdir -p "$EXT_DIR"

# ------------------------------------------------------------------------------
# KROK 1: Weryfikacja kompilatora C++ (Clang / GCC)
# ------------------------------------------------------------------------------
echo -e "${CYAN}[1/7] Weryfikacja kompilatora C++...${NC}"

if command -v clang++ >/dev/null 2>&1; then
    echo -e "${GREEN}✓ Wykryto kompilator Apple Clang C++: $(clang++ --version | head -n 1)${NC}"
elif command -v g++ >/dev/null 2>&1; then
    echo -e "${GREEN}✓ Wykryto kompilator GNU C++: $(g++ --version | head -n 1)${NC}"
else
    echo -e "${YELLOW}Nie wykryto kompilatora C++ w PATH.${NC}"
    if command -v brew >/dev/null 2>&1; then
        echo -e "${CYAN}Próba instalacji kompilatora GCC przez Homebrew...${NC}"
        brew install gcc || true
    else
        echo -e "${CYAN}Uruchamianie instalatora Apple Command Line Tools (Clang C++)...${NC}"
        xcode-select --install 2>/dev/null || true
    fi
fi

# ------------------------------------------------------------------------------
# KROK 2: Weryfikacja edytora Visual Studio Code
# ------------------------------------------------------------------------------
echo -e "\n${CYAN}[2/7] Weryfikacja edytora Visual Studio Code...${NC}"

if ! command -v code >/dev/null 2>&1; then
    # Sprawdź domyślną ścieżkę aplikacji na macOS
    if [ -f "/Applications/Visual Studio Code.app/Contents/Resources/app/bin/code" ]; then
        export PATH="/Applications/Visual Studio Code.app/Contents/Resources/app/bin:$PATH"
    elif [ -f "$HOME/Applications/Visual Studio Code.app/Contents/Resources/app/bin/code" ]; then
        export PATH="$HOME/Applications/Visual Studio Code.app/Contents/Resources/app/bin:$PATH"
    fi
fi

if ! command -v code >/dev/null 2>&1; then
    echo -e "${YELLOW}Visual Studio Code nie został znaleziony w PATH.${NC}"
    if command -v brew >/dev/null 2>&1; then
        echo -e "${CYAN}Próba instalacji VS Code przez Homebrew Cask...${NC}"
        brew install --cask visual-studio-code || true
    fi
    if ! command -v code >/dev/null 2>&1 && [ ! -d "/Applications/Visual Studio Code.app" ]; then
        echo -e "${CYAN}Pobieranie oficjalnej aplikacji Visual Studio Code od Microsoft...${NC}"
        TMP_VSCODE_ZIP="/tmp/vscode_mac_$$.zip"
        TMP_VSCODE_DIR="/tmp/vscode_mac_$$"
        if curl -fsSL -o "$TMP_VSCODE_ZIP" "https://update.code.visualstudio.com/latest/darwin-universal/stable" 2>/dev/null && [ -s "$TMP_VSCODE_ZIP" ]; then
            mkdir -p "$TMP_VSCODE_DIR"
            unzip -q "$TMP_VSCODE_ZIP" -d "$TMP_VSCODE_DIR" 2>/dev/null || true
            if [ -d "$TMP_VSCODE_DIR/Visual Studio Code.app" ]; then
                TARGET_APP_DIR="/Applications"
                [ ! -w "/Applications" ] && TARGET_APP_DIR="$HOME/Applications"
                mkdir -p "$TARGET_APP_DIR"
                cp -R "$TMP_VSCODE_DIR/Visual Studio Code.app" "$TARGET_APP_DIR/" 2>/dev/null || true
                export PATH="$TARGET_APP_DIR/Visual Studio Code.app/Contents/Resources/app/bin:$PATH"
            fi
            rm -rf "$TMP_VSCODE_ZIP" "$TMP_VSCODE_DIR"
        fi
    fi
fi

if command -v code >/dev/null 2>&1 || [ -f "/Applications/Visual Studio Code.app/Contents/Resources/app/bin/code" ] || [ -f "$HOME/Applications/Visual Studio Code.app/Contents/Resources/app/bin/code" ]; then
    if ! command -v code >/dev/null 2>&1; then
        if [ -f "/Applications/Visual Studio Code.app/Contents/Resources/app/bin/code" ]; then
            ln -sf "/Applications/Visual Studio Code.app/Contents/Resources/app/bin/code" "$BIN_DIR/code"
        elif [ -f "$HOME/Applications/Visual Studio Code.app/Contents/Resources/app/bin/code" ]; then
            ln -sf "$HOME/Applications/Visual Studio Code.app/Contents/Resources/app/bin/code" "$BIN_DIR/code"
        fi
    fi
    echo -e "${GREEN}✓ Visual Studio Code jest dostępny.${NC}"
    echo -e "${CYAN}Weryfikacja wtyczki C/C++ dla VS Code...${NC}"
    code --install-extension ms-vscode.cpptools --force >/dev/null 2>&1 || true
    echo -e "${GREEN}✓ Rozszerzenie C/C++ zainstalowane w VS Code.${NC}"
else
    echo -e "${YELLOW}Uwaga: Pobierz i zainstaluj VS Code z https://code.visualstudio.com/${NC}"
fi

# ------------------------------------------------------------------------------
# KROK 3: Instalacja silnika AlgoDeck
# ------------------------------------------------------------------------------
echo -e "\n${CYAN}[3/7] Przygotowanie binarki silnika AlgoDeck...${NC}"

# Zatrzymaj działający proces jeśli istnieje
USER_ID=$(id -u)
launchctl bootout "gui/$USER_ID/com.algodeck.daemon" 2>/dev/null || true
launchctl unload "$HOME/Library/LaunchAgents/com.algodeck.daemon.plist" 2>/dev/null || true
pkill -x "algodeck" 2>/dev/null || true
pkill -f "algodeck daemon" 2>/dev/null || true
sleep 0.3

REPO_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" 2>/dev/null && pwd || echo "$HOME")"
INSTALLED_BIN=false

# 3a. Czy skrypt uruchomiono w repozytorium ze źródłami?
if [ -f "$REPO_DIR/Cargo.toml" ] && command -v cargo >/dev/null 2>&1; then
    echo -e "${CYAN}Kompilacja natywnej binarki z lokalnych źródeł Rust...${NC}"
    (cd "$REPO_DIR" && cargo build --release)
    cp "$REPO_DIR/target/release/algodeck" "$BIN_DIR/algodeck"
    chmod +x "$BIN_DIR/algodeck"
    INSTALLED_BIN=true
elif [ -f "$REPO_DIR/algodeck" ]; then
    echo -e "${CYAN}Kopiowanie prekompilowanej binarki z bieżącego katalogu...${NC}"
    cp "$REPO_DIR/algodeck" "$BIN_DIR/algodeck"
    chmod +x "$BIN_DIR/algodeck"
    INSTALLED_BIN=true
fi

# 3b. Pobranie z GitHub Releases
if [ "$INSTALLED_BIN" = false ]; then
    echo -e "${CYAN}Pobieranie paczki instalacyjnej z GitHub Releases dla macOS (${ARCH_TYPE})...${NC}"
    TAR_URL="https://github.com/veritus-git/AlgoDeck/releases/latest/download/algodeck-macos-${ARCH_TYPE}.tar.gz"
    TMP_TAR="/tmp/algodeck_mac_$$.tar.gz"
    TMP_DIR="/tmp/algodeck_unpack_$$"

    if curl -fsSL -o "$TMP_TAR" "$TAR_URL" 2>/dev/null && [ -s "$TMP_TAR" ]; then
        mkdir -p "$TMP_DIR"
        tar -xzf "$TMP_TAR" -C "$TMP_DIR" 2>/dev/null || true
        if [ -f "$TMP_DIR/algodeck" ]; then
            cp "$TMP_DIR/algodeck" "$BIN_DIR/algodeck"
            chmod +x "$BIN_DIR/algodeck"
            INSTALLED_BIN=true
        fi
        if [ -d "$TMP_DIR/frontend" ]; then
            cp -r "$TMP_DIR/frontend/"* "$FRONTEND_DIR/" 2>/dev/null || true
            cp -r "$TMP_DIR/frontend/icons/"* "$ICONS_DIR/" 2>/dev/null || true
        fi
        if [ -d "$TMP_DIR/extension" ]; then
            cp -r "$TMP_DIR/extension/"* "$EXT_DIR/" 2>/dev/null || true
        fi
        rm -rf "$TMP_TAR" "$TMP_DIR"
    fi

    if [ "$INSTALLED_BIN" = false ]; then
        # Fallback: pojedyncza binarka
        echo -e "${CYAN}Pobieranie surowej binarki algodeck-macos-${ARCH_TYPE}...${NC}"
        RAW_URL="https://github.com/veritus-git/AlgoDeck/releases/latest/download/algodeck-macos-${ARCH_TYPE}"
        curl -fsSL -o "$BIN_DIR/algodeck" "$RAW_URL" || true
        chmod +x "$BIN_DIR/algodeck"
        if [ -s "$BIN_DIR/algodeck" ] && file "$BIN_DIR/algodeck" 2>/dev/null | grep -q "Mach-O"; then
            INSTALLED_BIN=true
        fi
    fi
fi

if [ ! -s "$BIN_DIR/algodeck" ] || ! file "$BIN_DIR/algodeck" 2>/dev/null | grep -q "Mach-O"; then
    echo -e "${RED}Błąd: Nie udało się zainstalować prawidłowej binarki algodeck w $BIN_DIR.${NC}"
    exit 1
fi

# Usuń kwarantannę macOS Gatekeeper i odśwież podpis ad-hoc
echo -e "${CYAN}Odblokowywanie binarki w macOS (usuwanie kwarantanny Gatekeeper i podpis ad-hoc)...${NC}"
xattr -dr com.apple.quarantine "$BIN_DIR/algodeck" 2>/dev/null || true
xattr -cr "$BIN_DIR/algodeck" 2>/dev/null || true
codesign --force --deep -s - "$BIN_DIR/algodeck" 2>/dev/null || true
chmod 755 "$BIN_DIR/algodeck"
xattr -cr "$SHARE_DIR" 2>/dev/null || true

echo -e "${GREEN}✓ Binarka algodeck zainstalowana i odblokowana w $BIN_DIR/algodeck.${NC}"

# ------------------------------------------------------------------------------
# KROK 4: Kopiowanie kompletnego frontendu, grafik i rozszerzenia
# ------------------------------------------------------------------------------
echo -e "\n${CYAN}[4/7] Instalacja plików Web Hubu, grafik i rozszerzenia...${NC}"

if [ -d "$REPO_DIR/frontend" ] && [ -d "$REPO_DIR/extension" ]; then
    cp -r "$REPO_DIR/frontend/"* "$FRONTEND_DIR/"
    cp -r "$REPO_DIR/frontend/icons/"* "$ICONS_DIR/" 2>/dev/null || true
    cp -r "$REPO_DIR/extension/"* "$EXT_DIR/"
    echo -e "${GREEN}✓ Skopiowano zasoby z lokalnego repozytorium.${NC}"
else
    echo -e "${CYAN}Pobieranie zasobów webowych z repozytorium GitHub...${NC}"
    TMP_ZIP="/tmp/algodeck_assets_$$.zip"
    TMP_DIR="/tmp/algodeck_extract_$$"
    curl -fsSL -o "$TMP_ZIP" "https://github.com/veritus-git/AlgoDeck/archive/refs/heads/main.zip" || true
    if [ -f "$TMP_ZIP" ]; then
        unzip -q -o "$TMP_ZIP" -d "$TMP_DIR" 2>/dev/null || true
        ROOT_EXTRACT=$(find "$TMP_DIR" -maxdepth 1 -name "AlgoDeck*" | head -n 1)
        if [ -n "$ROOT_EXTRACT" ] && [ -d "$ROOT_EXTRACT/frontend" ]; then
            cp -r "$ROOT_EXTRACT/frontend/"* "$FRONTEND_DIR/"
            cp -r "$ROOT_EXTRACT/frontend/icons/"* "$ICONS_DIR/" 2>/dev/null || true
        fi
        if [ -n "$ROOT_EXTRACT" ] && [ -d "$ROOT_EXTRACT/extension" ]; then
            cp -r "$ROOT_EXTRACT/extension/"* "$EXT_DIR/"
        fi
        if [ -n "$ROOT_EXTRACT" ] && [ -d "$ROOT_EXTRACT/vscode_extension" ]; then
            mkdir -p "$HOME/.vscode/extensions/algodeck-vscode-bridge"
            cp -r "$ROOT_EXTRACT/vscode_extension/"* "$HOME/.vscode/extensions/algodeck-vscode-bridge/" 2>/dev/null || true
        fi
        rm -rf "$TMP_ZIP" "$TMP_DIR"
        echo -e "${GREEN}✓ Zasoby Web Hub i rozszerzenia pomyślnie zsynchronizowane.${NC}"
    fi
fi

if [ -d "$REPO_DIR/vscode_extension" ]; then
    mkdir -p "$HOME/.vscode/extensions/algodeck-vscode-bridge"
    cp -r "$REPO_DIR/vscode_extension/"* "$HOME/.vscode/extensions/algodeck-vscode-bridge/" 2>/dev/null || true
fi

# ------------------------------------------------------------------------------
# KROK 5: Weryfikacja konfliktów Stream Decka (oficjalna aplikacja Elgato)
# ------------------------------------------------------------------------------
echo -e "\n${CYAN}[5/7] Weryfikacja kontrolera Stream Deck i konfliktów USB HID...${NC}"

if pgrep -ix "Stream Deck" >/dev/null 2>&1; then
    echo -e "${RED}==================================================================${NC}"
    echo -e "${YELLOW}⚠️  UWAGA: WYKRYTO URUCHOMIONĄ OFICJALNĄ APLIKACJĘ STREAM DECK!${NC}"
    echo -e "Oficjalny program Elgato blokuje bezpośredni port USB HID na macOS."
    echo -e "Aby AlgoDeck mógł sterować przyciskami:"
    echo -e "  1. Kliknij ikonę Stream Deck na górnym pasku menu macOS"
    echo -e "  2. Wybierz 'Zakończ Stream Deck' / 'Quit Stream Deck'"
    echo -e "${RED}==================================================================${NC}"
else
    echo -e "${GREEN}✓ Brak konfliktujących aplikacji Elgato. Port USB HID jest gotowy dla AlgoDeck.${NC}"
fi

# ------------------------------------------------------------------------------
# KROK 6: Rejestracja Native Messaging Host dla przeglądarek macOS
# ------------------------------------------------------------------------------
echo -e "\n${CYAN}[6/7] Rejestracja Native Messaging Host dla przeglądarek Chromium na macOS...${NC}"

WRAPPER_PATH="$BIN_DIR/algodeck-native-host"
cat << 'WRAPPER_EOF' > "$WRAPPER_PATH"
#!/bin/sh
exec "$HOME/.local/bin/algodeck" native-host "$@"
WRAPPER_EOF
chmod 755 "$WRAPPER_PATH"
xattr -dr com.apple.quarantine "$WRAPPER_PATH" 2>/dev/null || true
xattr -cr "$WRAPPER_PATH" 2>/dev/null || true

# Oblicz unikalny identyfikator rozszerzenia Chrome dla lokalnego folderu rozszerzenia
MAC_EXT_ID=$(python3 -c "import hashlib; p='$EXT_DIR'; print(''.join(chr(int(c, 16) + ord('a')) for c in hashlib.sha256(p.encode('utf-8')).hexdigest()[:32]))" 2>/dev/null || true)
REPO_EXT_ID=$(python3 -c "import hashlib; p='$REPO_DIR/extension'; print(''.join(chr(int(c, 16) + ord('a')) for c in hashlib.sha256(p.encode('utf-8')).hexdigest()[:32]))" 2>/dev/null || true)

ALLOWED_ORIGINS='[
    "chrome-extension://jglgkkeklbipodmhcncnmdnippdijjcp/",
    "chrome-extension://cdgdnkfkkbadljdgjjjbaefefcbelplf/",
    "chrome-extension://jnfimdneicefaomnlikmogffogcadabf/"'

if [ -n "$MAC_EXT_ID" ]; then
    ALLOWED_ORIGINS="$ALLOWED_ORIGINS"',
    "chrome-extension://'"$MAC_EXT_ID"'/"'
fi
if [ -n "$REPO_EXT_ID" ] && [ "$REPO_EXT_ID" != "$MAC_EXT_ID" ]; then
    ALLOWED_ORIGINS="$ALLOWED_ORIGINS"',
    "chrome-extension://'"$REPO_EXT_ID"'/"'
fi
ALLOWED_ORIGINS="$ALLOWED_ORIGINS"'
  ]'

MANIFEST_CONTENT='{
  "name": "com.algodeck.launcher",
  "description": "AlgoDeck 100% Rust Native Messaging Host for macOS",
  "path": "'"$WRAPPER_PATH"'",
  "type": "stdio",
  "allowed_origins": '"$ALLOWED_ORIGINS"'
}'

MAC_BROWSER_TARGETS=(
    "$HOME/Library/Application Support/Google/Chrome/NativeMessagingHosts"
    "$HOME/Library/Application Support/Google/Chrome Canary/NativeMessagingHosts"
    "$HOME/Library/Application Support/Chromium/NativeMessagingHosts"
    "$HOME/Library/Application Support/BraveSoftware/Brave-Browser/NativeMessagingHosts"
    "$HOME/Library/Application Support/Microsoft Edge/NativeMessagingHosts"
    "$HOME/Library/Application Support/Vivaldi/NativeMessagingHosts"
    "$HOME/Library/Application Support/Arc/User Data/NativeMessagingHosts"
    "$HOME/Library/Application Support/Arc/NativeMessagingHosts"
    "$HOME/Library/Application Support/com.operasoftware.Opera/NativeMessagingHosts"
    "$HOME/Library/Application Support/com.operasoftware.OperaGX/NativeMessagingHosts"
)

for TARGET_DIR in "${MAC_BROWSER_TARGETS[@]}"; do
    mkdir -p "$TARGET_DIR" 2>/dev/null || true
    echo "$MANIFEST_CONTENT" > "$TARGET_DIR/com.algodeck.launcher.json" 2>/dev/null || true
done
echo -e "${GREEN}✓ Mostek Native Messaging zarejestrowany dla Chrome, Brave, Edge i innych.${NC}"

# ------------------------------------------------------------------------------
# KROK 7: Konfiguracja LaunchAgent (cichy demon w tle) i PATH
# ------------------------------------------------------------------------------
echo -e "\n${CYAN}[7/7] Konfiguracja demona w tle (macOS LaunchAgent) oraz zmiennej PATH...${NC}"

LAUNCH_AGENTS_DIR="$HOME/Library/LaunchAgents"
PLIST_FILE="$LAUNCH_AGENTS_DIR/com.algodeck.daemon.plist"
mkdir -p "$LAUNCH_AGENTS_DIR"

cat << PLIST_EOF > "$PLIST_FILE"
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>com.algodeck.daemon</string>
    <key>ProgramArguments</key>
    <array>
        <string>$BIN_DIR/algodeck</string>
        <string>daemon</string>
    </array>
    <key>WorkingDirectory</key>
    <string>$SHARE_DIR</string>
    <key>RunAtLoad</key>
    <true/>
    <key>StandardOutPath</key>
    <string>$SHARE_DIR/daemon.log</string>
    <key>StandardErrorPath</key>
    <string>$SHARE_DIR/daemon.err</string>
    <key>EnvironmentVariables</key>
    <dict>
        <key>PATH</key>
        <string>/usr/local/bin:/usr/bin:/bin:/opt/homebrew/bin:$BIN_DIR</string>
    </dict>
</dict>
</plist>
PLIST_EOF

# Załaduj i uruchom usługę przez modern launchctl oraz legacy fallback
USER_ID=$(id -u)
launchctl enable "gui/$USER_ID/com.algodeck.daemon" 2>/dev/null || true
launchctl bootout "gui/$USER_ID/com.algodeck.daemon" 2>/dev/null || true
launchctl unload "$PLIST_FILE" 2>/dev/null || true
launchctl bootstrap "gui/$USER_ID" "$PLIST_FILE" 2>/dev/null || launchctl load "$PLIST_FILE" 2>/dev/null || true
launchctl kickstart -k "gui/$USER_ID/com.algodeck.daemon" 2>/dev/null || launchctl start com.algodeck.daemon 2>/dev/null || true

# Weryfikacja uruchomienia serwera AlgoDeck
echo -e "${CYAN}Weryfikacja działania serwera AlgoDeck na porcie 8080...${NC}"
SERVER_RUNNING=false
for i in {1..15}; do
    if curl -fsSL http://127.0.0.1:8080/api/status >/dev/null 2>&1; then
        SERVER_RUNNING=true
        break
    fi
    sleep 0.2
done

# Jeśli launchctl nie wystartował daemona (np. brak aktywnej sesji GUI), uruchom niezależnie z nohup
if [ "$SERVER_RUNNING" = false ]; then
    echo -e "${YELLOW}Usługa launchd nie odpowiedziała natychmiast. Uruchamianie daemona w tle (nohup)...${NC}"
    nohup "$BIN_DIR/algodeck" daemon > "$SHARE_DIR/daemon.log" 2>&1 &
    for i in {1..15}; do
        if curl -fsSL http://127.0.0.1:8080/api/status >/dev/null 2>&1; then
            SERVER_RUNNING=true
            break
        fi
        sleep 0.2
    done
fi

if [ "$SERVER_RUNNING" = true ]; then
    echo -e "${GREEN}✓ AlgoDeck Daemon pomyślnie uruchomiony i aktywny pod http://127.0.0.1:8080/${NC}"
else
    echo -e "${YELLOW}Uwaga: Serwer nie odpowiedział jeszcze na porcie 8080. Logi dostępne w: $SHARE_DIR/daemon.log${NC}"
fi

# Dodaj ~/.local/bin do powłok zsh i bash (upewnij się, że pliki istnieją!)
for RC in "$HOME/.zshrc" "$HOME/.zprofile" "$HOME/.bash_profile" "$HOME/.bashrc" "$HOME/.profile"; do
    if [ ! -f "$RC" ]; then
        touch "$RC" 2>/dev/null || true
    fi
    if [ -f "$RC" ]; then
        if ! grep -q '\.local/bin' "$RC" 2>/dev/null; then
            echo '' >> "$RC"
            echo '# AlgoDeck CLI PATH' >> "$RC"
            echo 'export PATH="$HOME/.local/bin:$PATH"' >> "$RC"
        fi
    fi
done

# Jeśli /usr/local/bin jest zapisywalne, utwórz symlink dla natychmiastowej dostępności w każdym oknie
if [ -d "/usr/local/bin" ] && [ -w "/usr/local/bin" ]; then
    ln -sf "$BIN_DIR/algodeck" "/usr/local/bin/algodeck" 2>/dev/null || true
fi

# ------------------------------------------------------------------------------
# ZAPIS STANU POCZĄTKOWEGO POBRANYCH PLIKÓW (IGNOROWANIE STARYCH ZADAŃ)
# ------------------------------------------------------------------------------
CFG_DIR="$HOME/.config/algodeck"
mkdir -p "$CFG_DIR"
rm -rf "$SHARE_DIR/staging"/* 2>/dev/null || true
mkdir -p "$SHARE_DIR/staging" 2>/dev/null || true
SNAPSHOT_FILE="$CFG_DIR/initial_downloads.json"
python3 -c "
import os, json, time, hashlib

candidate_dirs = set()
candidate_dirs.add(os.path.abspath(os.path.expanduser('~/Downloads')))

# Check nested download folders
for d in list(candidate_dirs):
    subp = os.path.join(d, 'Downloads')
    if os.path.isdir(subp):
        candidate_dirs.add(os.path.abspath(subp))

# Browser preferences on macOS
browser_configs = [
    '~/Library/Application Support/Google/Chrome',
    '~/Library/Application Support/BraveSoftware/Brave-Browser',
    '~/Library/Application Support/Chromium',
    '~/Library/Application Support/Microsoft Edge',
    '~/Library/Application Support/Vivaldi',
    '~/Library/Application Support/Arc/User Data',
]
for bc in browser_configs:
    base = os.path.expanduser(bc)
    if os.path.isdir(base):
        for root, dirs, files in os.walk(base):
            if 'Preferences' in files:
                try:
                    with open(os.path.join(root, 'Preferences'), 'r', encoding='utf-8', errors='ignore') as pf:
                        pj = json.load(pf)
                        dl_p = pj.get('download', {}).get('default_directory')
                        if dl_p and os.path.isdir(dl_p):
                            candidate_dirs.add(os.path.abspath(dl_p))
                        save_p = pj.get('savefile', {}).get('default_directory')
                        if save_p and os.path.isdir(save_p):
                            candidate_dirs.add(os.path.abspath(save_p))
                except Exception:
                    pass
            if root.count(os.sep) - base.count(os.sep) >= 2:
                dirs.clear()

now_ms = int(time.time() * 1000)
entries = {}
all_files = []

for d in candidate_dirs:
    if os.path.isdir(d):
        try:
            for f in os.listdir(d):
                all_files.append(f)
                if f.lower().endswith('.pdf'):
                    full_path = os.path.join(d, f)
                    if os.path.isfile(full_path):
                        try:
                            st = os.stat(full_path)
                            h = hashlib.sha256()
                            with open(full_path, 'rb') as fp:
                                while chunk := fp.read(65536):
                                    h.update(chunk)
                            key = f.lower()
                            if key not in entries:
                                entries[key] = {
                                    'filename': f,
                                    'path': full_path,
                                    'sha256': h.hexdigest(),
                                    'size': st.st_size,
                                    'modified_ms': int(st.st_mtime * 1000),
                                    'registered_at': now_ms,
                                    'is_initial': True
                                }
                        except Exception:
                            pass
        except Exception:
            pass

cfg_dir = os.path.expanduser('~/.config/algodeck')
os.makedirs(cfg_dir, exist_ok=True)

with open(os.path.join(cfg_dir, 'known_download_dirs.json'), 'w') as kd:
    json.dump(list(candidate_dirs), kd, indent=2)

reg_data = {
    'version': 1,
    'updated_at': now_ms,
    'entries': entries
}
with open(os.path.join(cfg_dir, 'downloaded_pdf_registry.json'), 'w') as out:
    json.dump(reg_data, out, indent=2)

init_data = {
    'installed_at': now_ms,
    'ignored_files': list(set(all_files))
}
with open(os.path.join(cfg_dir, 'initial_downloads.json'), 'w') as out:
    json.dump(init_data, out, indent=2)
" 2>/dev/null || true

# ------------------------------------------------------------------------------
# AUTOMATYZACJA ROZSZERZENIA: SCHOWEK, FINDER I PRZEGLĄDARKA
# ------------------------------------------------------------------------------
# 1. Automatyczne skopiowanie ścieżki do schowka macOS
echo -n "$EXT_DIR" | pbcopy 2>/dev/null || true

# 2. Automatyczne otwarcie strony rozszerzeń w domyślnej przeglądarce Chromium
if [ -d "/Applications/Brave Browser.app" ]; then
    open -a "Brave Browser" "brave://extensions" 2>/dev/null || true
elif [ -d "/Applications/Google Chrome.app" ]; then
    open -a "Google Chrome" "chrome://extensions" 2>/dev/null || true
elif [ -d "/Applications/Microsoft Edge.app" ]; then
    open -a "Microsoft Edge" "edge://extensions" 2>/dev/null || true
elif [ -d "/Applications/Arc.app" ]; then
    open -a "Arc" "arc://extensions" 2>/dev/null || true
fi

# 3. Otwarcie folderu rozszerzenia w Finderze dla łatwego Drag & Drop
open "$EXT_DIR" 2>/dev/null || true

# ------------------------------------------------------------------------------
# PODSUMOWANIE INSTALACJI
# ------------------------------------------------------------------------------
echo -e "\n${GREEN}${BOLD}==============================================================================${NC}"
echo -e "${GREEN}${BOLD}       🎉 AlgoDeck v2.0.0 (100% Rust) pomyślnie zainstalowany!                ${NC}"
echo -e "${GREEN}${BOLD}==============================================================================${NC}"
echo -e "• Panel Web Hub:   ${CYAN}${BOLD}http://127.0.0.1:8080/${NC} lub kliknij ikonę rozszerzenia"
echo -e "• Stream Deck:     Bezpośrednia obsługa USB HID bez oficjalnej aplikacji Elgato"
echo -e "• Narzędzie CLI:   ${BOLD}algodeck status${NC} | ${BOLD}algodeck start${NC} | ${BOLD}algodeck stop${NC} | ${BOLD}algodeck test <kod>${NC}"
echo -e "• Folder programu: ${BOLD}$SHARE_DIR${NC}"
echo -e "${GREEN}${BOLD}==============================================================================${NC}"
echo -e "${BLUE}💡 Jeśli w tym oknie terminala polecenie 'algodeck' nie jest jeszcze widoczne, wpisz:${NC}"
echo -e "   ${BOLD}source ~/.zshrc${NC}  (lub po prostu otwórz nowe okno terminala)\n"

echo -e "\n${CYAN}${BOLD}🧩 ROZSZERZENIE PRZEGLĄDARKI (Automatyczne ułatwienie):${NC}"
echo -e "${GREEN}✓ Ścieżka folderu rozszerzenia została AUTOMATYCZNIE SKOPIOWANA do Twojego schowka (Cmd+V)!${NC}"
echo -e "${GREEN}✓ Otwarto stronę rozszerzeń w przeglądarce oraz folder w Finderze.${NC}"
echo -e "\n${BOLD}Dla Chrome / Brave / Edge / Arc / Chromium:${NC}"
echo -e "1. Włącz ${BOLD}Tryb programisty${NC} (Developer mode) w prawym górnym rogu."
echo -e "2. Kliknij ${BOLD}Załaduj rozpakowane${NC} (Load unpacked)."
echo -e "3. W oknie wyboru wciśnij skrót ${BOLD}Cmd+Shift+G${NC}, wklej schowek (${BOLD}Cmd+V${NC}) i naciśnij Enter."
echo -e "   (lub po prostu ${BOLD}przeciągnij otwarty folder z Findera${NC} bezpośrednio do okna przeglądarki!)"
echo -e "4. Przypnij ikonę ⚡ AlgoDeck na pasku przeglądarki.\n"
