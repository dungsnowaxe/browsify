#!/bin/bash
set -e

# Unsigned personal build. Signing + notarization are intentionally deferred until
# a Developer ID certificate is configured; re-add codesign/gon steps then.
./scripts/build.sh
mkdir -p dist
cd apps/browsify/build
tar -czf ../../dist/Browsify-universal.tar.gz Browsify.app
echo "release: unsigned bundle at apps/browsify/build/Browsify.app"
echo "release: archived to dist/Browsify-universal.tar.gz"
