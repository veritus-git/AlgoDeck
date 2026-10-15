#!/usr/bin/env bash
TARGET_DIR="${1:-$HOME/AlgoDeckWorkspace}"
TARGET_FILE="${2:-}"
PROB_NAME=$(basename "$TARGET_DIR")

CONFIG_FILE="$HOME/.config/algodeck/settings.json"
VSCODE_MODE="single_window"
WORKSPACE_DIR="$HOME/AlgoDeckWorkspace"
OPEN_PDF_SPLIT="true"
PDF_POSITION="right"
SPLIT_RATIO="60"

if [ -f "$CONFIG_FILE" ]; then
    MODE_FROM_CFG=$(grep -o '"vscode_mode": *"[^"]*"' "$CONFIG_FILE" 2>/dev/null | cut -d'"' -f4)
    [ -n "$MODE_FROM_CFG" ] && VSCODE_MODE="$MODE_FROM_CFG"
    DIR_FROM_CFG=$(grep -o '"workspace_dir": *"[^"]*"' "$CONFIG_FILE" 2>/dev/null | cut -d'"' -f4)
    [ -n "$DIR_FROM_CFG" ] && WORKSPACE_DIR="$DIR_FROM_CFG"
    PDF_SPLIT_CFG=$(grep -o '"open_pdf_in_vscode": *[^,}]*' "$CONFIG_FILE" 2>/dev/null | awk '{print $2}')
    [ -n "$PDF_SPLIT_CFG" ] && OPEN_PDF_SPLIT="$PDF_SPLIT_CFG"
    PDF_POS_CFG=$(grep -o '"pdf_position": *"[^"]*"' "$CONFIG_FILE" 2>/dev/null | cut -d'"' -f4)
    [ -n "$PDF_POS_CFG" ] && PDF_POSITION="$PDF_POS_CFG"
    RATIO_CFG=$(grep -o '"editor_split_ratio": *[0-9]*' "$CONFIG_FILE" 2>/dev/null | awk '{print $2}')
    [ -n "$RATIO_CFG" ] && SPLIT_RATIO="$RATIO_CFG"
fi

[ -n "$ALGODECK_PDF_POS" ] && PDF_POSITION="$ALGODECK_PDF_POS"
[ -n "$ALGODECK_SPLIT_RATIO" ] && SPLIT_RATIO="$ALGODECK_SPLIT_RATIO"

DISP="${DISPLAY:-:0}"
CODE_BIN=$(command -v code || echo "/usr/bin/code")

[ -n "$ALGODECK_VSCODE_MODE" ] && VSCODE_MODE="$ALGODECK_VSCODE_MODE"

if [ "$VSCODE_MODE" = "separate_windows" ]; then
    OPEN_DIR="$TARGET_DIR"
    WIN_FLAG="--new-window"
else
    OPEN_DIR="$WORKSPACE_DIR"
    WIN_FLAG="--reuse-window"
fi

if [ -n "$TARGET_FILE" ] && [ -f "$TARGET_FILE" ]; then
    "$CODE_BIN" $WIN_FLAG "$OPEN_DIR" "$TARGET_FILE" >/dev/null 2>&1 &
else
    "$CODE_BIN" $WIN_FLAG "$OPEN_DIR" >/dev/null 2>&1 &
fi

# Geometria monitora głównego (Primary Display)
PRIM_LINE=$(xrandr --current 2>/dev/null | grep " connected primary" || xrandr --current 2>/dev/null | grep " connected" | head -n 1)
GEOM=$(echo "$PRIM_LINE" | grep -oP '\d+x\d+\+\d+\+\d+')
MON_W=$(echo "$GEOM" | cut -d'x' -f1)
MON_H=$(echo "$GEOM" | cut -d'x' -f2 | cut -d'+' -f1)
MON_X=$(echo "$GEOM" | cut -d'+' -f2)
MON_Y=$(echo "$GEOM" | cut -d'+' -f3)

[ -z "$MON_W" ] && MON_W=1920
[ -z "$MON_H" ] && MON_H=1080
[ -z "$MON_X" ] && MON_X=0
[ -z "$MON_Y" ] && MON_Y=0

SAFE_H=$(( MON_H - 35 ))
[ "$SAFE_H" -lt 400 ] && SAFE_H="$MON_H"

PDF_FILE=""
if [ -f "$TARGET_DIR/$PROB_NAME.pdf" ]; then
    PDF_FILE="$TARGET_DIR/$PROB_NAME.pdf"
elif [ -f "$TARGET_DIR/.algo/statement.pdf" ]; then
    PDF_FILE="$TARGET_DIR/.algo/statement.pdf"
else
    PDF_FILE=$(find "$TARGET_DIR" -maxdepth 2 -name "*.pdf" 2>/dev/null | head -n 1)
fi

(
    for attempt in {1..14}; do
        sleep 0.25
        WID=$(DISPLAY="$DISP" wmctrl -lx 2>/dev/null | grep -i "code\.code" | awk '{print $1}' | head -n 1)
        if [ -n "$WID" ]; then
            if [ "$OPEN_PDF_SPLIT" = "true" ] && [ -n "$PDF_FILE" ] && [ -n "$MON_W" ]; then
                RATIO=${SPLIT_RATIO:-60}
                if [ "$RATIO" -lt 20 ] || [ "$RATIO" -gt 80 ]; then
                    RATIO=60
                fi

                VSCODE_W=$(( MON_W * RATIO / 100 ))
                PDF_W=$(( MON_W - VSCODE_W ))

                if [ "$PDF_POSITION" = "left" ]; then
                    # PDF z lewej, VS Code z prawej
                    PDF_X=$MON_X
                    VSCODE_X=$(( MON_X + PDF_W ))
                else
                    # VS Code z lewej, PDF z prawej (domyślnie)
                    VSCODE_X=$MON_X
                    PDF_X=$(( MON_X + VSCODE_W ))
                fi

                DISPLAY="$DISP" wmctrl -i -r "$WID" -b remove,maximized_vert,maximized_horz 2>/dev/null || true
                DISPLAY="$DISP" wmctrl -i -r "$WID" -e "0,$VSCODE_X,$MON_Y,$VSCODE_W,$SAFE_H" 2>/dev/null || true
                DISPLAY="$DISP" wmctrl -i -a "$WID" 2>/dev/null || true

                # Uruchom czytnik PDF jeśli jeszcze nie jest otwarty
                if ! pgrep -f "$PDF_FILE" >/dev/null 2>&1; then
                    if command -v xreader >/dev/null 2>&1; then
                        xreader "$PDF_FILE" >/dev/null 2>&1 &
                    elif command -v atril >/dev/null 2>&1; then
                        atril "$PDF_FILE" >/dev/null 2>&1 &
                    elif command -v evince >/dev/null 2>&1; then
                        evince "$PDF_FILE" >/dev/null 2>&1 &
                    else
                        xdg-open "$PDF_FILE" >/dev/null 2>&1 &
                    fi
                fi

                # Dopasuj okno PDF do wyznaczonej pozycji z pętlą oczekiwania
                PDF_WID=""
                for p_attempt in {1..10}; do
                    sleep 0.2
                    PDF_WID=$(DISPLAY="$DISP" wmctrl -lx 2>/dev/null | grep -iE "xreader|atril|evince|okular" | tail -n 1 | awk '{print $1}')
                    [ -n "$PDF_WID" ] && break
                done
                if [ -n "$PDF_WID" ]; then
                    DISPLAY="$DISP" wmctrl -i -r "$PDF_WID" -b remove,maximized_vert,maximized_horz 2>/dev/null || true
                    DISPLAY="$DISP" wmctrl -i -r "$PDF_WID" -e "0,$PDF_X,$MON_Y,$PDF_W,$SAFE_H" 2>/dev/null || true
                fi
                # Przywróć fokus na kodzie
                DISPLAY="$DISP" wmctrl -i -a "$WID" 2>/dev/null || true
            else
                # Domyślnie pełny ekran na kod
                DISPLAY="$DISP" wmctrl -i -r "$WID" -b add,maximized_vert,maximized_horz 2>/dev/null || true
                DISPLAY="$DISP" wmctrl -i -a "$WID" 2>/dev/null || true
                DISPLAY="$DISP" wmctrl -i -R "$WID" 2>/dev/null || true
                DISPLAY="$DISP" xdotool windowactivate "$WID" 2>/dev/null || true
                DISPLAY="$DISP" xdotool windowraise "$WID" 2>/dev/null || true
            fi
            break
        fi
    done
) &
