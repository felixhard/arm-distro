# Project Plan: Minimal Arch-ARM + GNOME with Rust + Slint Installer

## Project Description

We want to build a **very minimal Arch Linux distribution** (aarch64 / ARM) with the **GNOME desktop environment**, capable of running on both **virtual machines** and **bare-metal Apple Silicon** (e.g. M4 Pro MacBook Pro).  

We will use **Rust** for all backend logic (filesystem operations, partitioning, shell execution) and **Slint** (a declarative GUI toolkit) for the installer frontend (UI).  

The installer must allow the user to choose how much of a disk or partition to allocate to this distribution. In the installer flow, **system language / locale** and **keyboard layout(s)** must be separate steps (users may want, for example, system messages in English but Swedish keyboard mapping).

We may leverage existing aarch64 / Archboot resources as a base, but we want to wrap them in a nicer installer and control the ISO / live environment.

---

## Goals & Requirements

- Bootable live ISO / USB environment for ARM64 which includes our installer  
- Installer UI that is user-friendly, wizard-style, with back / cancel navigation  
- Installer workflow with steps: locale, keyboard layout, timezone, disk partitioning, filesystem, install, configuration, bootloader, user setup  
- User can choose to use whole disk or custom partition layout; choose how much space to allocate  
- Installer logic (Rust) must perform partitioning, formatting, mounting, copying packages, configuring system, installing bootloader  
- The end system is minimal, with GNOME (gdm, gnome-shell, control center) and base utilities  
- Support Apple Silicon’s boot environment (UEFI, firmware, device tree)  
- Error handling, rollback or safe abort, and logging  
- Automated ISO build / packaging  
- Cross-compilation support for ARM64 target  

---

## Project Structure & Modules

project-root/
├── build/
│ ├── airootfs/ # live rootfs contents
│ └── iso/ # ISO tree, bootloader, kernel, etc
├── installer/
│ ├── ui/ # .slint files + UI glue
│ ├── backend/ # Rust modules for system logic
│ ├── app.rs # main entry & orchestration
│ └── Cargo.toml
├── configs/ # kernel, initramfs, bootloader templates
├── scripts/
│ └── build_iso.sh # builds the ISO image
└── docs/
└── project-plan.md # this document


### Installer / UI / Backend

- **UI (Slint)**  
  - Wizard interface: multiple screens / pages  
  - UI screens: welcome, locale, keyboard layout, timezone, partitioning, filesystem, install progress, system configuration, finish  
  - Forward / back / cancel buttons, summary / confirm screens, error dialogs  

- **Backend (Rust modules)**  
  - `disk` module: detect disks / block devices, existing partitions  
  - `partition` module: create, delete, resize partitions (wrapping `parted`, `sfdisk`, etc)  
  - `filesystem` module: mkfs, mkswap, mount, unmount, swapon / swapoff  
  - `installer` module: copying base system, package installation, generating fstab  
  - `configuration` module: write locale.conf, hostname, /etc/hosts, user creation, sudoers  
  - `bootloader` module: install UEFI / Apple Silicon boot stub, generate boot config  
  - `cleanup` module: unmount, disable swap, sync, reboot  

- **State & Flow Orchestration**  
  - A central `InstallerState` struct to carry all user choices  
  - UI triggers backend operations, passes progress & errors  
  - Logging & error reporting  

- **Cross-compilation & Packaging**  
  - Build the installer binary for aarch64  
  - Embed it into the live environment (inside `airootfs`)  
  - Build scripts to assemble ISO (bootloader, kernel, initramfs, rootfs)  

---

## Installer Flow (Wizard Steps)

1. **Welcome** — “Start installation” or abort  
2. **System Language / Locale** — pick locale for system messages / formats  
3. **Keyboard Layout(s)** — pick layout(s), variants, switching options  
4. **Timezone / Region** — select timezone  
5. **Disk / Target Selection**  
   - Option: use entire disk  
   - Option: manual partitioning (list disks, show partitions, permit create/delete/resize)  
   - Specify size or percentage, assign mount points (/, /boot, swap, /home)  
6. **Filesystem / Formatting**  
   - Choose filesystem types per partition (ext4, btrfs, etc)  
   - Format partitions, activate swap, mount them  
7. **Base System Installation**  
   - Install minimal base packages + GNOME + utilities  
   - Generate `/etc/fstab`  
8. **System Configuration (chroot stage)**  
   - Enable locale generation & set locale.conf  
   - Set hostname, write `/etc/hosts`  
   - Set root password; create user; enable sudo  
   - Configure keyboard, console, GUI mapping  
   - Enable networking service, gdm  
   - Install bootloader (UEFI / Apple stub) and generate config  
   - mkinitcpio / initramfs  
9. **Finish & Reboot**  
   - Unmount, disable swap, sync, reboot or exit  

Between steps: **Back / Cancel** navigation, **summary / confirmation** before destructive actions, **error handling / retry**, logging of all actions.

---

## Milestones & Timeline

| Milestone | Deliverable |
|---|---|
| Research & prototyping | Proof of concept: a Slint + Rust “hello installer” app; test Archboot on Apple Silicon |
| Live ISO skeleton | Minimal live environment boots, contains installer binary |
| UI + backend core | Implement core flow up to partitioning & filesystem steps |
| System install + bootloader | Integrate base system installation & bootloader logic |
| ISO automation | Script building ISO, packaging, cross-compile, embed installer |
| Testing & edge cases | Install on VM & hardware, test resizing, encryption, failure modes |
| Documentation & release | Final ISO, user docs, maintenance guide |

---

## Notes & Considerations

- **Apple Silicon boot support**: must include proper UEFI stub, kernel/dtb support, and detection logic  
- **Cross-compilation**: build Rust + Slint for aarch64; ensure dependencies are satisfied  
- **Binary size / dependencies**: keep installer minimal; Slint is lightweight  
- **Safety & rollback**: before formatting, show final summary and request explicit user confirmation  
- **Logging**: record logs (e.g. `/var/log/installer.log`) for debugging  
- **Extensibility**: later support for LUKS encryption, LVM, Btrfs subvolumes  
- **Error paths & abort cleanup**: on any failure, unmount partitions, disable swap, revert partial changes if possible  

---

If you like, I can convert this into a **Codex prompt** (i.e. “You are Codex, implement this plan in Rust + Slint”) and we can start generating code. Do you want me to produce that prompt next?
::contentReference[oaicite:0]{index=0}