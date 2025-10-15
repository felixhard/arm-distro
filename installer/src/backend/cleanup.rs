use anyhow::Result;

pub fn teardown_mounts(_root: &str) -> Result<()> {
    // TODO: Unmount filesystems and disable swap.
    Ok(())
}

pub fn sync_and_reboot() -> Result<()> {
    // TODO: Call sync and schedule reboot.
    Ok(())
}
