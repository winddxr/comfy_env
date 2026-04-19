use crate::cli::UndoArgs;
use crate::domain::{AppError, AppResult, ProjectRoot};

pub fn cmd_undo(_args: UndoArgs, _root: &ProjectRoot) -> AppResult<()> {
    Err(AppError::unimplemented("undo"))
}
