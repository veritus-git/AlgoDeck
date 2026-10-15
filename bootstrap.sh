#!/usr/bin/env bash
# ==============================================================================
# AlgoDeck ⚡ Koło MAP - 100% Rust One-Click Installer (Linux)
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
echo -e "${BOLD}AlgoDeck ⚡ Koło MAP - Oficjalny Instalator dla Linuksa (100% Rust)${NC}"
echo -e "Weryfikacja kompilatora C++, Stream Deck USB HID, VS Code, daemona i rozszerzenia...\n"

# 0. Sudo keepalive jeśli dostępne
HAS_SUDO=false
if [ "$EUID" -eq 0 ]; then
    HAS_SUDO=true
elif command -v sudo &> /dev/null; then
    if sudo -n true 2>/dev/null; then
        HAS_SUDO=true
    else
        echo -e "${YELLOW}Weryfikacja uprawnień administratora (sudo) dla pakietów i udev...${NC}"
        if sudo -v </dev/tty 2>/dev/null; then
            HAS_SUDO=true
            while true; do sudo -n true; sleep 40; kill -0 "$$" || exit; done 2>/dev/null &
        else
            echo -e "${YELLOW}Brak uprawnień sudo — instalacja będzie kontynuowana w przestrzeni użytkownika (~/.local).${NC}"
        fi
    fi
fi

# 1. Pakiety systemowe i kompilator C++
echo -e "${CYAN}[1/6] Weryfikacja pakietów systemowych i kompilatora C++...${NC}"
if [ "$HAS_SUDO" = true ]; then
    if command -v apt-get &> /dev/null; then
        sudo apt-get update -qq 2>/dev/null || true
        sudo apt-get install -y -qq \
            build-essential gdb clang pkg-config libhidapi-dev libusb-1.0-0-dev libudev-dev \
            curl wget git unzip tar 2>/dev/null || true
    elif command -v dnf &> /dev/null; then
        sudo dnf install -y -q \
            gcc gcc-c++ gdb clang pkgconfig hidapi-devel libusb1-devel systemd-devel \
            curl wget git unzip tar 2>/dev/null || true
    elif command -v pacman &> /dev/null; then
        sudo pacman -S --needed --noconfirm \
            base-devel gdb clang pkgconf hidapi libusb systemd-libs \
            curl wget git unzip tar 2>/dev/null || true
    elif command -v zypper &> /dev/null; then
        sudo zypper install -y \
            gcc-c++ gdb clang pkg-config hidapi-devel libusb-1_0-devel libudev-devel \
            curl wget git unzip tar 2>/dev/null || true
    fi
fi

if command -v g++ &> /dev/null; then
    echo -e "${GREEN}✓ Wykryto kompilator GNU C++: $(g++ --version | head -n 1)${NC}"
elif command -v clang++ &> /dev/null; then
    echo -e "${GREEN}✓ Wykryto kompilator Clang C++: $(clang++ --version | head -n 1)${NC}"
else
    echo -e "${YELLOW}Uwaga: Nie wykryto g++ ani clang++. Upewnij się, że masz zainstalowany kompilator C++20.${NC}"
fi

# 2. Sprawdzenie środowiska Rust (do budowy lokalnej)
echo -e "\n${CYAN}[2/6] Weryfikacja środowiska Rust...${NC}"
if command -v cargo &> /dev/null; then
    echo -e "${GREEN}✓ Wykryto Rust i Cargo: $(cargo --version)${NC}"
else
    echo -e "${BLUE}Cargo nie jest wymagane przy korzystaniu z gotowych binarek Releases.${NC}"
fi

# 3. Sprawdzenie Visual Studio Code
echo -e "\n${CYAN}[3/6] Weryfikacja edytora Visual Studio Code...${NC}"
if ! command -v code &> /dev/null && [ "$HAS_SUDO" = true ]; then
    echo -e "${YELLOW}Próba automatycznej instalacji Visual Studio Code...${NC}"
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
else
    echo -e "${YELLOW}Uwaga: Visual Studio Code nie został znaleziony w PATH.${NC}"
    echo -e "Zainstaluj VS Code z: https://code.visualstudio.com/"
fi

# 4. Instalacja silnika AlgoDeck i assetów
echo -e "\n${CYAN}[4/6] Przygotowanie binarki AlgoDeck, Web Hubu i rozszerzenia...${NC}"
REPO_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" 2>/dev/null && pwd || echo "$HOME")"
BIN_DIR="$HOME/.local/bin"
SHARE_DIR="$HOME/.local/share/algodeck"

mkdir -p "$BIN_DIR"
mkdir -p "$SHARE_DIR/frontend/icons"
mkdir -p "$SHARE_DIR/extension"
mkdir -p "$SHARE_DIR/icons"

INSTALLED_BIN=false

# 4a. Jeśli uruchomiono w sklonowanym repozytorium z Cargo.toml
if [ -f "$REPO_DIR/Cargo.toml" ] && command -v cargo &> /dev/null; then
    echo -e "${CYAN}Kompilacja lokalna ze źródeł (Rust release)...${NC}"
    (cd "$REPO_DIR" && cargo build --release)
    install -m 755 "$REPO_DIR/target/release/algodeck" "$BIN_DIR/algodeck"
    INSTALLED_BIN=true

    # Kopiuj kompletny frontend i rozszerzenie
    if [ -d "$REPO_DIR/frontend" ]; then
        cp -r "$REPO_DIR/frontend/"* "$SHARE_DIR/frontend/"
        cp -r "$REPO_DIR/frontend/icons/"* "$SHARE_DIR/icons/" 2>/dev/null || true
    fi
    if [ -d "$REPO_DIR/extension" ]; then
        cp -r "$REPO_DIR/extension/"* "$SHARE_DIR/extension/"
    fi
elif [ -f "$REPO_DIR/algodeck" ]; then
    # Jeśli uruchomiono w wypakowanej paczce release tar.gz
    echo -e "${CYAN}Kopiowanie prekompilowanej binarki z bieżącego katalogu...${NC}"
    install -m 755 "$REPO_DIR/algodeck" "$BIN_DIR/algodeck"
    INSTALLED_BIN=true
    if [ -d "$REPO_DIR/frontend" ]; then
        cp -r "$REPO_DIR/frontend/"* "$SHARE_DIR/frontend/"
        cp -r "$REPO_DIR/frontend/icons/"* "$SHARE_DIR/icons/" 2>/dev/null || true
    fi
    if [ -d "$REPO_DIR/extension" ]; then
        cp -r "$REPO_DIR/extension/"* "$SHARE_DIR/extension/"
    fi
    if [ -d "$REPO_DIR/vscode_extension" ]; then
        mkdir -p "$HOME/.vscode/extensions/algodeck-vscode-bridge"
        cp -r "$REPO_DIR/vscode_extension/"* "$HOME/.vscode/extensions/algodeck-vscode-bridge/" 2>/dev/null || true
    fi
fi

# 4b. Jeśli nie zainstalowano z lokalnych plików — pobierz z GitHub Releases
if [ "$INSTALLED_BIN" = false ]; then
    echo -e "${CYAN}Pobieranie paczki instalacyjnej z GitHub Releases...${NC}"
    TAR_URL="https://github.com/veritus-git/AlgoDeck/releases/latest/download/algodeck-linux-x86_64.tar.gz"
    RAW_BIN_URL="https://github.com/veritus-git/AlgoDeck/releases/latest/download/algodeck-linux-x86_64"
    TMP_TAR="/tmp/algodeck_linux_$$.tar.gz"
    TMP_DIR="/tmp/algodeck_unpack_$$"

    if curl -fsSL -o "$TMP_TAR" "$TAR_URL" 2>/dev/null && [ -s "$TMP_TAR" ]; then
        mkdir -p "$TMP_DIR"
        tar -xzf "$TMP_TAR" -C "$TMP_DIR"
        if [ -f "$TMP_DIR/algodeck" ]; then
            install -m 755 "$TMP_DIR/algodeck" "$BIN_DIR/algodeck"
        fi
        if [ -d "$TMP_DIR/frontend" ]; then
            cp -r "$TMP_DIR/frontend/"* "$SHARE_DIR/frontend/"
            cp -r "$TMP_DIR/frontend/icons/"* "$SHARE_DIR/icons/" 2>/dev/null || true
        fi
        if [ -d "$TMP_DIR/extension" ]; then
            cp -r "$TMP_DIR/extension/"* "$SHARE_DIR/extension/"
        fi
        if [ -d "$TMP_DIR/vscode_extension" ]; then
            mkdir -p "$HOME/.vscode/extensions/algodeck-vscode-bridge"
            cp -r "$TMP_DIR/vscode_extension/"* "$HOME/.vscode/extensions/algodeck-vscode-bridge/" 2>/dev/null || true
        fi
        rm -rf "$TMP_TAR" "$TMP_DIR"
        INSTALLED_BIN=true
    else
        # Fallback: pobierz surową binarkę i zaciągnij assety
        echo -e "${CYAN}Pobieranie pojedynczej binarki algodeck-linux-x86_64...${NC}"
        curl -fsSL -o "$BIN_DIR/algodeck" "$RAW_BIN_URL" || true
        chmod +x "$BIN_DIR/algodeck"

        echo -e "${CYAN}Pobieranie zasobów webowych i rozszerzenia...${NC}"
        TMP_ZIP="/tmp/algodeck_assets_$$.zip"
        TMP_DIR="/tmp/algodeck_extract_$$"
        curl -fsSL -o "$TMP_ZIP" "https://github.com/veritus-git/AlgoDeck/archive/refs/heads/main.zip" || true
        if [ -f "$TMP_ZIP" ]; then
            unzip -q -o "$TMP_ZIP" -d "$TMP_DIR" 2>/dev/null || true
            ROOT_EXTRACT=$(find "$TMP_DIR" -maxdepth 1 -name "AlgoDeck*" | head -n 1)
            if [ -n "$ROOT_EXTRACT" ] && [ -d "$ROOT_EXTRACT/frontend" ]; then
                cp -r "$ROOT_EXTRACT/frontend/"* "$SHARE_DIR/frontend/"
                cp -r "$ROOT_EXTRACT/frontend/icons/"* "$SHARE_DIR/icons/" 2>/dev/null || true
            fi
            if [ -n "$ROOT_EXTRACT" ] && [ -d "$ROOT_EXTRACT/extension" ]; then
                cp -r "$ROOT_EXTRACT/extension/"* "$SHARE_DIR/extension/"
            fi
            rm -rf "$TMP_ZIP" "$TMP_DIR"
        fi
    fi
fi
echo -e "${GREEN}✓ Silnik, Web Hub i rozszerzenie zainstalowane w ~/.local/share/algodeck/.${NC}"

# 5. Uprawnienia USB HID dla Stream Decka (udev)
echo -e "\n${CYAN}[5/6] Konfiguracja uprawnień USB HID dla kontrolerów Elgato Stream Deck...${NC}"
if [ "$HAS_SUDO" = true ] && [ -d /etc/udev/rules.d ]; then
    sudo tee /etc/udev/rules.d/99-streamdeck.rules > /dev/null << 'UDEV_EOF'
# Elgato Stream Deck USB HID rules for non-root users (AlgoDeck)
SUBSYSTEM=="usb", ATTRS{idVendor}=="0fd9", MODE="0666", TAG+="uaccess"
KERNEL=="hidraw*", ATTRS{idVendor}=="0fd9", MODE="0666", TAG+="uaccess"
UDEV_EOF
    sudo udevadm control --reload-rules 2>/dev/null || true
    sudo udevadm trigger 2>/dev/null || true
    echo -e "${GREEN}✓ Reguły udev zainstalowane (bezpośredni dostęp USB HID bez roota).${NC}"
else
    echo -e "${YELLOW}Pominięto instalację udev (wymaga sudo). Jeśli używasz fizycznego Stream Decka, dodaj ręcznie regułę do /etc/udev/rules.d/99-streamdeck.rules:${NC}"
    echo 'SUBSYSTEM=="usb", ATTRS{idVendor}=="0fd9", MODE="0666", TAG+="uaccess"'
fi

# 6. Rejestracja Native Messaging Host oraz usługa tła systemd
echo -e "\n${CYAN}[6/6] Konfiguracja integracji przeglądarek i usługi tła...${NC}"

# Wrapper natywny dla Native Messaging
cat << 'WRAPPER_EOF' > "$BIN_DIR/algodeck-native-host"
#!/bin/sh
exec "$HOME/.local/bin/algodeck" native-host
WRAPPER_EOF
chmod +x "$BIN_DIR/algodeck-native-host"

# Manifesty Chrome Native Messaging dla wszystkich przeglądarek Chromium
CHROME_TARGETS=(
    "$HOME/.config/google-chrome/NativeMessagingHosts"
    "$HOME/.config/chromium/NativeMessagingHosts"
    "$HOME/.config/BraveSoftware/Brave-Browser/NativeMessagingHosts"
    "$HOME/.config/microsoft-edge/NativeMessagingHosts"
    "$HOME/.config/vivaldi/NativeMessagingHosts"
    "$HOME/.config/opera/NativeMessagingHosts"
)

MANIFEST_CONTENT='{
  "name": "com.algodeck.launcher",
  "description": "AlgoDeck 100% Rust Native Messaging Host",
  "path": "'"$BIN_DIR/algodeck-native-host"'",
  "type": "stdio",
  "allowed_origins": [
    "chrome-extension://cdgdnkfkkbadljdgjjjbaefefcbelplf/",
    "chrome-extension://jnfimdneicefaomnlikmogffogcadabf/"
  ]
}'

for TARGET_DIR in "${CHROME_TARGETS[@]}"; do
    mkdir -p "$TARGET_DIR" 2>/dev/null || true
    echo "$MANIFEST_CONTENT" > "$TARGET_DIR/com.algodeck.launcher.json" 2>/dev/null || true
done
echo -e "${GREEN}✓ Mostek Native Messaging zarejestrowany dla Chrome, Brave, Chromium i Edge.${NC}"

# PATH i PKG_CONFIG_PATH w powłokach użytkownika
for RC in "$HOME/.bashrc" "$HOME/.zshrc" "$HOME/.profile"; do
    if [ -f "$RC" ]; then
        if ! grep -q '\.local/bin' "$RC" 2>/dev/null; then
            echo 'export PATH="$HOME/.local/bin:$PATH"' >> "$RC"
        fi
        if [ -d "$HOME/.local/lib/pkgconfig" ] && ! grep -q '\.local/lib/pkgconfig' "$RC" 2>/dev/null; then
            echo 'export PKG_CONFIG_PATH="$HOME/.local/lib/pkgconfig:$PKG_CONFIG_PATH"' >> "$RC"
        fi
    fi
done

# Ikona i plik .desktop
mkdir -p "$HOME/.local/share/applications"
mkdir -p "$HOME/.local/share/icons"
if [ -f "$SHARE_DIR/frontend/icons/icon128.png" ]; then
    cp "$SHARE_DIR/frontend/icons/icon128.png" "$HOME/.local/share/icons/algodeck.png"
fi

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
if command -v systemctl &> /dev/null && systemctl --user list-units &> /dev/null; then
    mkdir -p "$HOME/.config/systemd/user"
    cat << SERVICE_EOF > "$HOME/.config/systemd/user/algodeck.service"
[Unit]
Description=AlgoDeck ⚡ Koło MAP - 100% Rust Daemon & Stream Deck Controller
After=network.target

[Service]
Type=simple
ExecStart=$HOME/.local/bin/algodeck daemon
Restart=on-failure
RestartSec=2s
Environment="PATH=$HOME/.local/bin:/usr/local/bin:/usr/bin:/bin"

[Install]
WantedBy=default.target
SERVICE_EOF

    systemctl --user daemon-reload 2>/dev/null || true
    systemctl --user enable algodeck.service 2>/dev/null || true
    systemctl --user restart algodeck.service 2>/dev/null || true
    echo -e "${GREEN}✓ Usługa systemd (algodeck.service) uruchomiona w tle.${NC}"
else
    # Fallback dla środowisk bez aktywnego systemd --user (np. WSL1, kontenery)
    mkdir -p "$HOME/.config/autostart"
    cp "$HOME/.local/share/applications/algodeck.desktop" "$HOME/.config/autostart/" 2>/dev/null || true
    echo -e "${BLUE}ℹ Skrót autostartu dodany do ~/.config/autostart/algodeck.desktop.${NC}"
fi

echo -e "\n${GREEN}${BOLD}==============================================================================${NC}"
echo -e "${GREEN}${BOLD}       🎉 AlgoDeck v2.0.0 (100% Rust) pomyślnie zainstalowany!                ${NC}"
echo -e "${GREEN}${BOLD}==============================================================================${NC}"
echo -e "• Panel Web Hub: ${CYAN}${BOLD}http://127.0.0.1:8080/${NC} lub kliknij ikonę rozszerzenia"
echo -e "• Stream Deck: Bezpośrednia obsługa USB HID bez jakichkolwiek zewnętrznych apek"
echo -e "• Narzędzie CLI: ${BOLD}algodeck status${NC} | ${BOLD}algodeck test <kod_zadania>${NC} | ${BOLD}algodeck stop${NC}"
echo -e "${GREEN}${BOLD}==============================================================================${NC}"

echo -e "\n${CYAN}${BOLD}🧩 DODANIE ROZSZERZENIA DO PRZEGLĄDARKI (Chrome / Brave / Edge):${NC}"
echo -e "1. Wejdź w przeglądarce pod adres: ${BOLD}chrome://extensions${NC} (lub brave://extensions)"
echo -e "2. Włącz w prawym górnym rogu przełącznik ${BOLD}Tryb programisty${NC} (Developer mode)"
echo -e "3. Kliknij w lewym górnym rogu przycisk ${BOLD}Załaduj rozpakowane${NC} (Load unpacked)"
echo -e "4. Wybierz ten folder:"
echo -e "   ${GREEN}${BOLD}$HOME/.local/share/algodeck/extension${NC}"
echo -e "5. Gotowe! Przypnij ikonę ⚡ AlgoDeck na pasku przeglądarki.\n"
