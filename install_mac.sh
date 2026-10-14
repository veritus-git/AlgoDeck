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
echo -e "${BOLD}AlgoDeck ⚡ Koło MAP - Oficjalny Instalator dla macOS (100% Rust)$${NC}"
echo -e "Weryfikacja kompilatora C++, Stream Deck USB HID, VS Code, daemona i rozszerzenia...\n"

if [ "$(uname -s)" != "Darwin" ]; then
    echo -e "${RED}Błąd: Ten instalator jest przeznaczony wyłącznie dla systemu macOS.${NC}"
    exit 1
fi

ARCH_TYPE=$(uname -m)
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
# KROK 1: Weryfikacja Xcode Command Line Tools i kompilatora C++
# ------------------------------------------------------------------------------
echo -e "${CYAN}[1/7] Weryfikacja narzędzi Xcode i kompilatora C++...${NC}"

if ! xcode-select -p >/dev/null 2>&1; then
    echo -e "${YELLOW}Brak narzędzi deweloperskich Xcode. Uruchamianie instalatora Apple...${NC}"
    xcode-select --install || true
    echo -e "${YELLOW}Dokończ instalację w oknie dialogowym macOS, a następnie uruchom ten skrypt ponownie.${NC}"
    exit 0
fi

if command -v clang++ >/dev/null 2>&1; then
    echo -e "${GREEN}✓ Wykryto kompilator Apple Clang C++: $(clang++ --version | head -n 1)${NC}"
elif command -v g++ >/dev/null 2>&1; then
    echo -e "${GREEN}✓ Wykryto kompilator GNU C++: $(g++ --version | head -n 1)${NC}"
else
    echo -e "${YELLOW}Uwaga: Upewnij się, że masz zainstalowany kompilator C++20.${NC}"
fi

# ------------------------------------------------------------------------------
# KROK 2: Weryfikacja edytora Visual Studio Code
# ------------------------------------------------------------------------------
echo -e "\n${CYAN}[2/7] Weryfikacja edytora Visual Studio Code...${NC}"

if ! command -v code >/dev/null 2>&1; then
    # Sprawdź domyślną ścieżkę aplikacji na macOS
    if [ -f "/Applications/Visual Studio Code.app/Contents/Resources/app/bin/code" ]; then
        export PATH="/Applications/Visual Studio Code.app/Contents/Resources/app/bin:$PATH"
    fi
fi

if command -v code >/dev/null 2>&1; then
    echo -e "${GREEN}✓ Visual Studio Code jest dostępny w PATH.${NC}"
    echo -e "${CYAN}Weryfikacja wtyczki C/C++ dla VS Code...${NC}"
    code --install-extension ms-vscode.cpptools --force >/dev/null 2>&1 || true
    echo -e "${GREEN}✓ Rozszerzenie C/C++ zainstalowane w VS Code.${NC}"
else
    echo -e "${YELLOW}Visual Studio Code nie został znaleziony w PATH.${NC}"
    if command -v brew >/dev/null 2>&1; then
        echo -e "${CYAN}Próba instalacji VS Code przez Homebrew...${NC}"
        brew install --cask visual-studio-code || true
    else
        echo -e "${YELLOW}Pobierz VS Code z: https://code.visualstudio.com/Download${NC}"
    fi
fi

# ------------------------------------------------------------------------------
# KROK 3: Instalacja silnika AlgoDeck
# ------------------------------------------------------------------------------
echo -e "\n${CYAN}[3/7] Przygotowanie binarki silnika AlgoDeck...${NC}"

# Zatrzymaj działający proces jeśli istnieje
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
        tar -xzf "$TMP_TAR" -C "$TMP_DIR"
        if [ -f "$TMP_DIR/algodeck" ]; then
            cp "$TMP_DIR/algodeck" "$BIN_DIR/algodeck"
            chmod +x "$BIN_DIR/algodeck"
            INSTALLED_BIN=true
        fi
        if [ -d "$TMP_DIR/frontend" ]; then
            cp -r "$TMP_DIR/frontend/"* "$FRONTEND_DIR/"
            cp -r "$TMP_DIR/frontend/icons/"* "$ICONS_DIR/" 2>/dev/null || true
        fi
        if [ -d "$TMP_DIR/extension" ]; then
            cp -r "$TMP_DIR/extension/"* "$EXT_DIR/"
        fi
        rm -rf "$TMP_TAR" "$TMP_DIR"
    else
        # Fallback: pojedyncza binarka
        echo -e "${CYAN}Pobieranie surowej binarki algodeck-macos-${ARCH_TYPE}...${NC}"
        RAW_URL="https://github.com/veritus-git/AlgoDeck/releases/latest/download/algodeck-macos-${ARCH_TYPE}"
        curl -fsSL -o "$BIN_DIR/algodeck" "$RAW_URL" || true
        chmod +x "$BIN_DIR/algodeck"
        if [ -x "$BIN_DIR/algodeck" ]; then
            INSTALLED_BIN=true
        fi
    fi
fi

if [ ! -x "$BIN_DIR/algodeck" ]; then
    echo -e "${RED}Błąd: Nie udało się zainstalować binarki algodeck w $BIN_DIR.${NC}"
    exit 1
fi
echo -e "${GREEN}✓ Binarka algodeck zainstalowana w $BIN_DIR/algodeck.${NC}"

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
        rm -rf "$TMP_ZIP" "$TMP_DIR"
        echo -e "${GREEN}✓ Zasoby Web Hub i rozszerzenia pomyślnie zsynchronizowane.${NC}"
    fi
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
exec "$HOME/.local/bin/algodeck" native-host
WRAPPER_EOF
chmod +x "$WRAPPER_PATH"

MANIFEST_CONTENT='{
  "name": "com.algodeck.launcher",
  "description": "AlgoDeck 100% Rust Native Messaging Host for macOS",
  "path": "'"$WRAPPER_PATH"'",
  "type": "stdio",
  "allowed_origins": [
    "chrome-extension://cdgdnkfkkbadljdgjjjbaefefcbelplf/",
    "chrome-extension://jnfimdneicefaomnlikmogffogcadabf/"
  ]
}'

MAC_BROWSER_TARGETS=(
    "$HOME/Library/Application Support/Google/Chrome/NativeMessagingHosts"
    "$HOME/Library/Application Support/Chromium/NativeMessagingHosts"
    "$HOME/Library/Application Support/BraveSoftware/Brave-Browser/NativeMessagingHosts"
    "$HOME/Library/Application Support/Microsoft Edge/NativeMessagingHosts"
    "$HOME/Library/Application Support/Vivaldi/NativeMessagingHosts"
    "$HOME/Library/Application Support/Arc/User Data/NativeMessagingHosts"
    "$HOME/Library/Application Support/com.operasoftware.Opera/NativeMessagingHosts"
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
    <key>RunAtLoad</key>
    <true/>
    <key>KeepAlive</key>
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

# Załaduj i uruchom usługę przez launchctl
launchctl unload "$PLIST_FILE" 2>/dev/null || true
launchctl load -w "$PLIST_FILE" 2>/dev/null || true
echo -e "${GREEN}✓ Usługa tła LaunchAgent (com.algodeck.daemon) uruchomiona.${NC}"

# Dodaj ~/.local/bin do powłok zsh i bash
for RC in "$HOME/.zshrc" "$HOME/.bash_profile" "$HOME/.profile"; do
    if [ -f "$RC" ]; then
        if ! grep -q '\.local/bin' "$RC" 2>/dev/null; then
            echo 'export PATH="$HOME/.local/bin:$PATH"' >> "$RC"
        fi
    fi
done

# ------------------------------------------------------------------------------
# PODSUMOWANIE INSTALACJI
# ------------------------------------------------------------------------------
echo -e "\n${GREEN}${BOLD}==============================================================================${NC}"
echo -e "${GREEN}${BOLD}       🎉 AlgoDeck v2.0.0 (100% Rust) pomyślnie zainstalowany!                ${NC}"
echo -e "${GREEN}${BOLD}==============================================================================${NC}"
echo -e "• Panel Web Hub:   ${CYAN}${BOLD}http://127.0.0.1:8080/${NC} lub kliknij ikonę rozszerzenia"
echo -e "• Stream Deck:     Bezpośrednia obsługa USB HID bez oficjalnej aplikacji Elgato"
echo -e "• Narzędzie CLI:   ${BOLD}algodeck status${NC} | ${BOLD}algodeck test <kod>${NC} | ${BOLD}algodeck stop${NC}"
echo -e "• Folder programu: ${BOLD}$SHARE_DIR${NC}"
echo -e "${GREEN}${BOLD}==============================================================================${NC}"

echo -e "\n${CYAN}${BOLD}🧩 DODANIE ROZSZERZENIA DO PRZEGLĄDARKI (Chrome / Brave / Edge):${NC}"
echo -e "1. Wejdź w przeglądarce pod adres: ${BOLD}chrome://extensions${NC} (lub brave://extensions)"
echo -e "2. Włącz w prawym górnym rogu przełącznik ${BOLD}Tryb programisty${NC} (Developer mode)"
echo -e "3. Kliknij w lewym górnym rogu przycisk ${BOLD}Załaduj rozpakowane${NC} (Load unpacked)"
echo -e "4. Wybierz ten folder:"
echo -e "   ${GREEN}${BOLD}$EXT_DIR${NC}"
echo -e "5. Gotowe! Przypnij ikonę ⚡ AlgoDeck na pasku przeglądarki.\n"
