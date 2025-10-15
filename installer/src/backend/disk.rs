use anyhow::{Context, Result};
use serde::Deserialize;

use crate::backend::command::{
    run_command_json, CommandExecutor, CommandSpec, SystemCommandExecutor,
};
use crate::state::DiskIdentifier;

pub fn probe_block_devices() -> Result<Vec<DiskIdentifier>> {
    let executor = SystemCommandExecutor::default();
    probe_block_devices_with(&executor)
}

pub fn probe_block_devices_with(executor: &dyn CommandExecutor) -> Result<Vec<DiskIdentifier>> {
    let spec = CommandSpec::new(
        "lsblk",
        vec![
            "--json".to_string(),
            "--bytes".to_string(),
            "-d".to_string(),
            "-o".to_string(),
            "NAME,SIZE,MODEL,TYPE".to_string(),
        ],
    );
    let output: LsblkOutput =
        run_command_json(executor, &spec).context("failed to probe block devices with lsblk")?;

    let devices = output
        .blockdevices
        .into_iter()
        .filter(|device| device.r#type == "disk")
        .map(|device| DiskIdentifier {
            path: format!("/dev/{}", device.name),
            size_bytes: device.size,
            label: device.model.filter(|s| !s.trim().is_empty()),
        })
        .collect();

    Ok(devices)
}

#[derive(Debug, Deserialize)]
struct LsblkOutput {
    #[serde(default)]
    blockdevices: Vec<LsblkDevice>,
}

#[derive(Debug, Deserialize)]
struct LsblkDevice {
    name: String,
    size: u64,
    #[serde(default)]
    model: Option<String>,
    #[serde(rename = "type")]
    r#type: String,
}
