//! Executes shell commands in their task directories and stops on failure unless configured to continue.

use std::path::Path;
use std::process::Command;

use anyhow::{Context, Result, bail};

use super::Step;
use crate::paths::PathExt;

pub struct RunOptions {
    /// Displays commands without execution.
    pub dry_run: bool,
    /// Stops on the first failure; otherwise, executes all steps and fails afterward.
    pub fail_fast: bool,
}

impl RunOptions {
    /// Executes `steps` in repository `root`, printing each command before running it.
    pub fn run(&self, root: &Path, steps: &[Step]) -> Result<()> {
        let mut failures = Vec::new();

        for step in steps {
            println!(
                "== {} ({})",
                step.label,
                if step.cwd.as_os_str().is_empty() {
                    ".".to_owned()
                } else {
                    step.cwd.to_slash()
                }
            );
            println!("$ {}", step.command);

            if self.dry_run {
                continue;
            }

            let status = step
                .shell_command(root)
                .status()
                .with_context(|| format!("running {}", step.command))?;

            if !status.success() {
                if self.fail_fast {
                    bail!("{} failed: {}", step.label, step.command);
                }

                failures.push(step.label.clone());
            }
        }

        if !failures.is_empty() {
            bail!("{} task(s) failed: {}", failures.len(), failures.join(", "));
        }

        Ok(())
    }
}

impl Step {
    /// Runs the command through the shell (`sh -c`, `cmd /C` on Windows) in its folder below `root` with `env`.
    fn shell_command(&self, root: &Path) -> Command {
        let (shell, flag) = if cfg!(windows) {
            ("cmd", "/C")
        } else {
            ("sh", "-c")
        };
        let mut command = Command::new(shell);

        command
            .arg(flag)
            .arg(&self.command)
            .current_dir(root.join(&self.cwd))
            .envs(&self.env);

        command
    }
}
