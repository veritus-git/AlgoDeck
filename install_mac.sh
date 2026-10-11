#!/usr/bin/env bash
# ==============================================================================
# AlgoDeck ⚡ Koło MAP - Oficjalny Instalator dla macOS (Apple Silicon & Intel)
# Instalacja One-Liner:
# curl -fsSL https://raw.githubusercontent.com/veritus-git/AlgoDeck/main/install_mac.sh | bash
# ==============================================================================

set -e

# Kolory konsoli
RED='\033[0;31m'
GREEN='\033[0;32m'
CYAN='\033[0;36m'
YELLOW='\033[1;33m'
BOLD='\033[1m'
NC='\033[0m'

echo -e "\n${CYAN}${BOLD}==============================================================================${NC}"
echo -e "${CYAN}${BOLD}       ⚡ AlgoDeck (Koło MAP) — Instalator macOS (Apple Silicon & Intel)      ${NC}"
echo -e "${CYAN}${BOLD}==============================================================================${NC}\n"

# 1. Sprawdzenie systemu
OS_TYPE=$(uname -s)
if [ "$OS_TYPE" != "Darwin" ]; then
    echo -e "${RED}Błąd: Ten instalator jest przeznaczony wyłącznie dla systemu macOS.${NC}"
    exit 1
fi

ARCH_TYPE=$(uname -m)
echo -e "${GREEN}✓ Wykryto system macOS (${ARCH_TYPE})${NC}"

# 2. Narzędzia programistyczne Xcode Command Line Tools
echo -e "\n${CYAN}[1/6] Sprawdzanie kompilatorów C++ (Clang / Apple LLVM)...${NC}"
if ! xcode-select -p >/dev/null 2>&1; then
    echo -e "${YELLOW}Instalacja narzędzi Xcode Command Line Tools... Zaakceptuj monit w systemie.${NC}"
    xcode-select --install || true
    echo -e "${YELLOW}Po zakończeniu instalacji narzędzi uruchom ten skrypt ponownie.${NC}"
    exit 0
else
    echo -e "${GREEN}✓ Narzędzia programistyczne Xcode zainstalowane (clang, clang++).${NC}"
fi

# 3. Sprawdzanie Visual Studio Code
echo -e "\n${CYAN}[2/6] Sprawdzanie Visual Studio Code...${NC}"
if [ -d "/Applications/Visual Studio Code.app" ] || which code >/dev/null 2>&1; then
    echo -e "${GREEN}✓ Visual Studio Code jest zainstalowany.${NC}"
    if which code >/dev/null 2>&1; then
        code --install-extension ms-vscode.cpptools --force >/dev/null 2>&1 || true
    fi
else
    echo -e "${YELLOW}Wskazówka: Zainstaluj Visual Studio Code z https://code.visualstudio.com/${NC}"
fi

# 4. Sprawdzanie Pythona 3
echo -e "\n${CYAN}[3/6] Konfiguracja środowiska Python 3...${NC}"
PYTHON_CMD="python3"
if ! which $PYTHON_CMD >/dev/null 2>&1; then
    echo -e "${RED}Błąd: Python 3 nie został znaleziony. Zainstaluj go przez Homebrew (brew install python3) lub z python.org.${NC}"
    exit 1
fi
echo -e "${GREEN}✓ Używam: $($PYTHON_CMD --version)${NC}"

# 5. Klonowanie / Pobieranie AlgoDeck
INSTALL_DIR="$HOME/.local/share/algodeck"
echo -e "\n${CYAN}[4/6] Pobieranie AlgoDeck do $INSTALL_DIR...${NC}"
mkdir -p "$INSTALL_DIR"

if [ -d ".git" ] && [ -f "backend/server.py" ]; then
    # Instalacja z lokalnego folderu repozytorium
    echo -e "Kopiowanie plików z lokalnego katalogu roboczego..."
    cp -R . "$INSTALL_DIR/"
else
    if which git >/dev/null 2>&1; then
        if [ -d "$INSTALL_DIR/.git" ]; then
            cd "$INSTALL_DIR" && git pull origin main || true
        else
            git clone --depth 1 https://github.com/veritus-git/AlgoDeck.git "$INSTALL_DIR"
        fi
    else
        echo -e "${YELLOW}Pobieranie archiwum ZIP...${NC}"
        curl -fsSL https://github.com/veritus-git/AlgoDeck/archive/refs/heads/main.tar.gz | tar -xz -C "$INSTALL_DIR" --strip-components=1
    fi
fi

# Tworzenie wirtualnego środowiska
VENV_DIR="$INSTALL_DIR/venv"
if [ ! -d "$VENV_DIR" ]; then
    $PYTHON_CMD -m venv "$VENV_DIR"
fi
source "$VENV_DIR/bin/activate"
pip install --upgrade pip >/dev/null 2>&1 || true
pip install -r "$INSTALL_DIR/requirements.txt" >/dev/null 2>&1 || true
pip install websockets >/dev/null 2>&1 || true

# 6. Oficjalna integracja z Elgato Stream Deck na macOS
echo -e "\n${CYAN}[5/6] Konfiguracja Elgato Stream Deck (Oficjalny plugin macOS)...${NC}"
ELGATO_PLUGIN_DIR="$HOME/Library/Application Support/com.elgato.StreamDeck/Plugins/com.algodeck.streamdeck.sdPlugin"
mkdir -p "$(dirname "$ELGATO_PLUGIN_DIR")"

if [ -d "$INSTALL_DIR/streamdeck_plugin/com.algodeck.streamdeck.sdPlugin" ]; then
    cp -R "$INSTALL_DIR/streamdeck_plugin/com.algodeck.streamdeck.sdPlugin" "$(dirname "$ELGATO_PLUGIN_DIR")/"
    echo -e "${GREEN}✓ Zainstalowano plugin w: $ELGATO_PLUGIN_DIR${NC}"
fi

# 7. Konfiguracja CLI i skrótów macOS
echo -e "\n${CYAN}[6/6] Konfiguracja komend CLI 'algodeck' w ~/.local/bin...${NC}"
mkdir -p "$HOME/.local/bin"

# Kopiowanie lub pobieranie binarnego GUI w Rust
GUI_MAC_TARGET="$HOME/.local/bin/algodeck-gui"
if [ -f "$INSTALL_DIR/gui/target/release/algodeck-gui" ]; then
    cp "$INSTALL_DIR/gui/target/release/algodeck-gui" "$GUI_MAC_TARGET"
    chmod +x "$GUI_MAC_TARGET"
elif [ -f "$GUI_MAC_TARGET" ]; then
    chmod +x "$GUI_MAC_TARGET"
else
    ARCH=$(uname -m)
    if [ "$ARCH" = "arm64" ]; then
        MAC_BIN="algodeck-gui-macos-arm64"
    else
        MAC_BIN="algodeck-gui-macos-x86_64"
    fi
    echo -e "${CYAN}Pobieranie skompilowanej binarki algodeck-gui ($MAC_BIN) z GitHub Releases...${NC}"
    if curl -sSL -f "https://github.com/veritus-git/AlgoDeck/releases/latest/download/$MAC_BIN" -o "$GUI_MAC_TARGET"; then
        chmod +x "$GUI_MAC_TARGET"
        echo -e "${GREEN}✓ Pobrano natywną binarkę algodeck-gui z GitHub Releases!${NC}"
    elif which cargo >/dev/null 2>&1 && [ -d "$INSTALL_DIR/gui" ]; then
        echo -e "${YELLOW}Kompilacja natywnego GUI AlgoDeck w Rust...${NC}"
        (cd "$INSTALL_DIR/gui" && cargo build --release && cp target/release/algodeck-gui "$GUI_MAC_TARGET" && chmod +x "$GUI_MAC_TARGET") || true
    fi
fi

# Wrapper CLI 'algodeck' dla macOS
cat > "$HOME/.local/bin/algodeck" << 'EOF'
#!/usr/bin/env bash
INSTALL_DIR="$HOME/.local/share/algodeck"
VENV_DIR="$INSTALL_DIR/venv"

ACTION="${1:-start}"

is_running() {
    curl -s --max-time 1 http://127.0.0.1:8080/api/status >/dev/null 2>&1
}

start_daemon() {
    if is_running; then
        echo "⚡ AlgoDeck już działa w tle na porcie 8080."
        curl -s -X POST http://127.0.0.1:8080/api/launch >/dev/null 2>&1 || true
        osascript -e 'display notification "AlgoDeck jest aktywny! Ekran zachęty na Stream Decku." with title "AlgoDeck ⚡"' 2>/dev/null || true
        return 0
    fi

    echo "⚡ Uruchamianie AlgoDeck w tle..."
    mkdir -p "$HOME/.config/algodeck/logs"
    cd "$INSTALL_DIR"
    export PYTHONPATH="$INSTALL_DIR"
    nohup "$VENV_DIR/bin/python3" -m uvicorn backend.server:app --host 127.0.0.1 --port 8080 > "$HOME/.config/algodeck/logs/server.log" 2>&1 &

    for i in {1..30}; do
        if is_running; then break; fi
        sleep 0.1
    done

    curl -s -X POST http://127.0.0.1:8080/api/launch >/dev/null 2>&1 || true
    osascript -e 'display notification "Środowisko olimpijskie uruchomione w tle!" with title "AlgoDeck ⚡"' 2>/dev/null || true

    echo "✅ AlgoDeck został pomyślnie uruchomiony w tle!"
    echo "💡 Możesz zamknąć ten terminal — daemon działa w tle."
}

stop_daemon() {
    echo "⏹ Zatrzymywanie daemona AlgoDeck..."
    pkill -f "uvicorn backend.server:app" >/dev/null 2>&1 || true
    osascript -e 'display notification "Środowisko AlgoDeck zostało zatrzymane." with title "AlgoDeck ⚡"' 2>/dev/null || true
    echo "✓ AlgoDeck zatrzymany."
}

open_gui() {
    TAB="${1:---tab=settings}"
    if [ -x "$HOME/.local/bin/algodeck-gui" ]; then
        "$HOME/.local/bin/algodeck-gui" "$TAB" >/dev/null 2>&1 &
    else
        "$VENV_DIR/bin/python3" "$INSTALL_DIR/scripts/panel_dialog.py" "$TAB" >/dev/null 2>&1 &
    fi
}

case "$ACTION" in
    start) start_daemon ;;
    stop) stop_daemon ;;
    restart) stop_daemon; sleep 1; start_daemon ;;
    status)
        if is_running; then
            echo "🟢 AlgoDeck jest ONLINE (127.0.0.1:8080)"
        else
            echo "🔴 AlgoDeck jest OFFLINE"
        fi
        ;;
    gui|settings) shift || true; open_gui "$@" ;;
    --tab=*) open_gui "$ACTION" ;;
    *)
        if [[ "$ACTION" == --* ]]; then open_gui "$@"; else start_daemon; fi
        ;;
esac
EOF
chmod +x "$HOME/.local/bin/algodeck"

# Dodanie ~/.local/bin do PATH w zshrc
ZSHRC="$HOME/.zshrc"
if [ -f "$ZSHRC" ] && ! grep -q 'export PATH="$HOME/.local/bin:$PATH"' "$ZSHRC"; then
    echo 'export PATH="$HOME/.local/bin:$PATH"' >> "$ZSHRC"
fi

# AppleScript powiadomienie
osascript -e 'display notification "Instalacja AlgoDeck zakończona pomyślnie!" with title "AlgoDeck ⚡"' 2>/dev/null || true

echo -e "\n${GREEN}${BOLD}==============================================================================${NC}"
echo -e "${GREEN}${BOLD}       🎉 INSTALACJA ALGODECK NA MACOS ZAKOŃCZONA SUKCESEM!                  ${NC}"
echo -e "${GREEN}${BOLD}==============================================================================${NC}\n"
echo -e "🚀 ${BOLD}Jak korzystać z AlgoDeck na macOS:${NC}"
echo -e "  1. Wpisz w terminalu: ${BOLD}algodeck${NC} (uruchamia cichy proces w tle + ekran zachęty)"
echo -e "  2. Otwórz panel ustawień Rust: ${BOLD}algodeck settings${NC}"
echo -e "  3. W oficjalnej aplikacji Elgato Stream Deck dodaj profil AlgoDeck z listy akcji."
echo -e "  4. Zainstaluj rozszerzenie w Chrome/Brave z folderu: ${CYAN}$INSTALL_DIR/extension${NC}\n"
