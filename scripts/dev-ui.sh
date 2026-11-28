#!/bin/bash
# Start the Vite hot-reload UI connected to a running Browsify backend
# (production Browsify.app or a local `browsify ui` instance).
set -euo pipefail

cd "$(dirname "$0")/.."

usage() {
  cat <<'EOF'
Usage:
  ./scripts/dev-ui.sh "http://127.0.0.1:<port>/?token=<token>"
      Attach hot-reload UI to an already-running Browsify UI URL (e.g. production Browsify.app).

  ./scripts/dev-ui.sh --attach
      Start Vite without a preset backend and paste a Browsify UI URL into the dev bar.

  ./scripts/dev-ui.sh [--config <path> | --no-config]
      Start a local `browsify ui --no-open` server and wire Vite to it automatically.
      If another Browsify instance is already running, falls back to attach mode.
EOF
}

if [[ "${1:-}" == "-h" || "${1:-}" == "--help" ]]; then
  usage
  exit 0
fi

if [[ "${1:-}" == "--attach" ]]; then
  exec npm --prefix apps/ui run dev
fi

if [[ "${1:-}" == http://* || "${1:-}" == https://* ]]; then
  exec env BROWSIFY_UI_URL="$1" npm --prefix apps/ui run dev
fi

binary="apps/cli/build/Browsify.app/Contents/MacOS/browsify"
if [[ ! -x "$binary" && -x target/release/browsify ]]; then
  binary="target/release/browsify"
elif [[ ! -x "$binary" && -x target/debug/browsify ]]; then
  binary="target/debug/browsify"
elif [[ ! -x "$binary" ]]; then
  echo "Building local browsify binary (target/debug/browsify)…"
  cargo build --locked -p browsify-cli
  binary="target/debug/browsify"
fi

log_file=$(mktemp)
backend_pid=""

cleanup() {
  if [[ -n "$backend_pid" ]] && kill -0 "$backend_pid" 2>/dev/null; then
    kill "$backend_pid" 2>/dev/null || true
    wait "$backend_pid" 2>/dev/null || true
  fi
  rm -f "$log_file"
}
trap cleanup EXIT INT TERM

"$binary" ui --no-open "$@" >"$log_file" 2>&1 &
backend_pid=$!

ui_url=""
for _ in $(seq 1 100); do
  if grep -q "Browsify UI listening on http://" "$log_file" 2>/dev/null; then
    ui_url=$(sed -n 's/^Browsify UI listening on \(http:\/\/[^[:space:]]*\).*$/\1/p' "$log_file" | head -n 1)
    break
  fi
  if ! kill -0 "$backend_pid" 2>/dev/null; then
    break
  fi
  sleep 0.1
done

if [[ -z "$ui_url" ]]; then
  if grep -qi "already running" "$log_file" 2>/dev/null; then
    echo "Another Browsify instance is already running (holding the singleton lock)."
    echo "Starting Vite in attach mode — open Preferences from Browsify's menu bar and paste the URL with ?token=... into the top dev bar."
    backend_pid=""
    rm -f "$log_file"
    trap - EXIT INT TERM
    exec npm --prefix apps/ui run dev
  fi
  echo "Failed to start local Browsify UI server:" >&2
  cat "$log_file" >&2
  exit 1
fi

BROWSIFY_UI_URL="$ui_url" npm --prefix apps/ui run dev
