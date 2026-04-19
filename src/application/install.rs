use crate::cli::{InstallArgs, InstallTorchArgs};
use crate::domain::{AppError, AppResult, ProjectRoot};

pub fn cmd_install(_args: InstallArgs, _root: &ProjectRoot) -> AppResult<()> {
    Err(AppError::unimplemented("install"))
}

pub fn cmd_install_torch(_args: InstallTorchArgs, _root: &ProjectRoot) -> AppResult<()> {
    Err(AppError::unimplemented("install torch"))
}
