#!/bin/bash
set -e

./scripts/build.sh
mkdir -p dist

if [[ -n "$CODESIGN_IDENTITY" ]]; then
  codesign --deep --force --options runtime \
    --sign "$CODESIGN_IDENTITY" \
    ./apps/browsify/build/Browsify.app
fi

if [[ -f .env ]]; then
  export $(cat .env | xargs)
fi
gon scripts/gon-config.json
