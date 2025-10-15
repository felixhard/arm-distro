use std::process::{Command, ExitStatus, Stdio};

use anyhow::{Context, Result};
use serde::de::DeserializeOwned;

#[derive(Debug, Clone)]
pub struct CommandSpec {
    pub program: String,
    pub args: Vec<String>,
}

impl CommandSpec {
    pub fn new(program: impl Into<String>, args: impl Into<Vec<String>>) -> Self {
        Self {
            program: program.into(),
            args: args.into(),
        }
    }

    pub fn arg(mut self, value: impl Into<String>) -> Self {
        self.args.push(value.into());
        self
    }

    pub fn with_args(mut self, values: impl IntoIterator<Item = impl Into<String>>) -> Self {
        for value in values {
            self.args.push(value.into());
        }
        self
    }
}

#[derive(Debug)]
pub struct CommandOutput {
    program: String,
    args: Vec<String>,
    pub stdout: String,
    pub stderr: String,
    pub status: ExitStatus,
}

pub trait CommandExecutor: Send + Sync {
    fn run(&self, spec: &CommandSpec) -> Result<CommandOutput>;
}

impl CommandOutput {
    pub fn success(&self) -> bool {
        self.status.success()
    }

    pub fn program(&self) -> &str {
        &self.program
    }

    pub fn args(&self) -> &[String] {
        &self.args
    }
}

#[derive(Debug, Default, Clone)]
pub struct SystemCommandExecutor;

impl CommandExecutor for SystemCommandExecutor {
    fn run(&self, spec: &CommandSpec) -> Result<CommandOutput> {
        let mut cmd = Command::new(&spec.program);
        cmd.args(&spec.args)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        let output = cmd
            .output()
            .with_context(|| format!("failed to execute {}", spec.program))?;

        Ok(CommandOutput {
            program: spec.program.clone(),
            args: spec.args.clone(),
            stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
            status: output.status,
        })
    }
}

pub fn run_command_json<T>(executor: &dyn CommandExecutor, spec: &CommandSpec) -> Result<T>
where
    T: DeserializeOwned,
{
    let output = executor.run(spec)?;
    let value = serde_json::from_str(&output.stdout).with_context(|| {
        format!(
            "failed to parse JSON output from {} with args {:?}",
            output.program, output.args
        )
    })?;
    Ok(value)
}
