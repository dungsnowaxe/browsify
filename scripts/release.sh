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

# Apple Developer ID signing and notarization are deferred.
# GitHub release builds embed BROWSIFY_UPDATE_PUBLIC_KEY; the publish job signs the archive.
export MACOSX_DEPLOYMENT_TARGET=${MACOSX_DEPLOYMENT_TARGET:-13.0}
case "$MACOSX_DEPLOYMENT_TARGET" in
    12.0) archive=Browsify-arm64.tar.gz ;;
    13.0) archive=Browsify-macos13-arm64.tar.gz ;;
    *) echo "Unsupported macOS deployment target: $MACOSX_DEPLOYMENT_TARGET" >&2; exit 1 ;;
esac
./scripts/build.sh
architectures=$(lipo -archs apps/browsify/build/Browsify.app/Contents/MacOS/browsify)
if [ "$architectures" != arm64 ]; then
    echo "release: expected an arm64-only executable, found: $architectures" >&2
    exit 1
fi
# Seal the completed app bundle, not just its linker-signed executable.
codesign --force --sign - apps/browsify/build/Browsify.app
mkdir -p dist
rm -f dist/Browsify-arm64.tar.gz dist/Browsify-macos13-arm64.tar.gz
COPYFILE_DISABLE=1 tar -czf "dist/$archive" -C apps/browsify/build Browsify.app
printf '%s\n' "$archive" > dist/archive-name.txt
printf '%s\n' "$MACOSX_DEPLOYMENT_TARGET" > dist/minimum-macos.txt
(
    cd dist
    shasum -a 256 "$archive" > SHA256SUMS
)
echo "release: ad hoc-signed Apple Silicon bundle at apps/browsify/build/Browsify.app"
echo "release: archive and checksums in dist/"
