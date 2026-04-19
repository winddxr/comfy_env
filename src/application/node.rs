use crate::cli::{NodeAddArgs, NodeRemoveArgs};
use crate::domain::{AppError, AppResult, ProjectRoot};

pub fn cmd_node_add(_args: NodeAddArgs, _root: &ProjectRoot) -> AppResult<()> {
    Err(AppError::unimplemented("node add"))
}

pub fn cmd_node_remove(_args: NodeRemoveArgs, _root: &ProjectRoot) -> AppResult<()> {
    Err(AppError::unimplemented("node remove"))
}
