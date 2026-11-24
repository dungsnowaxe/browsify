#!/bin/bash
set -euo pipefail

cd "$(dirname "$0")/.."
export MACOSX_DEPLOYMENT_TARGET=${MACOSX_DEPLOYMENT_TARGET:-13.0}
VERSION=$(cargo metadata --no-deps --locked --format-version 1 | jq -r '.packages[] | select(.name == "browsify-cli") | .version')

# Build ui
(
    cd apps/ui
    npm run build
)

copy_assets() {
    local APP_DIR=$1
    mkdir -p "${APP_DIR}/Contents/Resources/ui"
    cp -r apps/ui/dist/* "${APP_DIR}/Contents/Resources/ui/"
    cp apps/cli/assets/Info.plist "${APP_DIR}/Contents/Info.plist"
    /usr/libexec/PlistBuddy -c "Set :CFBundleShortVersionString ${VERSION}" "${APP_DIR}/Contents/Info.plist"
    /usr/libexec/PlistBuddy -c "Set :CFBundleVersion ${VERSION}" "${APP_DIR}/Contents/Info.plist"
    /usr/libexec/PlistBuddy -c "Set :LSMinimumSystemVersion ${MACOSX_DEPLOYMENT_TARGET}" "${APP_DIR}/Contents/Info.plist"
    cp -r apps/cli/assets/Resources/* "${APP_DIR}/Contents/Resources/"
}

APP_DIR="apps/cli/build/Browsify.app"
rm -rf "${APP_DIR}"
mkdir -p "${APP_DIR}/Contents/MacOS"

cargo build --locked --release -p browsify-cli
cp target/release/browsify "${APP_DIR}/Contents/MacOS/browsify"
copy_assets "${APP_DIR}"

echo "Build complete ✨ (${APP_DIR})"
