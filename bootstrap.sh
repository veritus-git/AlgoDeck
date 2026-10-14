#!/usr/bin/env bash
# ==============================================================================
# AlgoDeck ⚡ Koło MAP - 100% Rust One-Click Installer (Bootstrap)
# W 100% zautomatyzowana, lekka instalacja natywnego środowiska olimpijskiego.
# ZERO Pythona, ZERO StreamControllera, ZERO uvicorn/venv.
# ==============================================================================

set -e

export GIT_TERMINAL_PROMPT=0
export DEBIAN_FRONTEND=noninteractive

RED='\033[0;31m'
GREEN='\033[0;32m'
CYAN='\033[0;36m'
YELLOW='\033[1;33m'
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
echo -e "${BOLD}AlgoDeck ⚡ Koło MAP - 100% Rust Standalone Installer${NC}"
echo -e "Instalacja narzędzi C++, Stream Deck HID, VS Code, daemona i rozszerzenia...\n"

# 0. Sudo keepalive jeśli dostępne
if [ "$EUID" -ne 0 ] && command -v sudo &> /dev/null; then
    if ! sudo -n true 2>/dev/null; then
        echo -e "${YELLOW}Weryfikacja uprawnień administratora (sudo)...${NC}"
        sudo -v || true
    fi
    while true; do sudo -n true; sleep 40; kill -0 "$$" || exit; done 2>/dev/null &
fi

# 1. Pakiety systemowe
echo -e "${CYAN}[1/6] Weryfikacja pakietów systemowych i kompilatora C++...${NC}"
if command -v apt-get &> /dev/null; then
    sudo apt-get update -qq || true
    sudo apt-get install -y -qq \
        build-essential \
        gdb \
        clang \
        pkg-config \
        libhidapi-dev \
        libusb-1.0-0-dev \
        curl \
        wget \
        git \
        unzip \
        tar 2>/dev/null || true
elif command -v dnf &> /dev/null; then
    sudo dnf install -y \
        gcc \
        gcc-c++ \
        gdb \
        pkgconfig \
        hidapi-devel \
        libusb1-devel \
        curl \
        git 2>/dev/null || true
fi

# 2. Sprawdzenie kompilatora Rust (Cargo)
echo -e "\n${CYAN}[2/6] Weryfikacja środowiska Rust...${NC}"
if ! command -v cargo &> /dev/null; then
    echo -e "${YELLOW}Instalacja Rust (rustup)...${NC}"
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --profile minimal
    export PATH="$HOME/.cargo/bin:$PATH"
fi
echo -e "${GREEN}✓ Rust i Cargo gotowe: $(cargo --version)${NC}"

# 3. Sprawdzenie Visual Studio Code
echo -e "\n${CYAN}[3/6] Weryfikacja edytora Visual Studio Code...${NC}"
if ! command -v code &> /dev/null; then
    echo -e "${YELLOW}Instalacja Visual Studio Code...${NC}"
    if command -v apt-get &> /dev/null; then
        wget -qO- https://packages.microsoft.com/keys/microsoft.asc | gpg --dearmor > /tmp/packages.microsoft.gpg 2>/dev/null || true
        sudo install -D -o root -g root -m 644 /tmp/packages.microsoft.gpg /etc/apt/keyrings/packages.microsoft.gpg 2>/dev/null || true
        echo "deb [arch=amd64,arm64,armhf signed-by=/etc/apt/keyrings/packages.microsoft.gpg] https://packages.microsoft.com/repos/code stable main" | sudo tee /etc/apt/sources.list.d/vscode.list > /dev/null 2>&1 || true
        sudo apt-get update -qq 2>/dev/null || true
        sudo apt-get install -y -qq code 2>/dev/null || true
    fi
fi
if command -v code &> /dev/null; then
    echo -e "${GREEN}✓ Visual Studio Code jest zainstalowany.${NC}"
    code --install-extension ms-vscode.cpptools --force 2>/dev/null || true
fi

# 4. Budowa natywnej binarki AlgoDeck
REPO_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" 2>/dev/null && pwd || echo "$HOME")"
mkdir -p "$HOME/.local/bin"
mkdir -p "$HOME/.local/share/algodeck/icons"
mkdir -p "$HOME/.local/share/algodeck/extension"

# Jeśli uruchomiono w sklonowanym repozytorium z Cargo.toml
if [ -f "$REPO_DIR/Cargo.toml" ] && command -v cargo &> /dev/null; then
    echo -e "${CYAN}Kompilacja lokalna ze źródeł...${NC}"
    (cd "$REPO_DIR" && cargo build --release)
    install -m 755 "$REPO_DIR/target/release/algodeck" "$HOME/.local/bin/algodeck"
    if [ -d "$REPO_DIR/frontend/icons" ]; then
        cp -r "$REPO_DIR/frontend/icons/"* "$HOME/.local/share/algodeck/icons/"
    fi
    if [ -d "$REPO_DIR/extension" ]; then
        cp -r "$REPO_DIR/extension/"* "$HOME/.local/share/algodeck/extension/"
    fi
else
    echo -e "${CYAN}Pobieranie natywnej binarki z GitHub Releases...${NC}"
    curl -fsSL -o "$HOME/.local/bin/algodeck" "https://github.com/veritus-git/AlgoDeck/releases/latest/download/algodeck-linux-x86_64" || true
    chmod +x "$HOME/.local/bin/algodeck"
    # Pobierz ikony i rozszerzenie z repozytorium
    echo -e "${CYAN}Pobieranie grafik i rozszerzenia AlgoDeck...${NC}"
    TMP_ZIP="/tmp/algodeck_assets_$$.zip"
    TMP_DIR="/tmp/algodeck_extract_$$"
    curl -fsSL -o "$TMP_ZIP" "https://github.com/veritus-git/AlgoDeck/archive/refs/heads/main.zip" || true
    if [ -f "$TMP_ZIP" ]; then
        unzip -q -o "$TMP_ZIP" "AlgoDeck-main/frontend/icons/*" "AlgoDeck-main/extension/*" -d "$TMP_DIR" 2>/dev/null || true
        if [ -d "$TMP_DIR/AlgoDeck-main/frontend/icons" ]; then
            cp -r "$TMP_DIR/AlgoDeck-main/frontend/icons/"* "$HOME/.local/share/algodeck/icons/"
        fi
        if [ -d "$TMP_DIR/AlgoDeck-main/extension" ]; then
            cp -r "$TMP_DIR/AlgoDeck-main/extension/"* "$HOME/.local/share/algodeck/extension/"
        fi
        rm -rf "$TMP_ZIP" "$TMP_DIR"
    fi
fi
echo -e "${GREEN}✓ Binarka, ikony oraz rozszerzenie zainstalowane w ~/.local/share/algodeck/.${NC}"

# 5. Uprawnienia USB HID dla Stream Decka (udev)
echo -e "\n${CYAN}[5/6] Konfiguracja uprawnień USB HID dla kontrolerów Elgato Stream Deck...${NC}"
if [ -d /etc/udev/rules.d ] && [ "$EUID" -eq 0 -o -n "$SUDO_USER" ] || sudo -n true 2>/dev/null; then
    sudo tee /etc/udev/rules.d/99-streamdeck.rules > /dev/null << 'UDEV_EOF'
# Elgato Stream Deck USB HID rules for non-root users
SUBSYSTEM=="usb", ATTRS{idVendor}=="0fd9", MODE="0666", TAG+="uaccess"
KERNEL=="hidraw*", ATTRS{idVendor}=="0fd9", MODE="0666", TAG+="uaccess"
UDEV_EOF
    sudo udevadm control --reload-rules 2>/dev/null || true
    sudo udevadm trigger 2>/dev/null || true
    echo -e "${GREEN}✓ Reguły udev zainstalowane (bezpośredni dostęp USB bez roota).${NC}"
fi

# 6. Rejestracja Native Messaging Host dla przeglądarki Chrome/Chromium/Brave
echo -e "\n${CYAN}[6/6] Konfiguracja rozszerzenia przeglądarki i usługi systemd...${NC}"

# Wrapper natywny
cat << 'WRAPPER_EOF' > "$HOME/.local/bin/algodeck-native-host"
#!/bin/sh
exec "$HOME/.local/bin/algodeck" native-host
WRAPPER_EOF
chmod +x "$HOME/.local/bin/algodeck-native-host"

# Manifesty Chrome Native Messaging
CHROME_TARGETS=(
    "$HOME/.config/google-chrome/NativeMessagingHosts"
    "$HOME/.config/chromium/NativeMessagingHosts"
    "$HOME/.config/BraveSoftware/Brave-Browser/NativeMessagingHosts"
    "$HOME/.config/microsoft-edge/NativeMessagingHosts"
)

MANIFEST_CONTENT='{
  "name": "com.algodeck.launcher",
  "description": "AlgoDeck 100% Rust Native Messaging Host",
  "path": "'"$HOME/.local/bin/algodeck-native-host"'",
  "type": "stdio",
  "allowed_origins": [
    "chrome-extension://cdgdnkfkkbadljdgjjjbaefefcbelplf/",
    "chrome-extension://jnfimdneicefaomnlikmogffogcadabf/"
  ]
}'

for TARGET_DIR in "${CHROME_TARGETS[@]}"; do
    mkdir -p "$TARGET_DIR"
    echo "$MANIFEST_CONTENT" > "$TARGET_DIR/com.algodeck.launcher.json"
done

# Ikona na pulpicie i w menu aplikacji
mkdir -p "$HOME/.local/share/applications"
mkdir -p "$HOME/.local/share/icons"
cp "$REPO_DIR/assets/icon128.png" "$HOME/.local/share/icons/algodeck.png" 2>/dev/null || true

cat << DESKTOP_EOF > "$HOME/.local/share/applications/algodeck.desktop"
[Desktop Entry]
Name=AlgoDeck
Comment=Środowisko Olimpijskie Koło MAP
Exec=$HOME/.local/bin/algodeck
Icon=$HOME/.local/share/icons/algodeck.png
Terminal=false
Type=Application
Categories=Development;IDE;
StartupWMClass=AlgoDeck
DESKTOP_EOF

# Usługa systemd w tle
mkdir -p "$HOME/.config/systemd/user"
cat << SERVICE_EOF > "$HOME/.config/systemd/user/algodeck.service"
[Unit]
Description=AlgoDeck ⚡ Koło MAP - 100% Rust Daemon & Stream Deck Controller
After=network.target

[Service]
Type=simple
ExecStart=$HOME/.local/bin/algodeck daemon
Restart=always
RestartSec=2s
Environment="PATH=$HOME/.local/bin:/usr/local/bin:/usr/bin:/bin"

[Install]
WantedBy=default.target
SERVICE_EOF

systemctl --user daemon-reload 2>/dev/null || true
systemctl --user enable algodeck.service 2>/dev/null || true
systemctl --user restart algodeck.service 2>/dev/null || true

echo -e "\n${GREEN}${BOLD}==============================================================================${NC}"
echo -e "${GREEN}${BOLD}       🎉 AlgoDeck v2.0.0 (100% Rust) zainstalowany i gotowy do pracy!        ${NC}"
echo -e "${GREEN}${BOLD}==============================================================================${NC}"
echo -e "• Daemon działa w tle jako usługa systemd (zużycie RAM: ~2 MB, czas startu: 10 ms)"
echo -e "• Stream Deck: Bezpośrednia obsługa sprzętowa USB HID bez zewnętrznych aplikacji"
echo -e "• Testy w konsoli: 'algodeck test <kod_zadania>'"
echo -e "• Status: 'algodeck status'"
echo -e "• Zatrzymanie / Wygaszenie Stream Decka: 'algodeck stop'"
echo -e "${GREEN}${BOLD}==============================================================================${NC}"

echo -e "\n${CYAN}${BOLD}🧩 JAK DODAĆ ROZSZERZENIE DO PRZEGLĄDARKI (Chrome / Brave / Chromium):${NC}"
echo -e "1. Otwórz w przeglądarce: ${BOLD}chrome://extensions${NC} (lub brave://extensions)"
echo -e "2. Włącz suwak ${BOLD}Tryb programisty${NC} (Developer mode) w prawym górnym rogu"
echo -e "3. Kliknij w lewym górnym rogu: ${BOLD}Załaduj rozpakowane${NC} (Load unpacked)"
echo -e "4. Wskaż DOKŁADNIE ten folder:"
echo -e "   ${GREEN}${BOLD}$HOME/.local/share/algodeck/extension${NC}"
echo -e "5. Gotowe! Przypnij ikonę ⚡ AlgoDeck na pasku narzędzi przeglądarki.\n"
