use anyhow::Result;

pub fn install_bootloader(_root: &str) -> Result<()> {
    // TODO: Implement UEFI + Apple Silicon boot logic.
    Ok(())
}

pub fn generate_boot_config(_root: &str) -> Result<()> {
    // TODO: Generate loader entries and kernel parameters.
    Ok(())
}
