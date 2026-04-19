use std::process::Command;

use anyhow::{Context, Result};

use crate::domain::CmdResult;

pub struct UvClient;

impl UvClient {
    pub fn version() -> Result<CmdResult> {
        run_command("uv", &["--version"])
    }

    pub fn find_python(selector: &str) -> Result<CmdResult> {
        run_command("uv", &["python", "find", "--no-python-downloads", selector])
    }
}

fn run_command(program: &str, args: &[&str]) -> Result<CmdResult> {
    let output = Command::new(program)
        .args(args)
        .output()
        .with_context(|| format!("failed to run `{program}`"))?;

    Ok(CmdResult::from_output(output))
}
