#!/usr/bin/env bash
set -euo pipefail

PROJECT_ROOT=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
DEFAULT_URL="https://release.archboot.eu/aarch64/latest/iso/archboot-2025.10.05-02.24-6.16.7-1-aarch64-ARCH-local-aarch64.iso"
ISO_SOURCE="${1:-$DEFAULT_URL}"
OUTPUT_NAME="${2:-}"  # optional second arg

BUILD_ROOT="$PROJECT_ROOT/build"
WORK_DIR="$BUILD_ROOT/archboot-repack"
MOUNT_DIR="$WORK_DIR/mount"
TREE_DIR="$WORK_DIR/tree"
ISO_OUT_DIR="$BUILD_ROOT/iso"
DOWNLOAD_DIR="$BUILD_ROOT/_downloads"
INSTALLER_REL="usr/local/bin/arm-installer"
DESKTOP_REL="usr/share/applications/arm-installer.desktop"
AUTOSTART_REL="etc/profile.d/arm-installer.sh"

mkdir -p "$DOWNLOAD_DIR" "$ISO_OUT_DIR" "$WORK_DIR"

if [[ "$ISO_SOURCE" =~ ^https?:// ]]; then
    echo "Downloading base ISO from $ISO_SOURCE"
    ISO_PATH="$DOWNLOAD_DIR/$(basename "$ISO_SOURCE")"
    curl -L -o "$ISO_PATH" "$ISO_SOURCE"
else
    ISO_PATH=$(realpath "$ISO_SOURCE")
fi

if [[ ! -f "$ISO_PATH" ]]; then
    echo "Base ISO not found: $ISO_PATH" >&2
    exit 1
fi

if [[ -z "$OUTPUT_NAME" ]]; then
    OUTPUT_NAME="$(basename "$ISO_PATH" .iso)-arm-distro.iso"
fi

for tool in rsync xorriso; do
    if ! command -v "$tool" >/dev/null 2>&1; then
        echo "Required tool '$tool' not found. Install rsync and libisoburn." >&2
        exit 1
    fi
done

if [[ $EUID -ne 0 ]]; then
    SUDO=sudo
else
    SUDO=""
fi

rm -rf "$MOUNT_DIR" "$TREE_DIR"
mkdir -p "$MOUNT_DIR" "$TREE_DIR"

$SUDO mount -o loop "$ISO_PATH" "$MOUNT_DIR"
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

mkdir -p "$TREE_DIR/$(dirname "$DESKTOP_REL")"
cat > "$TREE_DIR/$DESKTOP_REL" <<'DESKTOP'
[Desktop Entry]
Type=Application
Name=Arm Distro Installer
Exec=arm-installer
Terminal=true
Categories=System;
DESKTOP

mkdir -p "$TREE_DIR/$(dirname "$AUTOSTART_REL")"
cat > "$TREE_DIR/$AUTOSTART_REL" <<'AUTOSTART'
#!/bin/sh
if [ "$(tty)" = "/dev/tty1" ]; then
    /usr/local/bin/arm-installer
fi
AUTOSTART
chmod 755 "$TREE_DIR/$AUTOSTART_REL"

OUT_ISO="$ISO_OUT_DIR/$OUTPUT_NAME"

echo "Creating repacked ISO at $OUT_ISO"

xorriso -indev "$ISO_PATH" \
        -outdev "$OUT_ISO" \
        -map "$TREE_DIR" / \
        -boot_image any replay > "$WORK_DIR/xorriso.log" 2>&1 || {
    echo "Failed to create ISO. See $WORK_DIR/xorriso.log" >&2
    exit 1
}

echo "Repacked ISO written to $OUT_ISO"
