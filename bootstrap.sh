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
        cairo-devel \
        pkgconf-pkg-config \
        pkgconfig \
        gobject-introspection-devel \
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
        libcairo2-dev \
        pkg-config \
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

# Przygotowanie katalogu i skopiowanie pre-renderowanych ikon
mkdir -p "$HOME/.var/app/com.core447.StreamController/data/custom_icons"
if [ -d "$INSTALL_DIR/frontend/icons" ]; then
    cp -n "$INSTALL_DIR/frontend/icons/"*.png "$HOME/.var/app/com.core447.StreamController/data/custom_icons/" 2>/dev/null || true
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

# 7b. Skrypt sd_algo_code.sh (Otwieranie / przełączanie okna VS Code)
cat > "$HOME/.local/bin/sd_algo_code.sh" << 'EOF'
#!/usr/bin/env bash
TARGET_DIR="${1:-$HOME/algodeck-workspace}"
TARGET_FILE="${2:-}"
PROB_NAME=$(basename "$TARGET_DIR")

CONFIG_FILE="$HOME/.config/algodeck/settings.json"
VSCODE_MODE="single_window"
WORKSPACE_DIR="$HOME/algodeck-workspace"

if [ -f "$CONFIG_FILE" ]; then
    MODE_FROM_CFG=$(grep -o '"vscode_mode": *"[^"]*"' "$CONFIG_FILE" 2>/dev/null | cut -d'"' -f4)
    [ -n "$MODE_FROM_CFG" ] && VSCODE_MODE="$MODE_FROM_CFG"
    DIR_FROM_CFG=$(grep -o '"workspace_dir": *"[^"]*"' "$CONFIG_FILE" 2>/dev/null | cut -d'"' -f4)
    [ -n "$DIR_FROM_CFG" ] && WORKSPACE_DIR="$DIR_FROM_CFG"
fi

if [ "$VSCODE_MODE" = "single_window" ]; then
    WID=$(DISPLAY=:0 wmctrl -lx 2>/dev/null | grep -i "code\.code" | awk '{print $1}' | head -n 1)
    if [ -n "$WID" ]; then
        if [ -n "$TARGET_FILE" ] && [ -f "$TARGET_FILE" ]; then
            /usr/bin/code --reuse-window "$TARGET_FILE" >/dev/null 2>&1 &
        fi
        DISPLAY=:0 wmctrl -i -r "$WID" -b add,maximized_vert,maximized_horz 2>/dev/null || true
        DISPLAY=:0 wmctrl -i -a "$WID" 2>/dev/null || true
        exit 0
    fi

    if [ -n "$TARGET_FILE" ] && [ -f "$TARGET_FILE" ]; then
        nohup /usr/bin/code "$WORKSPACE_DIR" "$TARGET_FILE" >/dev/null 2>&1 &
    else
        nohup /usr/bin/code "$WORKSPACE_DIR" >/dev/null 2>&1 &
    fi
else
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
fi

for i in {1..20}; do
    WID=$(DISPLAY=:0 wmctrl -lx 2>/dev/null | grep -i "code\.code" | awk '{print $1}' | head -n 1)
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

if [ -n "$PROB" ]; then
    echo "${PROB^^}" > /tmp/algodeck_active_task.txt
fi

if [ -n "$PDIR" ] && [ -n "$PROB" ] && [ -f "$PDIR/$PROB.cpp" ]; then
    code --reuse-window "$PDIR/$PROB.cpp" >/dev/null 2>&1 &
fi

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
    sleep 0.15
    DISPLAY=:0 xdotool key --clearmodifiers ctrl+shift+b 2>/dev/null || true
fi
EOF
chmod +x "$HOME/.local/bin/sd_algo_run_vscode.sh"

# 7d. Skrypt sd_algo_test_vscode.sh (Wbudowany terminal VS Code: F6 / workbench.action.tasks.test)
cat > "$HOME/.local/bin/sd_algo_test_vscode.sh" << 'EOF'
#!/usr/bin/env bash
PDIR="$1"
PROB="$2"

if [ -n "$PROB" ]; then
    echo "${PROB^^}" > /tmp/algodeck_active_task.txt
fi

if [ -n "$PDIR" ] && [ -n "$PROB" ] && [ -f "$PDIR/$PROB.cpp" ]; then
    code --reuse-window "$PDIR/$PROB.cpp" >/dev/null 2>&1 &
fi

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
    sleep 0.15
    DISPLAY=:0 xdotool key --clearmodifiers F6 2>/dev/null || true
fi
EOF
chmod +x "$HOME/.local/bin/sd_algo_test_vscode.sh"

# 7e. Skrypt sd_algo_panel.sh (Panel kontrolny AlgoDeck / Ustawienia)
cat > "$HOME/.local/bin/sd_algo_panel.sh" << 'EOF'
#!/usr/bin/env bash
TAB="${1:---tab=settings}"
WID=$(DISPLAY=:0 wmctrl -l 2>/dev/null | grep -i "AlgoDeck ⚡ Centrum Kontroli" | awk '{print $1}' | head -n 1)

if [ -n "$WID" ]; then
    DISPLAY=:0 wmctrl -i -a "$WID" 2>/dev/null || true
else
    python3 "$HOME/.local/share/algodeck/scripts/panel_dialog.py" "$TAB" >/dev/null 2>&1 &
fi
EOF
chmod +x "$HOME/.local/bin/sd_algo_panel.sh"

# 7e2. Skrypt sd_algo_new_task.sh (Dodawanie nowego zadania)
cat > "$HOME/.local/bin/sd_algo_new_task.sh" << 'EOF'
#!/usr/bin/env bash
WID=$(DISPLAY=:0 wmctrl -l 2>/dev/null | grep -i "AlgoDeck ⚡ Centrum Kontroli" | awk '{print $1}' | head -n 1)

if [ -n "$WID" ]; then
    DISPLAY=:0 wmctrl -i -a "$WID" 2>/dev/null || true
else
    python3 "$HOME/.local/share/algodeck/scripts/panel_dialog.py" --tab=new >/dev/null 2>&1 &
fi
EOF
chmod +x "$HOME/.local/bin/sd_algo_new_task.sh"

# 7f. Skrypt sd_algo_switch.sh (Przełączanie zadań i galerii w kolejności chronologicznej)
cat > "$HOME/.local/bin/sd_algo_switch.sh" << 'EOF'
#!/usr/bin/env bash
set -e

ACTION="${1:-next}"
CURRENT="${2:-}"
WORKSPACE="$HOME/algodeck-workspace"
SERIAL="A00SA6042JGA63"

# 1. Pobierz kolejność zadań z API AlgoDeck (ścisła kolejność chronologiczna utworzenia)
PROJECTS=()
API_RES=$(curl -s --max-time 1 http://127.0.0.1:8080/api/problems 2>/dev/null || true)
if [ -n "$API_RES" ]; then
    P_LIST=$(python3 -c "import sys, json; data=json.loads(sys.stdin.read()); print(' '.join(p['problem_id'] for p in data.get('problems', [])))" <<< "$API_RES" 2>/dev/null || true)
    if [ -n "$P_LIST" ]; then
        PROJECTS=($P_LIST)
    fi
fi

# 2. Fallback gdy serwer jest wyłączony: sortuj według czasu utworzenia katalogu (najstarsze pierwsze)
if [ ${#PROJECTS[@]} -eq 0 ]; then
    while IFS= read -r dir; do
        [ -d "$dir" ] || continue
        bname=$(basename "$dir")
        [ "$bname" = "tests" ] && continue
        if [ -f "$dir/$bname.cpp" ] || [ -d "$dir/.algo" ]; then
            PROJECTS+=("$bname")
        fi
    done < <(ls -1v -tr "$WORKSPACE" 2>/dev/null)
fi

if [ ${#PROJECTS[@]} -eq 0 ]; then
    gdbus call --session --dest com.core447.StreamController --object-path /com/core447/StreamController --method com.core447.StreamController.ChangePage "$SERIAL" "ALGO_IDLE" >/dev/null 2>&1 || true
    exit 0
fi

if [ "$ACTION" = "menu" ]; then
    if [ -n "$CURRENT" ]; then
        echo "${CURRENT^^}" > /tmp/algodeck_active_task.txt
    fi
    gdbus call --session --dest com.core447.StreamController --object-path /com/core447/StreamController --method com.core447.StreamController.ChangePage "$SERIAL" "ALGO_MENU" >/dev/null 2>&1 || true
    exit 0
fi

if [ "$ACTION" = "to" ]; then
    TARGET="${CURRENT,,}"
else
    INDEX=0
    if [ -n "$CURRENT" ]; then
        CURRENT="${CURRENT,,}"
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

TARGET_UPPER="${TARGET^^}"
echo "$TARGET_UPPER" > /tmp/algodeck_active_task.txt

gdbus call --session --dest com.core447.StreamController --object-path /com/core447/StreamController --method com.core447.StreamController.ChangePage "$SERIAL" "$TARGET_UPPER" >/dev/null 2>&1 || true

curl -s -X POST "http://127.0.0.1:8080/api/set-active/$TARGET" >/dev/null 2>&1 || true

if [ -f "$HOME/.local/bin/sd_algo_code.sh" ]; then
    "$HOME/.local/bin/sd_algo_code.sh" "$WORKSPACE/$TARGET" "$WORKSPACE/$TARGET/$TARGET.cpp" >/dev/null 2>&1 || true
fi
EOF
chmod +x "$HOME/.local/bin/sd_algo_switch.sh"

# 7g. Skrypt sd_algo_menu_back.sh
cat > "$HOME/.local/bin/sd_algo_menu_back.sh" << 'EOF'
#!/usr/bin/env bash
SERIAL="A00SA6042JGA63"
WORKSPACE="$HOME/algodeck-workspace"
TARGET=""
if [ -f /tmp/algodeck_active_task.txt ]; then
    TARGET=$(cat /tmp/algodeck_active_task.txt 2>/dev/null || echo "")
fi
if [ -z "$TARGET" ]; then
    for dir in "$WORKSPACE"/*; do
        [ -d "$dir" ] || continue
        bname=$(basename "$dir")
        [ "$bname" = "tests" ] && continue
        if [ -f "$dir/$bname.cpp" ] || [ -d "$dir/.algo" ]; then
            TARGET="${bname^^}"
            break
        fi
    done
fi
if [ -z "$TARGET" ]; then
    TARGET="ALGO_IDLE"
fi

gdbus call --session --dest com.core447.StreamController --object-path /com/core447/StreamController --method com.core447.StreamController.ChangePage "$SERIAL" "$TARGET" >/dev/null 2>&1 || true
EOF
chmod +x "$HOME/.local/bin/sd_algo_menu_back.sh"

# 7h. Skrypt sd_algo_new_task.sh (Okno modalne dodawania nowego zadania)
cat > "$HOME/.local/bin/sd_algo_new_task.sh" << 'EOF'
#!/usr/bin/env bash
SCRIPT="$HOME/.local/share/algodeck/scripts/add_task_dialog.py"
if [ ! -f "$SCRIPT" ]; then
    SCRIPT="/home/linux/.gemini/antigravity-ide/scratch/algodeck/scripts/add_task_dialog.py"
fi
DISPLAY=:0 python3 "$SCRIPT" >/dev/null 2>&1 &
EOF
chmod +x "$HOME/.local/bin/sd_algo_new_task.sh"

# 7i. Skróty klawiszowe VS Code (Ctrl+Alt+R dla Run, Ctrl+Alt+E dla Test)
python3 -c '
import json, os
p = os.path.expanduser("~/.config/Code/User/keybindings.json")
try:
    os.makedirs(os.path.dirname(p), exist_ok=True)
    try:
        with open(p, "r", encoding="utf-8") as f:
            kb = json.load(f)
    except Exception:
        kb = []
    
    keys = {item.get("key") for item in kb}
    changed = False
    if "ctrl+alt+r" not in keys:
        kb.append({"key": "ctrl+alt+r", "command": "workbench.action.terminal.sendSequence", "args": {"text": "clear && bash .algo/run.sh\r"}})
        changed = True
    if "ctrl+alt+e" not in keys:
        kb.append({"key": "ctrl+alt+e", "command": "workbench.action.terminal.sendSequence", "args": {"text": "clear && bash .algo/test.sh\r"}})
        changed = True
    if changed:
        with open(p, "w", encoding="utf-8") as f:
            json.dump(kb, f, indent=4)
except Exception:
    pass
' 2>/dev/null || true

# 7j. Skrypt uruchamiający przeglądarkę z wczytanym rozszerzeniem AlgoDeck
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
