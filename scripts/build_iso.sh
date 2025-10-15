#!/usr/bin/env bash
set -euo pipefail

PROJECT_ROOT=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
PROFILE_SRC="$PROJECT_ROOT/configs/archiso"
PROFILE_BUILD="$PROJECT_ROOT/build/archiso-profile"
WORKDIR="$PROJECT_ROOT/build/work"
OUTDIR="$PROJECT_ROOT/build/iso"
INSTALLER_BIN="$PROJECT_ROOT/target/aarch64-unknown-linux-gnu/release/installer"
INSTALL_DEST="airootfs/usr/local/bin/arm-installer"

if ! command -v mkarchiso >/dev/null 2>&1; then
    echo "mkarchiso not found. Please install archiso (pacman -S archiso)." >&2
    exit 1
fi

cargo build --release --target aarch64-unknown-linux-gnu --bin installer --manifest-path "$PROJECT_ROOT/installer/Cargo.toml"

rm -rf "$PROFILE_BUILD"
mkdir -p "$PROFILE_BUILD"
cp -a "$PROFILE_SRC"/. "$PROFILE_BUILD"

install -Dm0755 "$INSTALLER_BIN" "$PROFILE_BUILD/$INSTALL_DEST"

rm -rf "$WORKDIR"
mkdir -p "$OUTDIR"

mkarchiso -v -w "$WORKDIR" -o "$OUTDIR" "$PROFILE_BUILD"
