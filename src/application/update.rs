use crate::cli::{UpdateInspectArgs, UpdatePromoteArgs, UpdateResolveArgs, UpdateRunArgs};
use crate::domain::{AppError, AppResult, ProjectRoot};

pub fn cmd_update_run(_args: UpdateRunArgs, _root: &ProjectRoot) -> AppResult<()> {
    Err(AppError::unimplemented("update run"))
}

pub fn cmd_update_inspect(_args: UpdateInspectArgs, _root: &ProjectRoot) -> AppResult<()> {
    Err(AppError::unimplemented("update inspect"))
}

pub fn cmd_update_abort(_args: UpdateInspectArgs, _root: &ProjectRoot) -> AppResult<()> {
    Err(AppError::unimplemented("update abort"))
}

pub fn cmd_update_promote(_args: UpdatePromoteArgs, _root: &ProjectRoot) -> AppResult<()> {
    Err(AppError::unimplemented("update promote"))
}

pub fn cmd_update_resolve(_args: UpdateResolveArgs, _root: &ProjectRoot) -> AppResult<()> {
    Err(AppError::unimplemented("update resolve"))
}
