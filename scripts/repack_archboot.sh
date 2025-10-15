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
ROOTFS_DIR="$WORK_DIR/rootfs"
ISO_OUT_DIR="$BUILD_ROOT/iso"
DOWNLOAD_DIR="$BUILD_ROOT/_downloads"
INSTALLER_REL="usr/local/bin/arm-installer"
DESKTOP_REL="usr/share/applications/arm-installer.desktop"
AUTOSTART_REL="etc/profile.d/arm-installer.sh"
TARGET_DIR="$PROJECT_ROOT/target"

for tool in rsync xorriso unsquashfs mksquashfs curl; do
    if ! command -v "$tool" >/dev/null 2>&1; then
        echo "Required tool '$tool' not found." >&2
        exit 1
    fi
done

mkdir -p "$DOWNLOAD_DIR" "$ISO_OUT_DIR" "$WORK_DIR"

if [[ "$ISO_SOURCE" =~ ^https?:// ]]; then
    ISO_PATH="$DOWNLOAD_DIR/$(basename "$ISO_SOURCE")"
    if [[ ! -f "$ISO_PATH" ]]; then
        echo "Downloading base ISO from $ISO_SOURCE"
        curl -L -o "$ISO_PATH" "$ISO_SOURCE"
    fi
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

if [[ $EUID -ne 0 ]]; then
    SUDO=sudo
else
    SUDO=""
fi

rm -rf "$TREE_DIR" "$ROOTFS_DIR"
mkdir -p "$MOUNT_DIR" "$TREE_DIR"

$SUDO mount -o loop "$ISO_PATH" "$MOUNT_DIR"
rsync -a "$MOUNT_DIR"/ "$TREE_DIR"/
$SUDO umount "$MOUNT_DIR"

ROOTFS_IMAGE=$(find "$TREE_DIR" -name '*.sfs' -print | head -n1)
if [[ -z "$ROOTFS_IMAGE" ]]; then
    echo "Could not locate rootfs squashfs image inside ISO" >&2
    exit 1
fi

echo "Unpacking root filesystem $ROOTFS_IMAGE"
rm -rf "$ROOTFS_DIR"
unsquashfs -f -d "$ROOTFS_DIR" "$ROOTFS_IMAGE"

ARCH=$(uname -m)
if [[ "$ARCH" == "aarch64" || "$ARCH" == "arm64" ]]; then
    cargo build --release --bin installer --manifest-path "$PROJECT_ROOT/installer/Cargo.toml" --target-dir "$TARGET_DIR"
    INSTALLER_BIN="$TARGET_DIR/release/installer"
else
    cargo build --release --target aarch64-unknown-linux-gnu --bin installer --manifest-path "$PROJECT_ROOT/installer/Cargo.toml" --target-dir "$TARGET_DIR"
    INSTALLER_BIN="$TARGET_DIR/aarch64-unknown-linux-gnu/release/installer"
fi

if [[ ! -f "$INSTALLER_BIN" ]]; then
    echo "Installer binary not found at $INSTALLER_BIN" >&2
    exit 1
fi

install -Dm755 "$INSTALLER_BIN" "$ROOTFS_DIR/$INSTALLER_REL"

mkdir -p "$ROOTFS_DIR/$(dirname "$DESKTOP_REL")"
cat > "$ROOTFS_DIR/$DESKTOP_REL" <<'DESKTOP'
[Desktop Entry]
Type=Application
Name=Arm Distro Installer
Exec=arm-installer
Terminal=true
Categories=System;
DESKTOP

mkdir -p "$ROOTFS_DIR/$(dirname "$AUTOSTART_REL")"
cat > "$ROOTFS_DIR/$AUTOSTART_REL" <<'AUTOSTART'
#!/bin/sh
if [ "$(tty)" = "/dev/tty1" ]; then
    /usr/local/bin/arm-installer
fi
AUTOSTART
chmod 755 "$ROOTFS_DIR/$AUTOSTART_REL"

echo "Repacking root filesystem"
rm -f "$ROOTFS_IMAGE"
mksquashfs "$ROOTFS_DIR" "$ROOTFS_IMAGE" -comp xz -noappend >/dev/null

OUT_ISO="$ISO_OUT_DIR/$OUTPUT_NAME"
mkdir -p "$ISO_OUT_DIR"

VOLUME_ID=$(xorriso -indev "$ISO_PATH" -pvd_info 2>/dev/null | awk -F '=' '/Volume id/ {print $2}' | sed 's/^ *//')
if [[ -z "$VOLUME_ID" ]]; then
    VOLUME_ID="ARM_DISTRO"
fi

echo "Creating repacked ISO at $OUT_ISO"

xorriso -as mkisofs \
    -iso-level 3 -full-iso9660-filenames -volid "$VOLUME_ID" \
    -eltorito-alt-boot --efi-boot EFI/BOOT/BOOTAA64.EFI -no-emul-boot \
    -output "$OUT_ISO" "$TREE_DIR" >/dev/null 2>&1

echo "Repacked ISO written to $OUT_ISO"
