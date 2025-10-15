use anyhow::Result;

use crate::backend::command::CommandSpec;
use crate::backend::{filesystem, packages, partition};
use crate::state::{FileSystem, InstallerState, PartitionSpec};

const TARGET_ROOT: &str = "/mnt/arm-distro";

#[derive(Debug, Clone)]
pub struct InstallPlan {
    steps: Vec<InstallStep>,
}

impl InstallPlan {
    pub fn new(steps: Vec<InstallStep>) -> Self {
        Self { steps }
    }

    pub fn steps(&self) -> &[InstallStep] {
        &self.steps
    }

    pub fn is_empty(&self) -> bool {
        self.steps.is_empty()
    }
}

#[derive(Debug, Clone)]
pub struct InstallStep {
    pub stage: InstallStage,
    pub summary: String,
    pub commands: Vec<CommandSpec>,
}

impl InstallStep {
    pub fn new(stage: InstallStage, summary: impl Into<String>, commands: Vec<CommandSpec>) -> Self {
        Self {
            stage,
            summary: summary.into(),
            commands,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InstallStage {
    PrepareEnvironment,
    PartitionDisks,
    FormatFilesystems,
    MountTarget,
    InstallBaseSystem,
    InstallDesktopEnvironment,
    ConfigureSystem,
    InstallBootloader,
    Finalize,
}

pub fn build_plan(state: &InstallerState) -> Result<InstallPlan> {
    let mut steps = Vec::new();

    steps.push(InstallStep::new(
        InstallStage::PrepareEnvironment,
        "Prepare live environment and validate selections",
        Vec::new(),
    ));

    let (partition_summary, partition_commands, plan_opt) = if let Some(plan) = &state.target {
        let plan_commands = partition::build_command_plan(plan)?.commands;
        (
            format!("Apply partition layout to {}", plan.target.path),
            plan_commands,
            Some(plan.clone()),
        )
    } else if let Some(disk) = &state.selected_disk {
        let default_plan = partition::default_plan_for_disk(disk);
        let plan_commands = partition::build_command_plan(&default_plan)?.commands;
        (
            format!("Partition disk {} using default layout", disk.path),
            plan_commands,
            Some(default_plan),
        )
    } else {
        (
            "No target disk selected; partitioning skipped".to_string(),
            Vec::new(),
            None,
        )
    };

    steps.push(InstallStep::new(
        InstallStage::PartitionDisks,
        partition_summary,
        partition_commands.clone(),
    ));

    let mut format_commands = Vec::new();
    let mut mount_commands = Vec::new();
    let mut mount_points: Vec<String> = Vec::new();
    let mut swap_devices: Vec<String> = Vec::new();

    if let Some(plan) = plan_opt {
        for (index, spec) in plan.partitions.iter().enumerate() {
            let device = partition_device_path(&plan.target.path, index + 1);

            if let Some(mkfs) = filesystem::mkfs_command(&device, spec)? {
                format_commands.push(mkfs);
            }

            if spec.filesystem == FileSystem::Swap {
                format_commands.push(filesystem::activate_swap_command(&device));
                swap_devices.push(device);
                continue;
            }

            if let Some(command) = filesystem::mount_command(spec, &device, TARGET_ROOT)? {
                if let Some(target_path) = mount_target_path(spec, TARGET_ROOT) {
                    mount_points.push(target_path.clone());
                    mount_commands.push(mkdir_p_command(target_path));
                }
                mount_commands.push(command);
            }
        }
    }

    steps.push(InstallStep::new(
        InstallStage::FormatFilesystems,
        "Format selected partitions",
        format_commands,
    ));

    steps.push(InstallStep::new(
        InstallStage::MountTarget,
        format!("Mount target partitions under {}", TARGET_ROOT),
        mount_commands,
    ));

    steps.push(InstallStep::new(
        InstallStage::InstallBaseSystem,
        "Install minimal Arch base system",
        vec![packages::install_base_packages(TARGET_ROOT)],
    ));

    steps.push(InstallStep::new(
        InstallStage::InstallDesktopEnvironment,
        "Install GNOME desktop packages",
        vec![packages::install_desktop_packages(TARGET_ROOT)],
    ));

    steps.push(InstallStep::new(
        InstallStage::ConfigureSystem,
        "Configure locale, users, networking, and services",
        build_config_commands(),
    ));

    steps.push(InstallStep::new(
        InstallStage::InstallBootloader,
        "Install and configure bootloader",
        build_bootloader_commands(),
    ));

    steps.push(InstallStep::new(
        InstallStage::Finalize,
        "Finalize installation and clean up mounts",
        build_finalize_commands(&mount_points, &swap_devices),
    ));

    Ok(InstallPlan::new(steps))
}

fn partition_device_path(disk_path: &str, index: usize) -> String {
    if disk_path.contains("nvme") || disk_path.contains("mmcblk") {
        format!("{}p{}", disk_path, index)
    } else {
        format!("{}{}", disk_path, index)
    }
}

fn mkdir_p_command(path: String) -> CommandSpec {
    CommandSpec::new(
        "mkdir",
        vec!["-p".into(), path],
    )
}

fn mount_target_path(spec: &PartitionSpec, root: &str) -> Option<String> {
    let mountpoint = spec.mountpoint.as_ref()?;
    let root = root.trim_end_matches('/');
    let mount = mountpoint.trim_start_matches('/');
    if mount.is_empty() {
        Some(root.to_string())
    } else {
        Some(format!("{}/{}", root, mount))
    }
}

fn build_finalize_commands(mount_points: &[String], swap_devices: &[String]) -> Vec<CommandSpec> {
    let mut commands = Vec::new();

    commands.push(CommandSpec::new("sync", Vec::<String>::new()));

    for device in swap_devices.iter() {
        commands.push(filesystem::deactivate_swap_command(device));
    }

    let mut seen_root = false;

    for path in mount_points.iter().rev() {
        if path == TARGET_ROOT {
            seen_root = true;
        }
        commands.push(CommandSpec::new("umount", vec![path.clone()]));
    }

    if !seen_root {
        commands.push(CommandSpec::new("umount", vec![TARGET_ROOT.into()]));
    }

    commands
}

fn build_config_commands() -> Vec<CommandSpec> {
    let mut commands = Vec::new();

    commands.push(CommandSpec::new(
        "arch-chroot",
        vec![
            TARGET_ROOT.into(),
            "sh".into(),
            "-c".into(),
            "locale-gen".into(),
        ],
    ));

    commands.push(CommandSpec::new(
        "arch-chroot",
        vec![
            TARGET_ROOT.into(),
            "sh".into(),
            "-c".into(),
            "echo 'LANG=en_US.UTF-8' > /etc/locale.conf".into(),
        ],
    ));

    commands.push(CommandSpec::new(
        "arch-chroot",
        vec![
            TARGET_ROOT.into(),
            "ln".into(),
            "-sf".into(),
            "/usr/share/zoneinfo/UTC".into(),
            "/etc/localtime".into(),
        ],
    ));

    commands.push(CommandSpec::new(
        "arch-chroot",
        vec![
            TARGET_ROOT.into(),
            "sh".into(),
            "-c".into(),
            "hwclock --systohc".into(),
        ],
    ));

    commands.push(CommandSpec::new(
        "arch-chroot",
        vec![
            TARGET_ROOT.into(),
            "useradd".into(),
            "-m".into(),
            "-G".into(),
            "wheel".into(),
            "armuser".into(),
        ],
    ));

    commands.push(CommandSpec::new(
        "arch-chroot",
        vec![
            TARGET_ROOT.into(),
            "sh".into(),
            "-c".into(),
            "echo 'armuser:armdistro' | chpasswd".into(),
        ],
    ));

    commands.push(CommandSpec::new(
        "arch-chroot",
        vec![
            TARGET_ROOT.into(),
            "sh".into(),
            "-c".into(),
            "echo '%wheel ALL=(ALL) NOPASSWD: ALL' > /etc/sudoers.d/99-arm-distro".into(),
        ],
    ));

    commands.extend(packages::enable_services_commands(TARGET_ROOT));

    commands
}

fn build_bootloader_commands() -> Vec<CommandSpec> {
    let mut commands = Vec::new();

    commands.push(CommandSpec::new(
        "arch-chroot",
        vec![
            TARGET_ROOT.into(),
            "sh".into(),
            "-c".into(),
            "bootctl install".into(),
        ],
    ));

    commands.push(CommandSpec::new(
        "arch-chroot",
        vec![
            TARGET_ROOT.into(),
            "sh".into(),
            "-c".into(),
            "echo 'default arch.conf' > /boot/loader/loader.conf".into(),
        ],
    ));

    commands.push(CommandSpec::new(
        "arch-chroot",
        vec![
            TARGET_ROOT.into(),
            "sh".into(),
            "-c".into(),
            "mkinitcpio -P".into(),
        ],
    ));

    commands
}
