use crate::cli::{EnvExportArgs, EnvImportArgs};
use crate::domain::{AppError, AppResult, ProjectRoot};

pub fn cmd_env_export(_args: EnvExportArgs, _root: &ProjectRoot) -> AppResult<()> {
    Err(AppError::unimplemented("env export"))
}

pub fn cmd_env_import(_args: EnvImportArgs, _root: &ProjectRoot) -> AppResult<()> {
    Err(AppError::unimplemented("env import"))
}
