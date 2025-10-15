pub mod boot;
pub mod cleanup;
pub mod command;
pub mod config;
pub mod disk;
pub mod filesystem;
pub mod packages;
pub mod partition;
pub mod tasks;

use std::sync::Arc;

use anyhow::Result;
use parking_lot::RwLock;
use tracing::info;

use crate::state::{DiskIdentifier, InstallerState};
use command::{CommandExecutor, SystemCommandExecutor};
use tasks::{build_plan, InstallPlan};

pub struct Backend {
    state: Arc<RwLock<InstallerState>>,
    executor: Arc<dyn CommandExecutor + Send + Sync>,
}

impl Backend {
    pub fn new(state: Arc<RwLock<InstallerState>>) -> Self {
        Self {
            state,
            executor: Arc::new(SystemCommandExecutor::default()),
        }
    }

    pub fn with_executor(
        state: Arc<RwLock<InstallerState>>,
        executor: Arc<dyn CommandExecutor + Send + Sync>,
    ) -> Self {
        Self { state, executor }
    }

    pub fn list_disks(&self) -> Result<Vec<DiskIdentifier>> {
        disk::probe_block_devices_with(self.executor.as_ref())
    }

    pub fn begin_installation(&self) -> Result<InstallPlan> {
        let state_snapshot = self.state.read().clone();
        let plan = build_plan(&state_snapshot)?;

        for step in plan.steps() {
            info!(stage = ?step.stage, summary = step.summary, command_count = step.commands.len(), "scheduled install step");
        }

        Ok(plan)
    }
}

impl Default for Backend {
    fn default() -> Self {
        Self::new(Arc::new(RwLock::new(InstallerState::default())))
    }
}

impl Clone for Backend {
    fn clone(&self) -> Self {
        Self {
            state: Arc::clone(&self.state),
            executor: Arc::clone(&self.executor),
        }
    }
}
