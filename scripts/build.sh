#!/bin/bash
set -e

# Build browsify-ui
(
    cd apps/browsify-ui
    npm run build
)

copy_assets() {
    local APP_DIR=$1
    mkdir -p "${APP_DIR}/Contents/Resources/ui"
    cp -r apps/browsify-ui/dist/* "${APP_DIR}/Contents/Resources/ui/"
    cp apps/browsify/assets/Info.plist "${APP_DIR}/Contents/Info.plist"
    cp -r apps/browsify/assets/Resources/* "${APP_DIR}/Contents/Resources/"
}

build_arch() {
    local RUST_TARGET=$1
    local ARCH_LABEL=$2
    local APP_DIR="apps/browsify/build/Browsify-${ARCH_LABEL}.app"

    mkdir -p "${APP_DIR}/Contents/MacOS"
    mkdir -p "${APP_DIR}/Contents/Resources"

    cargo build --release --target "${RUST_TARGET}" -p browsify
    cp "target/${RUST_TARGET}/release/browsify" "${APP_DIR}/Contents/MacOS/browsify"
}

if [ "${BUILD_UNIVERSAL:-0}" = "1" ]; then
    build_arch aarch64-apple-darwin arm64
    build_arch x86_64-apple-darwin amd64

    APP_DIR="apps/browsify/build/Browsify.app"
    mkdir -p "${APP_DIR}/Contents/MacOS"
    mkdir -p "${APP_DIR}/Contents/Resources"

    lipo -create \
        apps/browsify/build/Browsify-arm64.app/Contents/MacOS/browsify \
        apps/browsify/build/Browsify-amd64.app/Contents/MacOS/browsify \
        -output "${APP_DIR}/Contents/MacOS/browsify"

    lipo -info "${APP_DIR}/Contents/MacOS/browsify"
    copy_assets "${APP_DIR}"
else
    APP_DIR="apps/browsify/build/Browsify.app"
    rm -rf "${APP_DIR}"
    mkdir -p "${APP_DIR}/Contents/MacOS"
    mkdir -p "${APP_DIR}/Contents/Resources"

    cargo build --release -p browsify
    cp target/release/browsify "${APP_DIR}/Contents/MacOS/browsify"
    copy_assets "${APP_DIR}"
fi

echo "Build complete ✨ (${APP_DIR})"
