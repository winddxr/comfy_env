use crate::cli::PinMutateArgs;
use crate::domain::{AppError, AppResult, ProjectRoot};

pub fn cmd_pin_add(_args: PinMutateArgs, _root: &ProjectRoot) -> AppResult<()> {
    Err(AppError::unimplemented("pin add"))
}

pub fn cmd_pin_list(_root: &ProjectRoot) -> AppResult<()> {
    Err(AppError::unimplemented("pin list"))
}

pub fn cmd_pin_remove(_args: PinMutateArgs, _root: &ProjectRoot) -> AppResult<()> {
    Err(AppError::unimplemented("pin remove"))
}
