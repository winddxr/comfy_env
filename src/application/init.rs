use crate::cli::InitArgs;
use crate::domain::{AppError, AppResult, ProjectRoot};

pub fn cmd_init(_args: InitArgs, _root: &ProjectRoot) -> AppResult<()> {
    Err(AppError::unimplemented("init"))
}
