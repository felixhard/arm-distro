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
WORK_DIR="$BUILD_ROOT/archboot-repack"
ISO_OUT_DIR="$BUILD_ROOT/iso"
MOUNT_DIR="$WORK_DIR/mount"
TREE_DIR="$WORK_DIR/tree"
INSTALLER_REL="usr/local/bin/arm-installer"
DESKTOP_REL="usr/share/applications/arm-installer.desktop"
AUTOSTART_REL="etc/profile.d/arm-installer.sh"

mkdir -p "$WORK_DIR" "$ISO_OUT_DIR"
rm -rf "$TREE_DIR"
mkdir -p "$MOUNT_DIR" "$TREE_DIR"

if [[ $EUID -ne 0 ]]; then
    SUDO=sudo
else
    SUDO=""
fi

$SUDO mount -o loop "$BASE_ISO" "$MOUNT_DIR"
rsync -a "$MOUNT_DIR"/ "$TREE_DIR"/
$SUDO umount "$MOUNT_DIR"

ARCH=$(uname -m)
if [[ "$ARCH" == "aarch64" || "$ARCH" == "arm64" ]]; then
    cargo build --release --bin installer --manifest-path "$PROJECT_ROOT/installer/Cargo.toml"
    INSTALLER_BIN="$PROJECT_ROOT/target/release/installer"
else
    cargo build --release --target aarch64-unknown-linux-gnu --bin installer --manifest-path "$PROJECT_ROOT/installer/Cargo.toml"
    INSTALLER_BIN="$PROJECT_ROOT/target/aarch64-unknown-linux-gnu/release/installer"
fi

install -Dm755 "$INSTALLER_BIN" "$TREE_DIR/$INSTALLER_REL"

cat > "$TREE_DIR/$DESKTOP_REL" <<'DESKTOP'
[Desktop Entry]
Type=Application
Name=Arm Distro Installer
Exec=arm-installer
Terminal=true
Categories=System;
DESKTOP

cat > "$TREE_DIR/$AUTOSTART_REL" <<'AUTOSTART'
#!/bin/sh
if [ "$DISPLAY" = "" ]; then
    TTY=$(tty)
    if [ "$TTY" = "/dev/tty1" ]; then
        /usr/local/bin/arm-installer
    fi
fi
AUTOSTART
chmod 755 "$TREE_DIR/$AUTOSTART_REL"

OUTPUT_NAME="$(basename "$BASE_ISO" .iso)-arm-distro"
cd "$TREE_DIR"
$SUDO ./scripts/mkimage.sh iso "$OUTPUT_NAME"

ISO_SOURCE=$(find out -name '*.iso' -print -quit)
if [[ -z "$ISO_SOURCE" ]]; then
    echo "Failed to locate rebuilt ISO" >&2
    exit 1
fi
mv "$ISO_SOURCE" "$ISO_OUT_DIR/${OUTPUT_NAME}.iso"

echo "Repacked ISO written to $ISO_OUT_DIR/${OUTPUT_NAME}.iso"
