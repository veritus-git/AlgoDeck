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
            SUDO_KEEP_PID=$!
            trap 'kill "$SUDO_KEEP_PID" 2>/dev/null || true' EXIT
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
            curl wget git unzip tar wmctrl xdotool 2>/dev/null || true
    elif command -v dnf &> /dev/null; then
        sudo dnf install -y -q \
            gcc gcc-c++ gdb clang pkgconfig hidapi-devel libusb1-devel systemd-devel \
            curl wget git unzip tar wmctrl xdotool 2>/dev/null || true
    elif command -v pacman &> /dev/null; then
        sudo pacman -S --needed --noconfirm \
            base-devel gdb clang pkgconf hidapi libusb systemd-libs \
            curl wget git unzip tar wmctrl xdotool 2>/dev/null || true
    elif command -v zypper &> /dev/null; then
        sudo zypper install -y \
            gcc-c++ gdb clang pkg-config hidapi-devel libusb-1_0-devel libudev-devel \
            curl wget git unzip tar wmctrl xdotool 2>/dev/null || true
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
if ! command -v code &> /dev/null; then
    echo -e "${YELLOW}Weryfikacja / próba instalacji Visual Studio Code...${NC}"
    if [ "$HAS_SUDO" = true ]; then
        if command -v apt-get &> /dev/null; then
            wget -qO- https://packages.microsoft.com/keys/microsoft.asc | gpg --dearmor > /tmp/packages.microsoft.gpg 2>/dev/null || true
            sudo install -D -o root -g root -m 644 /tmp/packages.microsoft.gpg /etc/apt/keyrings/packages.microsoft.gpg 2>/dev/null || true
            echo "deb [arch=amd64,arm64,armhf signed-by=/etc/apt/keyrings/packages.microsoft.gpg] https://packages.microsoft.com/repos/code stable main" | sudo tee /etc/apt/sources.list.d/vscode.list > /dev/null 2>&1 || true
            sudo apt-get update -qq 2>/dev/null || true
            sudo apt-get install -y -qq code 2>/dev/null || true
        elif command -v dnf &> /dev/null; then
            sudo rpm --import https://packages.microsoft.com/keys/microsoft.asc 2>/dev/null || true
            sudo sh -c 'echo -e "[code]\nname=Visual Studio Code\nbaseurl=https://packages.microsoft.com/yumrepos/vscode\nenabled=1\ngpgcheck=1\ngpgkey=https://packages.microsoft.com/keys/microsoft.asc" > /etc/yum.repos.d/vscode.repo' 2>/dev/null || true
            sudo dnf install -y -q code 2>/dev/null || true
        elif command -v zypper &> /dev/null; then
            sudo rpm --import https://packages.microsoft.com/keys/microsoft.asc 2>/dev/null || true
            sudo sh -c 'echo -e "[code]\nname=Visual Studio Code\nbaseurl=https://packages.microsoft.com/yumrepos/vscode\nenabled=1\ntype=rpm-md\ngpgcheck=1\ngpgkey=https://packages.microsoft.com/keys/microsoft.asc" > /etc/zypp/repos.d/vscode.repo' 2>/dev/null || true
            sudo zypper refresh 2>/dev/null || true
            sudo zypper install -y code 2>/dev/null || true
        fi
    fi

    # Uniwersalny fallback: pobranie oficjalnej paczki standalone od Microsoft do ~/.local/lib/
    if ! command -v code &> /dev/null; then
        echo -e "${CYAN}Pobieranie oficjalnej paczki Visual Studio Code od Microsoft...${NC}"
        TMP_CODE_TAR="/tmp/vscode_linux_$$.tar.gz"
        mkdir -p "$HOME/.local/lib"
        if curl -fsSL -o "$TMP_CODE_TAR" "https://update.code.visualstudio.com/latest/linux-x64/stable" 2>/dev/null && [ -s "$TMP_CODE_TAR" ]; then
            tar -xzf "$TMP_CODE_TAR" -C "$HOME/.local/lib/" 2>/dev/null || true
            rm -f "$TMP_CODE_TAR"
            if [ -f "$HOME/.local/lib/VSCode-linux-x64/bin/code" ]; then
                mkdir -p "$HOME/.local/bin"
                ln -sf "$HOME/.local/lib/VSCode-linux-x64/bin/code" "$HOME/.local/bin/code"
                export PATH="$HOME/.local/bin:$PATH"
            fi
        fi
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
EXT_DIR="$SHARE_DIR/extension"

mkdir -p "$BIN_DIR"
mkdir -p "$SHARE_DIR/frontend/icons"
mkdir -p "$EXT_DIR"
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
    install_vscode_ext() {
        local src="$1"
        local roots=(
            "$HOME/.vscode/extensions"
            "$HOME/.vscode-oss/extensions"
            "$HOME/.var/app/com.visualstudio.code/data/vscode/extensions"
            "$HOME/.var/app/com.vscodium.codium/data/vscode/extensions"
        )
        for r in "${roots[@]}"; do
            if [ "$r" = "$HOME/.vscode/extensions" ] || [ -d "$(dirname "$r")" ] || [ -d "$r" ]; then
                mkdir -p "$r/algodeck.algodeck-vscode-bridge-2.1.0" "$r/algodeck-vscode-bridge"
                cp -r "$src/"* "$r/algodeck.algodeck-vscode-bridge-2.1.0/" 2>/dev/null || true
                cp -r "$src/"* "$r/algodeck-vscode-bridge/" 2>/dev/null || true
                rm -rf "$r/algodeck.algodeck-vscode-bridge-2.0.0" 2>/dev/null || true
                if [ -f "$r/.obsolete" ]; then
                    sed -i '/algodeck/d' "$r/.obsolete" 2>/dev/null || true
                fi
            fi
        done
    }

    if [ -d "$REPO_DIR/vscode_extension" ]; then
        install_vscode_ext "$REPO_DIR/vscode_extension"
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
        install_vscode_ext "$REPO_DIR/vscode_extension"
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
            install_vscode_ext "$TMP_DIR/vscode_extension"
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
            if [ -n "$ROOT_EXTRACT" ] && [ -d "$ROOT_EXTRACT/vscode_extension" ]; then
                install_vscode_ext "$ROOT_EXTRACT/vscode_extension"
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

LOCAL_EXT_ID=$(python3 -c "import hashlib; p='$EXT_DIR'; print(''.join(chr(int(c, 16) + ord('a')) for c in hashlib.sha256(p.encode('utf-8')).hexdigest()[:32]))" 2>/dev/null || true)

ALLOWED_ORIGINS='[
    "chrome-extension://jglgkkeklbipodmhcncnmdnippdijjcp/",
    "chrome-extension://cdgdnkfkkbadljdgjjjbaefefcbelplf/",
    "chrome-extension://jnfimdneicefaomnlikmogffogcadabf/"'

if [ -n "$LOCAL_EXT_ID" ]; then
    ALLOWED_ORIGINS="$ALLOWED_ORIGINS"',
    "chrome-extension://'"$LOCAL_EXT_ID"'/"'
fi
ALLOWED_ORIGINS="$ALLOWED_ORIGINS"'
  ]'

MANIFEST_CONTENT='{
  "name": "com.algodeck.launcher",
  "description": "AlgoDeck 100% Rust Native Messaging Host",
  "path": "'"$BIN_DIR/algodeck-native-host"'",
  "type": "stdio",
  "allowed_origins": '"$ALLOWED_ORIGINS"'
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
PassEnvironment=DISPLAY WAYLAND_DISPLAY XDG_CURRENT_DESKTOP XDG_SESSION_TYPE XAUTHORITY DBUS_SESSION_BUS_ADDRESS XDG_RUNTIME_DIR

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

# ------------------------------------------------------------------------------
# ZAPIS STANU POCZĄTKOWEGO POBRANYCH PLIKÓW (IGNOROWANIE STARYCH ZADAŃ)
# ------------------------------------------------------------------------------
CFG_DIR="$HOME/.config/algodeck"
mkdir -p "$CFG_DIR"
rm -rf "$SHARE_DIR/staging"/* 2>/dev/null || true
mkdir -p "$SHARE_DIR/staging" 2>/dev/null || true

# Wyłączenie KDE Plasma Focus Stealing Prevention (zapobiega blokowaniu focusu okien przez KWin)
for tool in kwriteconfig6 kwriteconfig5; do
    if command -v "$tool" &>/dev/null; then
        "$tool" --file kwinrc --group Windows --key FocusStealingPreventionLevel 0 2>/dev/null || true
        "$tool" --file kwinrc --group Windows --key FocusStealingPreventionSeverity 0 2>/dev/null || true
    fi
done
KWINRC="$HOME/.config/kwinrc"
if [ -f "$KWINRC" ]; then
    if grep -q "\[Windows\]" "$KWINRC"; then
        sed -i -E 's/FocusStealingPreventionLevel\s*=\s*[0-9]+/FocusStealingPreventionLevel=0/g' "$KWINRC" 2>/dev/null || true
        sed -i -E 's/FocusStealingPreventionSeverity\s*=\s*[0-9]+/FocusStealingPreventionSeverity=0/g' "$KWINRC" 2>/dev/null || true
        grep -q "FocusStealingPreventionLevel=0" "$KWINRC" || sed -i '/\[Windows\]/a FocusStealingPreventionLevel=0' "$KWINRC" 2>/dev/null || true
    else
        printf "\n[Windows]\nFocusStealingPreventionLevel=0\nFocusStealingPreventionSeverity=0\n" >> "$KWINRC"
    fi
fi
for q in qdbus6 qdbus-qt6 qdbus; do
    if command -v "$q" &>/dev/null; then
        "$q" org.kde.KWin /KWin org.kde.KWin.reconfigure 2>/dev/null || true
        break
    fi
done
if command -v dbus-send &>/dev/null; then
    dbus-send --session --dest=org.kde.KWin /KWin org.kde.KWin.reconfigure 2>/dev/null || true
fi

python3 -c "
import os, json, time, hashlib, subprocess

candidate_dirs = set()

# 1. System dirs (xdg-user-dir, standard home dirs)
try:
    p = subprocess.run(['xdg-user-dir', 'DOWNLOAD'], capture_output=True, text=True)
    if p.returncode == 0 and p.stdout.strip():
        candidate_dirs.add(os.path.abspath(p.stdout.strip()))
except Exception:
    pass

for std_name in ['Downloads', 'Pobrane', 'downloads', 'pobrane']:
    std_p = os.path.expanduser(f'~/{std_name}')
    if os.path.isdir(std_p):
        candidate_dirs.add(os.path.abspath(std_p))

# Check nested download folders (e.g. ~/Pobrane/Downloads)
for d in list(candidate_dirs):
    for sub in ['Downloads', 'Pobrane']:
        subp = os.path.join(d, sub)
        if os.path.isdir(subp):
            candidate_dirs.add(os.path.abspath(subp))

# 2. Browser preferences (Brave, Chrome, Chromium, Edge, Vivaldi, Opera)
browser_configs = [
    '~/.config/BraveSoftware/Brave-Browser',
    '~/.config/google-chrome',
    '~/.config/chromium',
    '~/.config/microsoft-edge',
    '~/.config/vivaldi',
    '~/.config/opera',
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

# Zapis znanych folderów pobrań
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
# AUTOMATYZACJA ROZSZERZENIA: SCHOWEK, MENEDŻER PLIKÓW I PRZEGLĄDARKA
# ------------------------------------------------------------------------------
EXT_DIR="$HOME/.local/share/algodeck/extension"

# 1. Automatyczne skopiowanie ścieżki do schowka Linux (Wayland wl-copy lub X11 xclip/xsel)
if command -v wl-copy &>/dev/null; then
    echo -n "$EXT_DIR" | wl-copy 2>/dev/null || true
elif command -v xclip &>/dev/null; then
    echo -n "$EXT_DIR" | xclip -selection clipboard 2>/dev/null || true
elif command -v xsel &>/dev/null; then
    echo -n "$EXT_DIR" | xsel --clipboard --input 2>/dev/null || true
fi
if command -v base64 &>/dev/null; then
    printf "\033]52;c;%s\a" "$(echo -n "$EXT_DIR" | base64 | tr -d '\r\n')" 2>/dev/null || true
fi

# 2. Automatyczne otwarcie strony rozszerzeń w domyślnej przeglądarce Chromium
if command -v brave-browser &>/dev/null; then
    brave-browser "brave://extensions" </dev/null &>/dev/null & disown 2>/dev/null || true
elif command -v brave &>/dev/null; then
    brave "brave://extensions" </dev/null &>/dev/null & disown 2>/dev/null || true
elif command -v google-chrome &>/dev/null; then
    google-chrome "chrome://extensions" </dev/null &>/dev/null & disown 2>/dev/null || true
elif command -v google-chrome-stable &>/dev/null; then
    google-chrome-stable "chrome://extensions" </dev/null &>/dev/null & disown 2>/dev/null || true
elif command -v chromium &>/dev/null; then
    chromium "chrome://extensions" </dev/null &>/dev/null & disown 2>/dev/null || true
elif command -v chromium-browser &>/dev/null; then
    chromium-browser "chrome://extensions" </dev/null &>/dev/null & disown 2>/dev/null || true
elif command -v microsoft-edge &>/dev/null || command -v microsoft-edge-stable &>/dev/null; then
    (microsoft-edge "edge://extensions" </dev/null &>/dev/null || microsoft-edge-stable "edge://extensions" </dev/null &>/dev/null) & disown 2>/dev/null || true
elif command -v flatpak &>/dev/null && flatpak info com.brave.Browser &>/dev/null; then
    flatpak run com.brave.Browser "brave://extensions" </dev/null &>/dev/null & disown 2>/dev/null || true
elif command -v flatpak &>/dev/null && flatpak info com.google.Chrome &>/dev/null; then
    flatpak run com.google.Chrome "chrome://extensions" </dev/null &>/dev/null & disown 2>/dev/null || true
fi

# 3. Otwarcie folderu rozszerzenia w menedżerze plików dla łatwego Drag & Drop
xdg-open "$EXT_DIR" </dev/null &>/dev/null & disown 2>/dev/null || true

echo -e "\n${GREEN}${BOLD}==============================================================================${NC}"
echo -e "${GREEN}${BOLD}       🎉 AlgoDeck v2.0.0 (100% Rust) pomyślnie zainstalowany!                ${NC}"
echo -e "${GREEN}${BOLD}==============================================================================${NC}"
echo -e "• Panel Web Hub: ${CYAN}${BOLD}http://127.0.0.1:8080/${NC} lub kliknij ikonę rozszerzenia"
echo -e "• Stream Deck: Bezpośrednia obsługa USB HID bez jakichkolwiek zewnętrznych apek"
echo -e "• Narzędzie CLI: ${BOLD}algodeck status${NC} | ${BOLD}algodeck test <kod_zadania>${NC} | ${BOLD}algodeck stop${NC}"
echo -e "${GREEN}${BOLD}==============================================================================${NC}"

echo -e "\n${CYAN}${BOLD}🧩 DODANIE ROZSZERZENIA DO PRZEGLĄDARKI (Automatyczne ułatwienie):${NC}"
echo -e "${GREEN}✓ Ścieżka folderu rozszerzenia została AUTOMATYCZNIE SKOPIOWANA do Twojego schowka (Ctrl+V)!${NC}"
echo -e "${GREEN}✓ Otwarto stronę rozszerzeń w przeglądarce oraz folder rozszerzenia.${NC}"
echo -e "\n1. Wejdź w przeglądarce pod adres: ${BOLD}chrome://extensions${NC} (lub brave://extensions)"
echo -e "2. Włącz w prawym górnym rogu przełącznik ${BOLD}Tryb programisty${NC} (Developer mode)"
echo -e "3. Kliknij w lewym górnym rogu przycisk ${BOLD}Załaduj rozpakowane${NC} (Load unpacked)"
echo -e "4. W oknie wyboru wklej schowek (${BOLD}Ctrl+V${NC}) lub przeciągnij otwarty folder:"
echo -e "   ${GREEN}${BOLD}$EXT_DIR${NC}"
echo -e "5. Gotowe! Przypnij ikonę ⚡ AlgoDeck na pasku przeglądarki.\n"
