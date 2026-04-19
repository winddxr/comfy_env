use crate::cli::RunArgs;
use crate::domain::{AppError, AppResult, ProjectRoot};

pub fn cmd_run(_args: RunArgs, _root: &ProjectRoot) -> AppResult<()> {
    Err(AppError::unimplemented("run"))
}

pub fn cmd_stop(_root: &ProjectRoot) -> AppResult<()> {
    Err(AppError::unimplemented("stop"))
}
