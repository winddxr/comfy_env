use std::process::Command;

use anyhow::{Context, Result};

use crate::domain::CmdResult;

pub struct GitClient;

impl GitClient {
    pub fn version() -> Result<CmdResult> {
        let output = Command::new("git")
            .arg("--version")
            .output()
            .context("failed to run `git`")?;

        Ok(CmdResult::from_output(output))
    }
}
