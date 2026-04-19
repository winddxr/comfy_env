#![allow(dead_code)]

use std::path::Path;
use std::process::Command;

use anyhow::{Context, Result};

use crate::domain::CmdResult;

pub struct PythonClient;

impl PythonClient {
    pub fn version(python: &Path) -> Result<CmdResult> {
        let output = Command::new(python)
            .arg("--version")
            .output()
            .with_context(|| format!("failed to run `{}`", python.display()))?;

        Ok(CmdResult::from_output(output))
    }
}
