#!/usr/bin/env bash
# Otwieranie Panelu Kontrolnego AlgoDeck (Preferowany natywny Rust GUI)
TAB="${1:---tab=settings}"

WID=$(DISPLAY=:0 wmctrl -l 2>/dev/null | grep -i "AlgoDeck ⚡ Centrum Kontroli" | awk '{print $1}' | head -n 1)

if [ -n "$WID" ]; then
    DISPLAY=:0 wmctrl -i -a "$WID" 2>/dev/null || true
else
    if [ -x "$HOME/.local/bin/algodeck-gui" ]; then
        "$HOME/.local/bin/algodeck-gui" "$TAB" >/dev/null 2>&1 &
    elif [ -x "$HOME/.local/share/algodeck/gui/target/release/algodeck-gui" ]; then
        "$HOME/.local/share/algodeck/gui/target/release/algodeck-gui" "$TAB" >/dev/null 2>&1 &
    else
        python3 "$HOME/.local/share/algodeck/scripts/panel_dialog.py" "$TAB" >/dev/null 2>&1 &
    fi
fi
