use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, Result, bail};

use crate::domain::{CmdResult, ProjectRoot};
use crate::toml_support::RuntimeConfig;

pub struct UvClient {
    python: String,
}

pub struct StagedWorkdir {
    path: PathBuf,
}

impl StagedWorkdir {
    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn pyproject_toml(&self) -> PathBuf {
        self.path.join("pyproject.toml")
    }

    pub fn uv_lock(&self) -> PathBuf {
        self.path.join("uv.lock")
    }
}

impl UvClient {
    pub fn new(python: impl Into<String>) -> Self {
        Self {
            python: python.into(),
        }
    }

    pub fn version() -> Result<CmdResult> {
        run_command("uv", &["--version"], None, &[])
    }

    pub fn find_python(selector: &str) -> Result<CmdResult> {
        run_command(
            "uv",
            &["python", "find", "--no-python-downloads", selector],
            None,
            &[],
        )
    }

    pub fn lock(&self, project_dir: &Path) -> Result<CmdResult> {
        run_command(
            "uv",
            &["lock", "--python", self.python.as_str()],
            Some(project_dir),
            &[],
        )
    }

    pub fn sync(&self, project_dir: &Path, env_path: &Path) -> Result<CmdResult> {
        let env_value = absolute_env_path(env_path, project_dir)?;
        let envs = [("UV_PROJECT_ENVIRONMENT", env_value)];
        run_command(
            "uv",
            &[
                "sync",
                "--python",
                self.python.as_str(),
                "--locked",
                "--exact",
                "--all-groups",
            ],
            Some(project_dir),
            &envs,
        )
    }

    #[allow(dead_code)]
    pub fn add(&self, project_dir: &Path, group: &str, spec: &str, frozen: bool) -> Result<CmdResult> {
        let mut args = vec![
            "add".to_string(),
            "--group".to_string(),
            group.to_string(),
            "--python".to_string(),
            self.python.clone(),
        ];
        if frozen {
            args.push("--frozen".to_string());
        }
        args.push(spec.to_string());
        let refs = args.iter().map(String::as_str).collect::<Vec<_>>();
        run_command("uv", &refs, Some(project_dir), &[])
    }

    #[allow(dead_code)]
    pub fn remove(
        &self,
        project_dir: &Path,
        group: &str,
        pkg: &str,
        frozen: bool,
    ) -> Result<CmdResult> {
        let mut args = vec![
            "remove".to_string(),
            "--group".to_string(),
            group.to_string(),
            "--python".to_string(),
            self.python.clone(),
        ];
        if frozen {
            args.push("--frozen".to_string());
        }
        args.push(pkg.to_string());
        let refs = args.iter().map(String::as_str).collect::<Vec<_>>();
        run_command("uv", &refs, Some(project_dir), &[])
    }
}

pub fn create_staged_workdir(root: &ProjectRoot) -> Result<StagedWorkdir> {
    let workdir = crate::fs_support::create_workdir(&root.work_dir())?;
    crate::fs_support::copy_file(&root.pyproject_toml(), &workdir.path().join("pyproject.toml"))?;
    crate::fs_support::copy_file(&root.uv_lock(), &workdir.path().join("uv.lock"))?;
    Ok(StagedWorkdir {
        path: workdir.path().to_path_buf(),
    })
}

pub fn promote_workdir(workdir: &StagedWorkdir, root: &ProjectRoot) -> Result<()> {
    let pyproject = std::fs::read(workdir.pyproject_toml())
        .with_context(|| format!("failed to read {}", workdir.pyproject_toml().display()))?;
    crate::fs_support::atomic_write(&root.pyproject_toml(), &pyproject)?;

    let lock = std::fs::read(workdir.uv_lock())
        .with_context(|| format!("failed to read {}", workdir.uv_lock().display()))?;
    crate::fs_support::atomic_write(&root.uv_lock(), &lock)?;

    Ok(())
}

pub fn sync_prod(root: &ProjectRoot, config: &RuntimeConfig) -> Result<CmdResult> {
    let python = config
        .runtime
        .python
        .as_deref()
        .filter(|value| !value.trim().is_empty())
        .context("config.toml is missing runtime.python")?;
    let env_path = if config.runtime.prod_env.is_absolute() {
        config.runtime.prod_env.clone()
    } else {
        root.as_path().join(&config.runtime.prod_env)
    };

    UvClient::new(python).sync(root.as_path(), &env_path)
}

fn absolute_env_path(env_path: &Path, cwd: &Path) -> Result<OsString> {
    let absolute = if env_path.is_absolute() {
        env_path.to_path_buf()
    } else {
        cwd.join(env_path)
    };
    if !absolute.is_absolute() {
        bail!("UV_PROJECT_ENVIRONMENT must be absolute: {}", absolute.display());
    }
    Ok(absolute.into_os_string())
}

fn run_command(
    program: &str,
    args: &[&str],
    cwd: Option<&Path>,
    envs: &[(&str, OsString)],
) -> Result<CmdResult> {
    let resolved = resolve_program(program);
    let mut command = Command::new(&resolved);
    command.args(args);
    if let Some(cwd) = cwd {
        command.current_dir(cwd);
    }
    for (key, value) in envs {
        command.env(key, value);
    }

    let output = command
        .output()
        .with_context(|| format!("failed to run `{}`", resolved.display()))?;

    Ok(CmdResult::from_output(output))
}

fn resolve_program(program: &str) -> PathBuf {
    if program == "uv" {
        if let Some(path) = std::env::var_os("GOV_UV_BIN") {
            return PathBuf::from(path);
        }
    }

    PathBuf::from(program)
}
