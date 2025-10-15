# Development Environment Setup

This repository targets building a minimal Arch Linux ARM installer with a Rust + Slint frontend. The steps below prepare a macOS or Linux workstation for day-to-day development.

## 1. Toolchain Prerequisites

- **Rust toolchain**: Install via [`rustup`](https://rustup.rs) and add the `aarch64-unknown-linux-gnu` target.
  ```bash
  rustup toolchain install stable
  rustup target add aarch64-unknown-linux-gnu
  ```
- **Cargo helpers**: `cargo-binstall` (optional for faster install of dev tools), `cargo-zigbuild` or [`cross`](https://github.com/cross-rs/cross) for cross-linking if native aarch64 libs are unavailable.
- **Slint build deps**:
  - macOS: `brew install cmake ninja pkg-config` plus a recent clang.
  - Linux: `sudo pacman -S cmake ninja pkgconf clang` (adjust for your distro).
- **Node-free UI building**: Slint generates Rust code at build time, no extra runtime deps required.

## 2. Arch ARM Build Requirements

- `archiso` tooling for ISO creation (`pacman -S archiso` or build from source on other distros).
- `qemu-system-aarch64` with UEFI firmware (`edk2-aarch64-code.fd`) for VM testing.
- `parted`, `sfdisk`, `mkfs.*`, `dosfstools`, `mtools`, `e2fsprogs`, `btrfs-progs`, `xfsprogs` to support partitioning and filesystems.
- `curl`, `aria2`, or similar download helpers for fetching packages and kernels into the live rootfs.

## 3. Project Layout Overview

```
project-root/
├── build/
│   ├── airootfs/        # Live environment overlay
│   └── iso/             # Generated ISO artifacts
├── configs/             # Kernel, initramfs, bootloader templates
├── docs/                # Documentation (this directory)
├── installer/           # Rust + Slint installer application
│   ├── src/             # Rust sources
│   └── ui/              # Slint UI definitions
├── scripts/             # Automation (ISO build, testing)
└── project-plan.md
```

## 4. Helpful Commands

- `cargo check` (host) — ensure the installer compiles for development.
- `cargo build --target aarch64-unknown-linux-gnu` — produce an ARM binary.
- `scripts/build_iso.sh` — *(planned)* orchestrator for the live image.
- `qemu-system-aarch64 ...` — boot the resulting image under emulation.

## 5. Next Steps

1. Verify the toolchain by running `cargo check` inside `installer/` once Rust is installed.
2. Flesh out backend command abstractions (wrapping `lsblk`, `parted`, `mkfs`).
3. Expand the Slint UI into the full wizard and connect it to the backend controller.
4. Automate ISO assembly under `scripts/build_iso.sh` using `archiso`.

Keep this document updated as dependencies evolve.
