#!/usr/bin/env bash
set -euo pipefail

PROJECT_ROOT=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
PROFILE_SRC="$PROJECT_ROOT/configs/archiso"
PROFILE_BUILD="$PROJECT_ROOT/build/archiso-profile"
WORKDIR="$PROJECT_ROOT/build/work"
OUTDIR="$PROJECT_ROOT/build/iso"
DEPS_DIR="$PROJECT_ROOT/build/_deps"
ARCHISO_REPO="https://gitlab.archlinux.org/archlinux/archiso.git"
ARCHISO_DIR="$DEPS_DIR/archiso"
MKARCHISO="$ARCHISO_DIR/mkarchiso"
INSTALLER_BIN="$PROJECT_ROOT/target/aarch64-unknown-linux-gnu/release/installer"
INSTALL_DEST="airootfs/usr/local/bin/arm-installer"

for tool in mksquashfs xorriso mkfs.fat; do
    if ! command -v "$tool" >/dev/null 2>&1; then
        echo "Required tool '$tool' not found. Install squashfs-tools, dosfstools, libisoburn." >&2
        exit 1
    fi
done

mkdir -p "$DEPS_DIR"
if [ ! -d "$ARCHISO_DIR" ]; then
    echo "Cloning archiso tooling..."
    git clone --depth 1 "$ARCHISO_REPO" "$ARCHISO_DIR"
fi

if [ ! -x "$MKARCHISO" ]; then
    echo "mkarchiso script not found at $MKARCHISO" >&2
    exit 1
fi

cargo build --release --target aarch64-unknown-linux-gnu --bin installer --manifest-path "$PROJECT_ROOT/installer/Cargo.toml"

rm -rf "$PROFILE_BUILD"
mkdir -p "$PROFILE_BUILD"
cp -a "$PROFILE_SRC"/. "$PROFILE_BUILD"

install -Dm0755 "$INSTALLER_BIN" "$PROFILE_BUILD/$INSTALL_DEST"

rm -rf "$WORKDIR"
mkdir -p "$OUTDIR"

bash "$MKARCHISO" -v -w "$WORKDIR" -o "$OUTDIR" "$PROFILE_BUILD"
