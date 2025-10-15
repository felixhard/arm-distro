use anyhow::{anyhow, bail, Context, Result};

use crate::backend::command::CommandSpec;
use crate::state::{
    DiskIdentifier, DiskMode, DiskPlan, FileSystem, PartitionFlag, PartitionSize, PartitionSpec,
};

const MIB: u64 = 1024 * 1024;

pub fn validate_plan(disk: &DiskIdentifier, plan: &DiskPlan) -> Result<()> {
    if plan.target.path != disk.path {
        bail!("disk mismatch between selection and plan");
    }

    if plan.partitions.is_empty() {
        bail!("partition plan must contain at least one partition");
    }

    Ok(())
}

#[derive(Debug, Default)]
pub struct PartitionCommandPlan {
    pub commands: Vec<CommandSpec>,
}

pub fn default_plan_for_disk(disk: &DiskIdentifier) -> DiskPlan {
    let boot_partition = PartitionSpec {
        id: "esp".into(),
        mountpoint: Some("/boot/efi".into()),
        filesystem: FileSystem::Fat32,
        size: PartitionSize::ExactBytes(512 * 1024 * 1024),
        flags: vec![PartitionFlag::Esp, PartitionFlag::Boot],
    };

    let root_partition = PartitionSpec {
        id: "root".into(),
        mountpoint: Some("/".into()),
        filesystem: FileSystem::Ext4,
        size: PartitionSize::Remainder,
        flags: Vec::new(),
    };

    DiskPlan {
        target: disk.clone(),
        mode: DiskMode::UseEntireDisk,
        partitions: vec![boot_partition, root_partition],
    }
}

pub fn build_command_plan(plan: &DiskPlan) -> Result<PartitionCommandPlan> {
    let mut commands = Vec::new();
    let disk_path = plan.target.path.clone();

    // Create a GPT label by default; later this may vary per target requirements.
    commands.push(CommandSpec::new(
        "parted",
        vec![
            disk_path.clone(),
            "--script".into(),
            "mklabel".into(),
            "gpt".into(),
        ],
    ));

    let ranges = compute_ranges(plan)?;

    for (index, (spec, range)) in plan.partitions.iter().zip(ranges.iter()).enumerate() {
        let mut args = vec![
            disk_path.clone(),
            "--script".into(),
            "mkpart".into(),
            spec.id.clone(),
        ];

        if let Some(hint) = filesystem_hint(&spec.filesystem) {
            args.push(hint.into());
        }

        args.push(format!("{}MiB", range.start_mib));
        args.push(format!("{}MiB", range.end_mib));

        commands.push(CommandSpec::new("parted", args));

        for flag in &spec.flags {
            if let Some(flag_name) = flag_name(flag) {
                commands.push(CommandSpec::new(
                    "parted",
                    vec![
                        disk_path.clone(),
                        "--script".into(),
                        "set".into(),
                        (index + 1).to_string(),
                        flag_name.into(),
                        "on".into(),
                    ],
                ));
            }
        }
    }

    Ok(PartitionCommandPlan { commands })
}

pub fn apply_plan(plan: &DiskPlan, run_cmd: &dyn Fn(&CommandSpec) -> Result<()>) -> Result<()> {
    let command_plan = build_command_plan(plan)?;
    for command in command_plan.commands {
        run_cmd(&command)?;
    }
    Ok(())
}

pub fn describe_partition(spec: &PartitionSpec) -> String {
    let mountpoint = spec
        .mountpoint
        .clone()
        .unwrap_or_else(|| "unassigned".into());
    format!("{}: {} ({})", spec.id, mountpoint, spec.filesystem.label())
}

fn filesystem_hint(fs: &FileSystem) -> Option<&'static str> {
    match fs {
        FileSystem::Ext4 => Some("ext4"),
        FileSystem::Btrfs => Some("btrfs"),
        FileSystem::Xfs => Some("xfs"),
        FileSystem::Swap => None,
        FileSystem::Fat32 => Some("fat32"),
        FileSystem::Other(_) => None,
    }
}

fn flag_name(flag: &PartitionFlag) -> Option<&str> {
    match flag {
        PartitionFlag::Boot => Some("boot"),
        PartitionFlag::Esp => Some("esp"),
        PartitionFlag::Swap => Some("swap"),
        PartitionFlag::Lvm => Some("lvm"),
        PartitionFlag::Custom(name) => Some(name.as_str()),
    }
}

#[derive(Debug, Clone)]
struct PartitionRange {
    start_mib: u64,
    end_mib: u64,
}

fn compute_ranges(plan: &DiskPlan) -> Result<Vec<PartitionRange>> {
    let total_mib = plan.target.size_bytes / MIB;
    if total_mib == 0 {
        bail!("target disk size too small to compute partition ranges");
    }

    let mut ranges: Vec<Option<PartitionRange>> = vec![None; plan.partitions.len()];
    let mut cursor_mib = 1u64; // leave room for alignment
    let mut remainder_index = None;

    for (index, spec) in plan.partitions.iter().enumerate() {
        match spec.size {
            PartitionSize::Remainder => {
                if remainder_index.is_some() {
                    bail!("only one remainder partition is supported");
                }
                remainder_index = Some(index);
            }
            _ => {
                let span_mib = partition_size_to_mib(&spec.size, total_mib)
                    .with_context(|| format!("failed to compute size for partition {}", spec.id))?;
                let end_mib = cursor_mib.saturating_add(span_mib);
                ranges[index] = Some(PartitionRange {
                    start_mib: cursor_mib,
                    end_mib,
                });
                cursor_mib = end_mib;
            }
        }
    }

    if let Some(idx) = remainder_index {
        if cursor_mib >= total_mib {
            bail!("no remaining space for remainder partition");
        }
        ranges[idx] = Some(PartitionRange {
            start_mib: cursor_mib,
            end_mib: total_mib,
        });
        cursor_mib = total_mib;
    }

    if plan.mode == DiskMode::UseEntireDisk && cursor_mib < total_mib {
        // keep the remainder recognised but avoid creating an extra partition automatically
    }

    let mut final_ranges = Vec::with_capacity(plan.partitions.len());
    for (spec, range) in plan.partitions.iter().zip(ranges.into_iter()) {
        let range = range.ok_or_else(|| anyhow!("missing range for partition {}", spec.id))?;
        if range.end_mib <= range.start_mib {
            bail!("invalid range computed for partition {}", spec.id);
        }
        final_ranges.push(range);
    }

    Ok(final_ranges)
}

fn partition_size_to_mib(size: &PartitionSize, total_mib: u64) -> Result<u64> {
    let span_mib = match size {
        PartitionSize::ExactBytes(bytes) => {
            if *bytes == 0 {
                bail!("partition size must be > 0 bytes");
            }
            bytes.div_ceil(MIB)
        }
        PartitionSize::Percentage(percent) => {
            if *percent == 0 || *percent > 100 {
                bail!("invalid percentage size: {}", percent);
            }
            ((total_mib * (*percent as u64)).saturating_div(100)).max(1)
        }
        PartitionSize::Remainder => 0,
    };

    if span_mib == 0 {
        bail!("computed partition size is zero");
    }

    Ok(span_mib)
}
