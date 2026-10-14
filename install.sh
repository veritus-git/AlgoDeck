#!/usr/bin/env bash
# ==============================================================================
# AlgoDeck ⚡ Koło MAP - Linux One-Click Entrypoint
# ==============================================================================
set -e

DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" 2>/dev/null && pwd || true)"
if [ -n "$DIR" ] && [ -f "$DIR/bootstrap.sh" ]; then
    exec "$DIR/bootstrap.sh" "$@"
else
    exec bash -c "$(curl -fsSL https://raw.githubusercontent.com/veritus-git/AlgoDeck/main/bootstrap.sh)" "$@"
fi
