#!/usr/bin/env bash
# Quick development runner for AlgoDeck
set -e
PROJECT_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$PROJECT_ROOT"

if [ -d ".venv" ]; then
    source .venv/bin/activate
fi

export PYTHONPATH="$PROJECT_ROOT"
echo "🚀 Uruchamianie AlgoDeck pod adresem http://127.0.0.1:8080..."
exec python3 -m uvicorn backend.server:app --host 127.0.0.1 --port 8080 --reload
