#!/usr/bin/env bash
# ==============================================================================
# AlgoDeck ⚡ Koło MAP - Oficjalny Instalator dla macOS (Apple Silicon & Intel)
# 100% Rust Architecture — Zero Python, Zero StreamController, Zero Elgato App
# ==============================================================================

set -e

RED='\033[0;31m'
GREEN='\033[0;32m'
CYAN='\033[0;36m'
YELLOW='\033[1;33m'
BOLD='\033[1m'
NC='\033[0m'

echo -e "\n${CYAN}${BOLD}==============================================================================${NC}"
echo -e "${CYAN}${BOLD}   ⚡ AlgoDeck (Koło MAP) — 100% Rust Installer (macOS Apple Silicon & Intel) ${NC}"
echo -e "${CYAN}${BOLD}==============================================================================${NC}\n"

if [ "$(uname -s)" != "Darwin" ]; then
    echo -e "${RED}Błąd: Ten instalator jest przeznaczony wyłącznie dla systemu macOS.${NC}"
    exit 1
fi

ARCH_TYPE=$(uname -m)
echo -e "${GREEN}✓ Wykryto system macOS (${ARCH_TYPE})${NC}"

# 1. Xcode CLI Tools
if ! xcode-select -p >/dev/null 2>&1; then
    echo -e "${YELLOW}Instalacja narzędzi Xcode Command Line Tools...${NC}"
    xcode-select --install || true
    echo -e "${YELLOW}Uruchom skrypt ponownie po zakończeniu instalacji Xcode Tools.${NC}"
    exit 0
fi

# 2. Rust & Cargo
if ! command -v cargo >/dev/null 2>&1; then
    echo -e "${YELLOW}Instalacja Rust (rustup)...${NC}"
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --profile minimal
    export PATH="$HOME/.cargo/bin:$PATH"
fi

# 3. Kompilacja i instalacja binarki algodeck
REPO_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
mkdir -p "$HOME/.local/bin"

echo -e "${CYAN}Kompilacja natywnej binarki AlgoDeck w Rust...${NC}"
(cd "$REPO_DIR" && cargo build --release)
cp "$REPO_DIR/target/release/algodeck" "$HOME/.local/bin/algodeck"
chmod +x "$HOME/.local/bin/algodeck"

# 4. Native Messaging Host dla Chrome na macOS
WRAPPER_PATH="$HOME/.local/bin/algodeck-native-host"
cat << 'EOF' > "$WRAPPER_PATH"
#!/bin/sh
exec "$HOME/.local/bin/algodeck" native-host
EOF
chmod +x "$WRAPPER_PATH"

CHROME_TARGET="$HOME/Library/Application Support/Google/Chrome/NativeMessagingHosts"
mkdir -p "$CHROME_TARGET"
cat << EOF > "$CHROME_TARGET/com.algodeck.launcher.json"
{
  "name": "com.algodeck.launcher",
  "description": "AlgoDeck 100% Rust Native Messaging Host for macOS",
  "path": "$WRAPPER_PATH",
  "type": "stdio",
  "allowed_origins": [
    "chrome-extension://cdgdnkfkkbadljdgjjjbaefefcbelplf/",
    "chrome-extension://jnfimdneicefaomnlikmogffogcadabf/"
  ]
}
EOF

echo -e "\n${GREEN}${BOLD}✓ AlgoDeck v2.0.0 (100% Rust) pomyślnie zainstalowany na macOS!${NC}"
echo -e "• Uruchom 'algodeck' aby otworzyć panel"
echo -e "• Stream Deck: Bezpośrednia obsługa USB HID bez oficjalnej aplikacji Elgato"
