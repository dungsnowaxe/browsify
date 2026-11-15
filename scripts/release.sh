#!/bin/bash
set -euo pipefail

cd "$(dirname "$0")/.."
VERSION=$(cargo metadata --no-deps --locked --format-version 1 | jq -r '.packages[] | select(.name == "browsify") | .version')
if [[ ! "${VERSION}" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
    echo "release: only stable X.Y.Z versions are supported" >&2
    exit 1
fi
if [ -n "${RELEASE_TAG:-}" ] && [ "${RELEASE_TAG}" != "v${VERSION}" ]; then
    echo "release: tag ${RELEASE_TAG} does not match Cargo version v${VERSION}" >&2
    exit 1
fi

# Unsigned personal build. Developer ID signing and notarization are deferred.
BUILD_UNIVERSAL=1 ./scripts/build.sh
mkdir -p dist
tar -czf dist/Browsify-universal.tar.gz -C apps/browsify/build Browsify.app
(
    cd dist
    shasum -a 256 Browsify-universal.tar.gz > SHA256SUMS
)
echo "release: unsigned universal bundle at apps/browsify/build/Browsify.app"
echo "release: archive and checksums in dist/"
