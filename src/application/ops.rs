use crate::cli::OpInspectArgs;
use crate::domain::{AppError, AppResult, ProjectRoot};

pub fn cmd_op_list(_root: &ProjectRoot) -> AppResult<()> {
    Err(AppError::unimplemented("op list"))
}

pub fn cmd_op_inspect(_args: OpInspectArgs, _root: &ProjectRoot) -> AppResult<()> {
    Err(AppError::unimplemented("op inspect"))
}
