use crate::cli::ResolveArgs;
use crate::domain::{AppError, AppResult, ProjectRoot};

pub fn cmd_resolve(_args: ResolveArgs, _root: &ProjectRoot) -> AppResult<()> {
    Err(AppError::unimplemented("resolve"))
}
