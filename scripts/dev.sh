#!/bin/bash
# Run with the normal TOML config, built-in defaults, or a particular TOML file.
set -eu

cd "$(dirname "$0")/.."
binary="apps/cli/build/Browsify.app/Contents/MacOS/browsify"
if [[ ! -x "$binary" && -x target/release/browsify ]]; then
  binary="target/release/browsify"
elif [[ ! -x "$binary" ]]; then
  binary="target/debug/browsify"
fi

case "${1:-normal}" in
  0|normal) exec "$binary" --window ;;
  1|none) exec "$binary" --window --no-config ;;
  *.toml) exec "$binary" --window --config "$1" ;;
  *) echo "Usage: $0 [normal|none|path/to/config.toml]" >&2; exit 2 ;;
esac
