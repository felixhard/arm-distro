use anyhow::{anyhow, Result};

use crate::state::{InstallerState, NetworkConfig};

pub fn apply_locale(_root: &str, state: &InstallerState) -> Result<()> {
    if state.locale.language.is_empty() {
        return Err(anyhow!("locale language cannot be empty"));
    }
    if state.locale.region.is_empty() {
        return Err(anyhow!("locale region cannot be empty"));
    }
    Ok(())
}

pub fn configure_network(_root: &str, network: &NetworkConfig) -> Result<()> {
    if network.hostname.is_empty() {
        return Err(anyhow!("hostname cannot be empty"));
    }
    Ok(())
}
