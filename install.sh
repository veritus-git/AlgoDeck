#!/usr/bin/env bash
# ==============================================================================
# AlgoDeck ⚡ Koło MAP - Linux One-Click Entrypoint
# ==============================================================================
set -e

DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" 2>/dev/null && pwd || true)"
if [ -n "$DIR" ] && [ -f "$DIR/bootstrap.sh" ]; then
    exec "$DIR/bootstrap.sh" "$@"
else
    TMP_BOOTSTRAP="$(mktemp /tmp/algodeck_bootstrap.XXXXXX.sh 2>/dev/null || echo "/tmp/algodeck_bootstrap_$$.sh")"
    trap 'rm -f "$TMP_BOOTSTRAP" 2>/dev/null || true' EXIT
    if curl -fsSL https://raw.githubusercontent.com/veritus-git/AlgoDeck/main/bootstrap.sh -o "$TMP_BOOTSTRAP"; then
        chmod +x "$TMP_BOOTSTRAP"
        exec bash "$TMP_BOOTSTRAP" "$@"
    else
        exec bash <(curl -fsSL https://raw.githubusercontent.com/veritus-git/AlgoDeck/main/bootstrap.sh) "$@"
    fi
fi
