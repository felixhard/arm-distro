#!/usr/bin/env bash
set -euo pipefail

if [[ $# -ne 1 ]]; then
    echo "Usage: $0 /path/to/archboot-aarch64.iso" >&2
    exit 1
fi

BASE_ISO=$(realpath "$1")
if [[ ! -f "$BASE_ISO" ]]; then
    echo "Base ISO not found: $BASE_ISO" >&2
    exit 1
fi

PROJECT_ROOT=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
BUILD_ROOT="$PROJECT_ROOT/build"
WORK_DIR="$BUILD_ROOT/archboot-overlay"
ISO_OUT_DIR="$BUILD_ROOT/iso"
OVERLAY_DIR="$WORK_DIR/overlay"
INSTALLER_REL="usr/local/bin/arm-installer"
DESKTOP_REL="usr/share/applications/arm-installer.desktop"
AUTOSTART_REL="etc/profile.d/arm-installer.sh"
OUTPUT_NAME="$(basename "$BASE_ISO" .iso)-arm-distro.iso"

mkdir -p "$WORK_DIR" "$ISO_OUT_DIR"
rm -rf "$OVERLAY_DIR"
mkdir -p "$OVERLAY_DIR"

for tool in xorriso; do
    if ! command -v "$tool" >/dev/null 2>&1; then
        echo "Required tool '$tool' not found. Install libisoburn." >&2
        exit 1
    fi
done

ARCH=$(uname -m)
if [[ "$ARCH" == "aarch64" || "$ARCH" == "arm64" ]]; then
    cargo build --release --bin installer --manifest-path "$PROJECT_ROOT/installer/Cargo.toml"
    INSTALLER_BIN="$PROJECT_ROOT/target/release/installer"
else
    cargo build --release --target aarch64-unknown-linux-gnu --bin installer --manifest-path "$PROJECT_ROOT/installer/Cargo.toml"
    INSTALLER_BIN="$PROJECT_ROOT/target/aarch64-unknown-linux-gnu/release/installer"
fi

install -Dm755 "$INSTALLER_BIN" "$OVERLAY_DIR/$INSTALLER_REL"

mkdir -p "$OVERLAY_DIR/$(dirname "$DESKTOP_REL")"
cat > "$OVERLAY_DIR/$DESKTOP_REL" <<'DESKTOP'
[Desktop Entry]
Type=Application
Name=Arm Distro Installer
Exec=arm-installer
Terminal=true
Categories=System;
DESKTOP

mkdir -p "$OVERLAY_DIR/$(dirname "$AUTOSTART_REL")"
cat > "$OVERLAY_DIR/$AUTOSTART_REL" <<'AUTOSTART'
#!/bin/sh
TTY=$(tty)
if [ "$TTY" = "/dev/tty1" ]; then
    /usr/local/bin/arm-installer
fi
AUTOSTART
chmod 755 "$OVERLAY_DIR/$AUTOSTART_REL"

OUT_ISO="$ISO_OUT_DIR/$OUTPUT_NAME"

echo "Creating repacked ISO at $OUT_ISO"

xorriso -indev "$BASE_ISO" \
        -outdev "$OUT_ISO" \
        -map "$OVERLAY_DIR" / \
        -boot_image any replay > "$WORK_DIR/xorriso.log" 2>&1 || {
    echo "Failed to create ISO. See $WORK_DIR/xorriso.log" >&2
    exit 1
}

echo "Repacked ISO written to $OUT_ISO"
