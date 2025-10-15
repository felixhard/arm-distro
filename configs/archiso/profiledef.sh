#!/usr/bin/env bash

set -euo pipefail

iso_name="arm-distro"
iso_label="ARM_DISTRO"
iso_publisher="Arm Distro"
iso_application="Arm Distro Minimal Installer"
install_dir="arch"
buildmodes=(iso)
bootmodes=('uefi-x86_64.systemd-boot')
arch="aarch64"
packagelist=('base' 'linux-zen' 'linux-zen-firmware' 'arch-install-scripts' 'networkmanager' 'openssh' 'gnome-shell' 'gdm')
hooks=()
quiet="n"
