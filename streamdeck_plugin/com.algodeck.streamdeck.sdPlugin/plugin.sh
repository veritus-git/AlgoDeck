#!/usr/bin/env bash
DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
if [ -f "$HOME/.local/share/algodeck/venv/bin/python3" ]; then
    exec "$HOME/.local/share/algodeck/venv/bin/python3" "$DIR/plugin.py" "$@"
elif [ -f "$HOME/.algodeck/venv/bin/python3" ]; then
    exec "$HOME/.algodeck/venv/bin/python3" "$DIR/plugin.py" "$@"
else
    exec python3 "$DIR/plugin.py" "$@"
fi
