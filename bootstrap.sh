#!/usr/bin/env bash
# ==============================================================================
# AlgoDeck - One-Click Installer (Bootstrap) dla Fedora KDE Plasma
# Automatyczna konfiguracja środowiska olimpijskiego C++ i integracji Stream Deck
# ==============================================================================

set -e

# Kolory do logowania w terminalu
RED='\033[0;31m'
GREEN='\033[0;32m'
CYAN='\033[0;36m'
YELLOW='\033[1;33m'
BOLD='\033[1m'
NC='\033[0m' # No Color

echo -e "${CYAN}${BOLD}"
echo "  █████╗ ██╗      ██████╗  ██████╗ ██████╗ ███████╗ ██████╗██╗  ██╗"
echo " ██╔══██╗██║     ██╔════╝ ██╔═══██╗██╔══██╗██╔════╝██╔════╝██║ ██╔╝"
echo " ███████║██║     ██║  ███╗██║   ██║██║  ██║█████╗  ██║     █████═╝ "
echo " ██╔══██║██║     ██║   ██║██║   ██║██║  ██║██╔══╝  ██║     ██╔═██╗ "
echo " ██║  ██║███████╗╚██████╔╝╚██████╔╝██████╔╝███████╗╚██████╗██║  ██╗"
echo " ╚═╝  ╚═╝╚══════╝ ╚═════╝  ╚═════╝ ╚═════╝ ╚══════╝ ╚═════╝╚═╝  ╚═╝"
echo -e "${NC}"
echo -e "${BOLD}One-Click Bootstrap dla Fedora KDE Plasma${NC}"
echo -e "Instalacja narzędzi C++, Stream Decka, VS Code oraz środowiska AlgoDeck...\n"

# 1. Weryfikacja systemu operacyjnego
if [ -f /etc/os-release ]; then
    . /etc/os-release
    echo -e "${CYAN}[1/8] Wykryto system:${NC} $PRETTY_NAME"
    if [[ "$ID" != "fedora" && "$ID_LIKE" != *"fedora"* ]]; then
        echo -e "${YELLOW}Uwaga: Skrypt zoptymalizowano pod dystrybucję Fedora. Wykryto $ID. Kontynuowanie z dnf/apt...${NC}"
    fi
else
    echo -e "${RED}Nie można określić dystrybucji systemu operacyjnego.${NC}"
    exit 1
fi

# 2. Instalacja narzędzi C++ i bibliotek systemowych przez DNF
echo -e "\n${CYAN}[2/8] Instalacja kompilatorów C++, bibliotek i narzędzi deweloperskich...${NC}"
if command -v dnf &> /dev/null; then
    sudo dnf check-update || true
    sudo dnf install -y \
        gcc \
        gcc-c++ \
        gdb \
        clang \
        make \
        cmake \
        ninja-build \
        valgrind \
        glibc-devel \
        libstdc++-devel \
        python3 \
        python3-devel \
        python3-pip \
        libusb1-devel \
        hidapi-devel \
        systemd-devel \
        wl-clipboard \
        xclip \
        flatpak \
        curl \
        wget \
        git
elif command -v apt-get &> /dev/null; then
    sudo apt-get update
    sudo apt-get install -y \
        build-essential \
        gdb \
        clang \
        cmake \
        ninja-build \
        valgrind \
        python3 \
        python3-dev \
        python3-pip \
        python3-venv \
        libusb-1.0-0-dev \
        libhidapi-dev \
        wl-clipboard \
        xclip \
        curl \
        git
fi

# 3. Konfiguracja reguł udev dla Elgato Stream Deck
echo -e "\n${CYAN}[3/8] Konfiguracja reguł udev dla kontrolera Elgato Stream Deck...${NC}"
sudo tee /etc/udev/rules.d/60-streamdeck.rules > /dev/null << 'EOF'
# Elgato Stream Deck USB permissions
SUBSYSTEM=="usb", ATTRS{idVendor}=="0fd9", TAG+="uaccess", MODE="0666"
KERNEL=="hidraw*", ATTRS{idVendor}=="0fd9", TAG+="uaccess", MODE="0666"
EOF

sudo udevadm control --reload-rules
sudo udevadm trigger
echo -e "${GREEN}✓ Reguły udev załadowane pomyślnie.${NC}"

# 4. Instalacja Visual Studio Code i rozszerzeń C++
echo -e "\n${CYAN}[4/8] Sprawdzanie i instalacja Visual Studio Code...${NC}"
if ! command -v code &> /dev/null; then
    if command -v dnf &> /dev/null; then
        echo "Dodawanie oficjalnego repozytorium Microsoft RPM dla VS Code..."
        sudo rpm --import https://packages.microsoft.com/keys/microsoft.asc
        sudo sh -c 'echo -e "[code]\nname=Visual Studio Code\ntype=rpm-md\nbaseurl=https://packages.microsoft.com/yumrepos/vscode\nenabled=1\ngpgcheck=1\ngpgkey=https://packages.microsoft.com/keys/microsoft.asc" > /etc/yum.repos.d/vscode.repo'
        sudo dnf check-update || true
        sudo dnf install -y code
    fi
fi

if command -v code &> /dev/null; then
    echo "Instalacja rozszerzeń C++ w VS Code..."
    code --install-extension ms-vscode.cpptools --force || true
    code --install-extension ms-vscode.cpptools-extension-pack --force || true
    echo -e "${GREEN}✓ VS Code i rozszerzenia C++ zainstalowane.${NC}"
else
    echo -e "${YELLOW}Uwaga: VS Code nie został zainstalowany automatycznie.${NC}"
fi

# 5. Instalacja StreamController (Flatpak)
echo -e "\n${CYAN}[5/8] Konfiguracja StreamControllera (oprogramowanie Stream Deck pod Linuksem)...${NC}"
if command -v flatpak &> /dev/null; then
    flatpak remote-add --if-not-exists flathub https://dl.flathub.org/repo/flathub.flatpakrepo || true
    flatpak install -y flathub com.core447.StreamController || true
    echo -e "${GREEN}✓ StreamController zainstalowany z Flathuba.${NC}"
fi

# 6. Konfiguracja środowiska Python dla AlgoDeck
echo -e "\n${CYAN}[6/8] Tworzenie środowiska Python i instalacja zależności AlgoDeck...${NC}"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
INSTALL_DIR="$HOME/.local/share/algodeck"
VENV_DIR="$INSTALL_DIR/venv"

mkdir -p "$INSTALL_DIR"
mkdir -p "$HOME/algodeck-workspace"
mkdir -p "$HOME/.local/bin"

# Kopiowanie plików aplikacji do ~/.local/share/algodeck jeśli instalujemy globalnie
if [ "$PROJECT_ROOT" != "$INSTALL_DIR" ]; then
    rsync -av --exclude='.venv' --exclude='__pycache__' "$PROJECT_ROOT/" "$INSTALL_DIR/"
fi

python3 -m venv "$VENV_DIR"
"$VENV_DIR/bin/pip" install --upgrade pip
"$VENV_DIR/bin/pip" install -r "$INSTALL_DIR/requirements.txt"
echo -e "${GREEN}✓ Środowisko Python i pakiety zainstalowane.${NC}"

# 7. Utworzenie skrótu CLI i wpisu w menu KDE Plasma (.desktop)
echo -e "\n${CYAN}[7/8] Konfiguracja polecenia 'algodeck' oraz ikony w menu KDE Plasma...${NC}"

cat > "$HOME/.local/bin/algodeck" << EOF
#!/usr/bin/env bash
source "$VENV_DIR/bin/activate"
export PYTHONPATH="$INSTALL_DIR"
cd "$INSTALL_DIR"
exec python3 -m uvicorn backend.server:app --host 127.0.0.1 --port 8080 "\$@"
EOF
chmod +x "$HOME/.local/bin/algodeck"

# Upewnij się, że ~/.local/bin jest w PATH użytkownika
if [[ ":$PATH:" != *":$HOME/.local/bin:"* ]]; then
    echo 'export PATH="$HOME/.local/bin:$PATH"' >> "$HOME/.bashrc"
fi

# Wpis Desktop dla KDE Plasma Kickoff
mkdir -p "$HOME/.local/share/applications"
cat > "$HOME/.local/share/applications/algodeck.desktop" << EOF
[Desktop Entry]
Name=AlgoDeck
Comment=Zautomatyzowane środowisko do algorytmiki (Szkopuł / OI) ze Stream Deckiem
Exec=$HOME/.local/bin/algodeck
Icon=applications-development
Terminal=true
Type=Application
Categories=Development;IDE;Education;
StartupNotify=true
EOF

# Systemd User Service (opcjonalny autostart)
mkdir -p "$HOME/.config/systemd/user"
cat > "$HOME/.config/systemd/user/algodeck.service" << EOF
[Unit]
Description=AlgoDeck - Olimpijskie środowisko programistyczne
After=network.target

[Service]
Type=simple
WorkingDirectory=$INSTALL_DIR
ExecStart=$HOME/.local/bin/algodeck
Restart=on-failure
RestartSec=5s

[Install]
WantedBy=default.target
EOF

systemctl --user daemon-reload
echo -e "${GREEN}✓ Skróty i konfiguracja systemowa zakończona.${NC}"

# 8. Podsumowanie i instrukcje
echo -e "\n${GREEN}${BOLD}==============================================================================${NC}"
echo -e "${GREEN}${BOLD}           INSTALACJA ALGODECK ZAKOŃCZONA SUKCESEM! 🎉                      ${NC}"
echo -e "${GREEN}${BOLD}==============================================================================${NC}\n"
echo -e "Jak zacząć:"
echo -e "  1. Uruchom serwer wpisując w terminalu:  ${CYAN}algodeck${NC}"
echo -e "     Lub znajdź '${CYAN}AlgoDeck${NC}' w menu programów KDE Plasma."
echo -e "  2. Otwórz w przeglądarce:               ${CYAN}http://localhost:8080${NC}"
echo -e "  3. Przeciągnij plik PDF z zadaniem ze Szkopuła lub kliknij 'Zadanie demo'."
echo -e "  4. Podłącz Stream Decka lub korzystaj z wirtualnego panelu w przeglądarce!\n"

echo -e "${YELLOW}Konfiguracja konta Gemini (opcjonalna):${NC}"
echo -e "  Aby korzystać z pełnej analizy AI, ustaw swój klucz lub zaloguj się:"
echo -e "  export GEMINI_API_KEY=\"twój_klucz_api\""
echo -e "  (Bez klucza program automatycznie korzysta z wbudowanego parsera heurystycznego).\n"
