use anyhow::Result;

use crate::backend::command::CommandSpec;
use crate::backend::partition;
use crate::state::InstallerState;

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

    let (partition_summary, partition_commands) = if let Some(plan) = &state.target {
        let plan_commands = partition::build_command_plan(plan)?.commands;
        (
            format!("Apply partition layout to {}", plan.target.path),
            plan_commands,
        )
    } else if let Some(disk) = &state.selected_disk {
        (
            format!("Partition disk {} using default layout", disk.path),
            Vec::new(),
        )
    } else {
        (
            "No target disk selected; partitioning skipped".to_string(),
            Vec::new(),
        )
    };

    steps.push(InstallStep::new(
        InstallStage::PartitionDisks,
        partition_summary,
        partition_commands,
    ));

    steps.push(InstallStep::new(
        InstallStage::FormatFilesystems,
        "Format selected partitions",
        Vec::new(),
    ));

    steps.push(InstallStep::new(
        InstallStage::MountTarget,
        "Mount target root and boot partitions",
        Vec::new(),
    ));

    steps.push(InstallStep::new(
        InstallStage::InstallBaseSystem,
        "Install minimal Arch base system",
        Vec::new(),
    ));

    steps.push(InstallStep::new(
        InstallStage::InstallDesktopEnvironment,
        "Install GNOME desktop packages",
        Vec::new(),
    ));

    steps.push(InstallStep::new(
        InstallStage::ConfigureSystem,
        "Configure locale, users, networking, and services",
        Vec::new(),
    ));

    steps.push(InstallStep::new(
        InstallStage::InstallBootloader,
        "Install and configure bootloader",
        Vec::new(),
    ));

    steps.push(InstallStep::new(
        InstallStage::Finalize,
        "Finalize installation and clean up mounts",
        Vec::new(),
    ));

    Ok(InstallPlan::new(steps))
}
