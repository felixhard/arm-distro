use anyhow::{bail, Result};

use crate::backend::command::CommandSpec;
use crate::state::{FileSystem, PartitionSpec};

pub fn mkfs_command(device_path: &str, spec: &PartitionSpec) -> Result<Option<CommandSpec>> {
    match &spec.filesystem {
        FileSystem::Ext4 => Ok(Some(CommandSpec::new(
            "mkfs.ext4",
            vec!["-F".into(), device_path.into()],
        ))),
        FileSystem::Btrfs => Ok(Some(CommandSpec::new(
            "mkfs.btrfs",
            vec!["-f".into(), device_path.into()],
        ))),
        FileSystem::Xfs => Ok(Some(CommandSpec::new(
            "mkfs.xfs",
            vec!["-f".into(), device_path.into()],
        ))),
        FileSystem::Fat32 => Ok(Some(CommandSpec::new(
            "mkfs.fat",
            vec!["-F".into(), "32".into(), device_path.into()],
        ))),
        FileSystem::Swap => Ok(Some(CommandSpec::new("mkswap", vec![device_path.into()]))),
        FileSystem::Other(fs) => {
            if fs.is_empty() {
                bail!("custom filesystem identifier cannot be empty");
            }
            Ok(Some(CommandSpec::new(fs.clone(), vec![device_path.into()])))
        }
    }
}

pub fn mount_command(
    spec: &PartitionSpec,
    device_path: &str,
    root: &str,
) -> Result<Option<CommandSpec>> {
    let Some(mountpoint) = spec.mountpoint.as_ref() else {
        return Ok(None);
    };

    if mountpoint.is_empty() {
        bail!("mountpoint cannot be empty when provided");
    }

    let full_target = format!(
        "{}/{}",
        root.trim_end_matches('/'),
        mountpoint.trim_start_matches('/')
    );

    let command = match spec.filesystem {
        FileSystem::Swap => None,
        _ => Some(CommandSpec::new(
            "mount",
            vec![device_path.into(), full_target],
        )),
    };

    Ok(command)
}

pub fn activate_swap_command(device_path: &str) -> CommandSpec {
    CommandSpec::new("swapon", vec![device_path.into()])
}

pub fn deactivate_swap_command(device_path: &str) -> CommandSpec {
    CommandSpec::new("swapoff", vec![device_path.into()])
}
