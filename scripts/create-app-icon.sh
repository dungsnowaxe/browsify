#!/usr/bin/env bash
set -euo pipefail

# Render the canonical SVG with AppKit, preserving transparent corners.
ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
WORK_DIR="$(mktemp -d "${TMPDIR:-/tmp}/browsify-app-icon.XXXXXX")"
trap 'rm -rf "$WORK_DIR"' EXIT
ICONSET_DIR="$WORK_DIR/browsify.iconset"
mkdir -p "$ICONSET_DIR"

swift "$ROOT_DIR/scripts/render-icon.swift" "$ROOT_DIR/assets/brand/browsify-logo.svg" "$WORK_DIR/browsify-logo.svg.png"
for SIZE in 16 32 128 256 512; do
  sips -s format png -z "$SIZE" "$SIZE" "$WORK_DIR/browsify-logo.svg.png" \
    --out "$ICONSET_DIR/icon_${SIZE}x${SIZE}.png" >/dev/null
  sips -s format png -z "$((SIZE * 2))" "$((SIZE * 2))" "$WORK_DIR/browsify-logo.svg.png" \
    --out "$ICONSET_DIR/icon_${SIZE}x${SIZE}@2x.png" >/dev/null
done
iconutil -c icns "$ICONSET_DIR" -o "$ROOT_DIR/apps/browsify/assets/Resources/browsify.icns"
echo "Generated Browsify app icon from assets/brand/browsify-logo.svg"
