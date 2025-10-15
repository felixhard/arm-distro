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

use anyhow::{Context, Result};
use parking_lot::RwLock;
use tracing::{error, info};

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

    pub fn execute_plan_stream<F>(&self, plan: InstallPlan, mut on_log: F) -> Result<bool>
    where
        F: FnMut(String),
    {
        let mut success = true;

        for step in plan.steps() {
            on_log(format!("== {:?} ==\n{}", step.stage, step.summary));

            for command in &step.commands {
                on_log(format!("$ {} {}", command.program, command.args.join(" ")));

                match self
                    .executor
                    .run(command)
                    .with_context(|| format!("failed to run {}", command.program))
                {
                    Ok(output) => {
                        for line in output.stdout.lines().filter(|l| !l.trim().is_empty()) {
                            on_log(line.to_string());
                        }
                        for line in output.stderr.lines().filter(|l| !l.trim().is_empty()) {
                            on_log(format!("stderr: {line}"));
                        }

                        if !output.success() {
                            success = false;
                            on_log(format!(
                                "command exited with status {:?}",
                                output.status.code()
                            ));
                            break;
                        }
                    }
                    Err(err) => {
                        success = false;
                        error!("command failed: {:#}", err);
                        on_log(format!("error: {err:#}"));
                        break;
                    }
                }
            }

            if !success {
                break;
            }
        }

        Ok(success)
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
