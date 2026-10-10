#!/usr/bin/env bash
# ==============================================================================
# AlgoDeck ⚡ Koło MAP - One-Click Full Installer (Bootstrap)
# W 100% zautomatyzowana instalacja: kompilatory C++, Stream Deck, VS Code,
# rozszerzenie do przeglądarki, systemd daemon oraz minimalistyczne profile.
# ==============================================================================

set -e

# Wyłączenie interaktywnych monitów (git, debconf, pip)
export GIT_TERMINAL_PROMPT=0
export GIT_ASKPASS=/bin/true
export SSH_ASKPASS=/bin/true
export PIP_NO_INPUT=1
export DEBIAN_FRONTEND=noninteractive
export ELECTRON_DISABLE_SECURITY_WARNINGS=1

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
echo -e "${NC}"
echo -e "${BOLD}AlgoDeck ⚡ Koło MAP - Kompletny Instalator One-Click${NC}"
echo -e "Instalacja narzędzi C++, Stream Decka, VS Code, daemona i rozszerzenia...\n"

# 0. Jednorazowa autoryzacja sudo z podtrzymywaniem w tle
if [ "$EUID" -ne 0 ] && command -v sudo &> /dev/null; then
    if ! sudo -n true 2>/dev/null; then
        echo -e "${YELLOW}Weryfikacja uprawnień administratora (sudo)...${NC}"
        sudo -v
    fi
    while true; do sudo -n true; sleep 40; kill -0 "$$" || exit; done 2>/dev/null &
fi

# 1. Wykrycie systemu
if [ -f /etc/os-release ]; then
    . /etc/os-release
    echo -e "${CYAN}[1/8] Wykryto system:${NC} $PRETTY_NAME"
fi

# 2. Pakiety systemowe (Kompilatory C++, narzędzia okienkowe, schowki)
echo -e "\n${CYAN}[2/8] Instalacja pakietów systemowych i narzędzi kompilacji C++...${NC}"
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
        wmctrl \
        xdotool \
        gnome-terminal \
        flatpak \
        curl \
        wget \
        git \
        unzip \
        tar
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
        wmctrl \
        xdotool \
        gnome-terminal \
        curl \
        wget \
        git \
        unzip \
        tar
fi
echo -e "${GREEN}✓ Pakiety systemowe zainstalowane.${NC}"

# 3. Reguły udev dla Elgato Stream Deck
echo -e "\n${CYAN}[3/8] Konfiguracja reguł udev dla kontrolera Elgato Stream Deck...${NC}"
sudo tee /etc/udev/rules.d/60-streamdeck.rules > /dev/null << 'EOF'
SUBSYSTEM=="usb", ATTRS{idVendor}=="0fd9", TAG+="uaccess", MODE="0666"
KERNEL=="hidraw*", ATTRS{idVendor}=="0fd9", TAG+="uaccess", MODE="0666"
EOF

sudo udevadm control --reload-rules
sudo udevadm trigger
echo -e "${GREEN}✓ Reguły udev aktywne.${NC}"

# 4. Visual Studio Code i rozszerzenia C++
echo -e "\n${CYAN}[4/8] Sprawdzanie i konfiguracja Visual Studio Code...${NC}"
if ! command -v code &> /dev/null; then
    if command -v dnf &> /dev/null; then
        echo "Dodawanie repozytorium Microsoft RPM dla VS Code..."
        sudo rpm --import https://packages.microsoft.com/keys/microsoft.asc
        sudo sh -c 'echo -e "[code]\nname=Visual Studio Code\ntype=rpm-md\nbaseurl=https://packages.microsoft.com/yumrepos/vscode\nenabled=1\ngpgcheck=1\ngpgkey=https://packages.microsoft.com/keys/microsoft.asc" > /etc/yum.repos.d/vscode.repo'
        sudo dnf check-update || true
        sudo dnf install -y code
    elif command -v apt-get &> /dev/null; then
        wget -qO- https://packages.microsoft.com/keys/microsoft.asc | gpg --dearmor > /tmp/packages.microsoft.gpg
        sudo install -D -o root -g root -m 644 /tmp/packages.microsoft.gpg /etc/apt/keyrings/packages.microsoft.gpg
        sudo sh -c 'echo "deb [arch=amd64,arm64,armhf signed-by=/etc/apt/keyrings/packages.microsoft.gpg] https://packages.microsoft.com/repos/code stable main" > /etc/apt/sources.list.d/vscode.list'
        rm -f /tmp/packages.microsoft.gpg
        sudo apt-get update
        sudo apt-get install -y code
    fi
fi

if command -v code &> /dev/null; then
    echo "Instalacja rozszerzenia C++ w VS Code..."
    code --password-store="basic" --no-sandbox --install-extension ms-vscode.cpptools --force || true
    echo -e "${GREEN}✓ VS Code i rozszerzenia C++ gotowe.${NC}"
fi

# 5. StreamController (Flatpak)
echo -e "\n${CYAN}[5/8] Konfiguracja StreamControllera (zarządzanie Stream Deck pod Linuksem)...${NC}"
if command -v flatpak &> /dev/null; then
    flatpak remote-add --user --if-not-exists flathub https://dl.flathub.org/repo/flathub.flatpakrepo || true
    flatpak install --user -y flathub com.core447.StreamController || true
    echo -e "${GREEN}✓ StreamController zainstalowany.${NC}"
fi

# 6. Przygotowanie katalogów i środowiska Python dla AlgoDeck
echo -e "\n${CYAN}[6/8] Przygotowanie środowiska i zależności AlgoDeck...${NC}"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
INSTALL_DIR="$HOME/.local/share/algodeck"
VENV_DIR="$INSTALL_DIR/venv"
WORKSPACE_DIR="$HOME/algodeck-workspace"

mkdir -p "$INSTALL_DIR"
mkdir -p "$WORKSPACE_DIR"
mkdir -p "$HOME/.local/bin"

if [ -d "$SCRIPT_DIR/backend" ]; then
    PROJECT_ROOT="$SCRIPT_DIR"
    if [ "$PROJECT_ROOT" != "$INSTALL_DIR" ]; then
        echo "Kopiowanie plików AlgoDeck do $INSTALL_DIR..."
        rsync -av --exclude='.venv' --exclude='__pycache__' "$PROJECT_ROOT/" "$INSTALL_DIR/"
    fi
else
    echo "Pobieranie najnowszej wersji AlgoDeck z GitHub..."
    if [ ! -d "$INSTALL_DIR/backend" ]; then
        curl -fsSL https://github.com/veritus-git/AlgoDeck/archive/refs/heads/main.tar.gz | tar -xz -C "$INSTALL_DIR" --strip-components=1 2>/dev/null || true
    fi
fi

# Środowisko Python bez zbędnego balastu AI
python3 -m venv "$VENV_DIR"
"$VENV_DIR/bin/pip" install --upgrade pip

if [ -f "$INSTALL_DIR/requirements.txt" ]; then
    "$VENV_DIR/bin/pip" install --no-input -r "$INSTALL_DIR/requirements.txt"
else
    "$VENV_DIR/bin/pip" install --no-input fastapi uvicorn pydantic python-multipart pypdf pillow websockets psutil streamdeck
fi

# Generowanie minimalistycznych ikon symbolicznych
echo "Generowanie minimalistycznych ikon dla Stream Decka..."
PYTHONPATH="$INSTALL_DIR" "$VENV_DIR/bin/python3" "$INSTALL_DIR/backend/streamdeck/icons_generator.py" || true

echo -e "${GREEN}✓ Środowisko AlgoDeck i ikony zainstalowane.${NC}"

# 7. Skrypty wykonawcze w ~/.local/bin/
echo -e "\n${CYAN}[7/8] Instalacja skrótów systemowych w ~/.local/bin...${NC}"

# 7a. CLI 'algodeck'
cat > "$HOME/.local/bin/algodeck" << EOF
#!/usr/bin/env bash
source "$VENV_DIR/bin/activate"
export PYTHONPATH="$INSTALL_DIR"
cd "$INSTALL_DIR"
exec python3 -m uvicorn backend.server:app --host 127.0.0.1 --port 8080 "\$@"
EOF
chmod +x "$HOME/.local/bin/algodeck"

# 7b. Skrypt sd_algo_code.sh (Otwieranie / maksymalizacja okna VS Code)
cat > "$HOME/.local/bin/sd_algo_code.sh" << 'EOF'
#!/usr/bin/env bash
TARGET_DIR="${1:-$HOME/algodeck-workspace}"
TARGET_FILE="${2:-}"
PROB_NAME=$(basename "$TARGET_DIR")

WID=$(DISPLAY=:0 wmctrl -lx 2>/dev/null | grep -i "code\.code" | grep -i "$PROB_NAME" | awk '{print $1}' | head -n 1)

if [ -n "$WID" ]; then
    DISPLAY=:0 wmctrl -i -r "$WID" -b add,maximized_vert,maximized_horz 2>/dev/null || true
    DISPLAY=:0 wmctrl -i -a "$WID" 2>/dev/null || true
    exit 0
fi

if [ -n "$TARGET_FILE" ] && [ -f "$TARGET_FILE" ]; then
    nohup /usr/bin/code "$TARGET_DIR" "$TARGET_FILE" >/dev/null 2>&1 &
else
    nohup /usr/bin/code "$TARGET_DIR" >/dev/null 2>&1 &
fi

for i in {1..20}; do
    WID=$(DISPLAY=:0 wmctrl -lx 2>/dev/null | grep -i "code\.code" | grep -i "$PROB_NAME" | awk '{print $1}' | head -n 1)
    if [ -n "$WID" ]; then
        DISPLAY=:0 wmctrl -i -r "$WID" -b add,maximized_vert,maximized_horz 2>/dev/null || true
        DISPLAY=:0 wmctrl -i -a "$WID" 2>/dev/null || true
        exit 0
    fi
    sleep 0.1
done
EOF
chmod +x "$HOME/.local/bin/sd_algo_code.sh"

# 7c. Skrypt sd_algo_run_vscode.sh (Wbudowany terminal VS Code: Ctrl+Shift+B)
cat > "$HOME/.local/bin/sd_algo_run_vscode.sh" << 'EOF'
#!/usr/bin/env bash
PDIR="$1"
PROB="$2"

WID=$(DISPLAY=:0 wmctrl -lx 2>/dev/null | grep -i "code\.code" | grep -i "$PROB" | awk '{print $1}' | head -n 1)
if [ -z "$WID" ]; then
    WID=$(DISPLAY=:0 wmctrl -lx 2>/dev/null | grep -i "code\.code" | awk '{print $1}' | head -n 1)
fi

if [ -z "$WID" ]; then
    code "$PDIR" "$PDIR/$PROB.cpp" &
    sleep 0.8
    WID=$(DISPLAY=:0 wmctrl -lx 2>/dev/null | grep -i "code\.code" | awk '{print $1}' | head -n 1)
fi

if [ -n "$WID" ]; then
    DISPLAY=:0 wmctrl -i -a "$WID" 2>/dev/null || true
    sleep 0.1
    DISPLAY=:0 xdotool key --window "$WID" --clearmodifiers ctrl+shift+b 2>/dev/null || true
fi
EOF
chmod +x "$HOME/.local/bin/sd_algo_run_vscode.sh"

# 7d. Skrypt sd_algo_switch.sh (Przełączanie zadań i galerii)
cat > "$HOME/.local/bin/sd_algo_switch.sh" << 'EOF'
#!/usr/bin/env bash
set -e

ACTION="${1:-next}"
CURRENT="${2:-}"
WORKSPACE="$HOME/algodeck-workspace"
SERIAL="A00SA6042JGA63"

PROJECTS=()
for dir in "$WORKSPACE"/*; do
    [ -d "$dir" ] || continue
    bname=$(basename "$dir")
    [ "$bname" = "tests" ] && continue
    if [ -f "$dir/$bname.cpp" ] || [ -d "$dir/.algo" ]; then
        PROJECTS+=("$bname")
    fi
done

if [ ${#PROJECTS[@]} -eq 0 ]; then
    notify-send "AlgoDeck" "Brak zapisanych zadań w workspace." 2>/dev/null || true
    exit 0
fi

if [ "$ACTION" = "menu" ]; then
    if [ -n "$CURRENT" ]; then
        echo "$CURRENT" | tr '[:lower:]' '[:upper:]' > /tmp/algodeck_active_task.txt
    fi
    gdbus call --session --dest com.core447.StreamController --object-path /com/core447/StreamController --method com.core447.StreamController.ChangePage "$SERIAL" "ALGO_MENU" >/dev/null 2>&1 || true
    exit 0
fi

if [ "$ACTION" = "to" ]; then
    TARGET="$(echo "$CURRENT" | tr '[:upper:]' '[:lower:]')"
else
    INDEX=0
    if [ -n "$CURRENT" ]; then
        CURRENT=$(echo "$CURRENT" | tr '[:upper:]' '[:lower:]')
        for i in "${!PROJECTS[@]}"; do
            if [ "${PROJECTS[$i]}" = "$CURRENT" ]; then
                INDEX=$i
                break
            fi
        done
    fi

    TOTAL=${#PROJECTS[@]}
    if [ "$ACTION" = "next" ]; then
        NEW_INDEX=$(( (INDEX + 1) % TOTAL ))
    elif [ "$ACTION" = "prev" ]; then
        NEW_INDEX=$(( (INDEX - 1 + TOTAL) % TOTAL ))
    else
        NEW_INDEX=$INDEX
    fi
    TARGET="${PROJECTS[$NEW_INDEX]}"
fi

TARGET_UPPER=$(echo "$TARGET" | tr '[:lower:]' '[:upper:]')
echo "$TARGET_UPPER" > /tmp/algodeck_active_task.txt

gdbus call --session --dest com.core447.StreamController --object-path /com/core447/StreamController --method com.core447.StreamController.ChangePage "$SERIAL" "$TARGET_UPPER" >/dev/null 2>&1 || true

curl -s -X POST "http://127.0.0.1:8080/api/set-active/$TARGET" >/dev/null 2>&1 || true

if [ -f "$HOME/.local/bin/sd_algo_code.sh" ]; then
    "$HOME/.local/bin/sd_algo_code.sh" "$WORKSPACE/$TARGET" "$WORKSPACE/$TARGET/$TARGET.cpp" >/dev/null 2>&1 || true
fi

notify-send -u low "AlgoDeck" "📂 Aktywne zadanie: $TARGET_UPPER" 2>/dev/null || true
EOF
chmod +x "$HOME/.local/bin/sd_algo_switch.sh"

# 7e. Skrypt sd_algo_menu_back.sh
cat > "$HOME/.local/bin/sd_algo_menu_back.sh" << 'EOF'
#!/usr/bin/env bash
SERIAL="A00SA6042JGA63"
TARGET="AKC"
if [ -f /tmp/algodeck_active_task.txt ]; then
    TARGET=$(cat /tmp/algodeck_active_task.txt)
fi
gdbus call --session --dest com.core447.StreamController --object-path /com/core447/StreamController --method com.core447.StreamController.ChangePage "$SERIAL" "$TARGET" >/dev/null 2>&1 || true
EOF
chmod +x "$HOME/.local/bin/sd_algo_menu_back.sh"

# 7f. Skrypt uruchamiający przeglądarkę z wczytanym rozszerzeniem AlgoDeck
cat > "$HOME/.local/bin/algodeck-browser" << EOF
#!/usr/bin/env bash
EXT_DIR="$INSTALL_DIR/extension"
if command -v google-chrome &> /dev/null; then
    exec google-chrome --load-extension="\$EXT_DIR" "http://localhost:8080" "\$@"
elif command -v brave-browser &> /dev/null; then
    exec brave-browser --load-extension="\$EXT_DIR" "http://localhost:8080" "\$@"
elif command -v chromium &> /dev/null; then
    exec chromium --load-extension="\$EXT_DIR" "http://localhost:8080" "\$@"
else
    xdg-open "http://localhost:8080"
fi
EOF
chmod +x "$HOME/.local/bin/algodeck-browser"

# Dodanie ~/.local/bin do PATH w bashrc i zshrc jeśli brak
for rc in "$HOME/.bashrc" "$HOME/.zshrc"; do
    if [ -f "$rc" ] && ! grep -q '\$HOME/\.local/bin' "$rc"; then
        echo 'export PATH="$HOME/.local/bin:$PATH"' >> "$rc"
    fi
done

# 8. Konfiguracja usługi systemd użytkownika (Autostart w tle)
echo -e "\n${CYAN}[8/8] Konfiguracja i uruchomienie usługi AlgoDeck w systemd...${NC}"
mkdir -p "$HOME/.config/systemd/user"
cat > "$HOME/.config/systemd/user/algodeck.service" << EOF
[Unit]
Description=AlgoDeck ⚡ Koło MAP - Daemon środowiska olimpijskiego
After=network.target

[Service]
Type=simple
WorkingDirectory=$INSTALL_DIR
ExecStart=$HOME/.local/bin/algodeck
Restart=always
RestartSec=3s

[Install]
WantedBy=default.target
EOF

systemctl --user daemon-reload
systemctl --user enable algodeck.service
systemctl --user restart algodeck.service || true

# Aktywator w menu aplikacji (.desktop)
mkdir -p "$HOME/.local/share/applications"
cat > "$HOME/.local/share/applications/algodeck.desktop" << EOF
[Desktop Entry]
Name=AlgoDeck ⚡ Koło MAP
Comment=Zautomatyzowane środowisko do algorytmiki C++ i Stream Decka
Exec=$HOME/.local/bin/algodeck-browser
Icon=applications-development
Terminal=false
Type=Application
Categories=Development;IDE;Education;
StartupNotify=true
EOF

echo -e "\n${GREEN}${BOLD}==============================================================================${NC}"
echo -e "${GREEN}${BOLD}          🎉 INSTALACJA ALGODECK ZAKOŃCZONA SUKCESEM!                        ${NC}"
echo -e "${GREEN}${BOLD}==============================================================================${NC}\n"
echo -e "${BOLD}Środowisko jest w 100% gotowe i działa w tle jako usługa systemd.${NC}\n"
echo -e "🚀 ${BOLD}Jak korzystać z AlgoDeck:${NC}"
echo -e "  1. ${CYAN}Rozszerzenie przeglądarki:${NC}"
echo -e "     - Wpisz w terminalu: ${BOLD}algodeck-browser${NC} (otworzy przeglądarkę z wczytanym rozszerzeniem)"
echo -e "     - Lub w Chrome/Brave wejdź na: ${BOLD}chrome://extensions${NC} -> włącz 'Tryb programisty' ->"
echo -e "       kliknij 'Załaduj rozpakowane' i wybierz folder: ${CYAN}$INSTALL_DIR/extension${NC}"
echo -e "  2. ${CYAN}W oknie rozszerzenia:${NC}"
echo -e "     - Przeciągnij plik PDF zadania i/lub archiwum ZIP z testami (np. chwocen.zip, akcje_testy.zip)"
echo -e "     - Lub skorzystaj z zakładki 'Ręczny Workspace' (wystarczy podać nazwę zadania!)"
echo -e "     - Kliknij 'Utwórz Workspace' — automatycznie otwiera się VS Code z gotowym szablonem C++20!"
echo -e "  3. ${CYAN}Stream Deck (Minimalistyczne Symbole):${NC}"
echo -e "     - [${CYAN}</>${NC}] VS Code  |  [${GREEN}⚗${NC}] Testy (test.sh)  |  [${YELLOW}▶${NC}] Odpal  |  [${RED}⏹${NC}] Kill  |  [${CYAN}⊞${NC}] Galeria"
echo -e "     - Klawisze posiadają czyste, minimalistyczne symbole bez zbędnych etykiet tekstowych."
echo -e "  4. ${CYAN}Lokalny panel Web UI:${NC}"
echo -e "     - Zawsze dostępny również bezpośrednio pod adresem: ${CYAN}http://localhost:8080${NC}\n"
