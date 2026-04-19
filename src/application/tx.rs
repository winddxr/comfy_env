use crate::cli::{TxInspectArgs, TxPromoteArgs, TxRunArgs};
use crate::domain::{AppError, AppResult, ProjectRoot};

pub fn cmd_tx_run(_args: TxRunArgs, _root: &ProjectRoot) -> AppResult<()> {
    Err(AppError::unimplemented("tx run"))
}

pub fn cmd_tx_inspect(_args: TxInspectArgs, _root: &ProjectRoot) -> AppResult<()> {
    Err(AppError::unimplemented("tx inspect"))
}

pub fn cmd_tx_abort(_args: TxInspectArgs, _root: &ProjectRoot) -> AppResult<()> {
    Err(AppError::unimplemented("tx abort"))
}

pub fn cmd_tx_promote(_args: TxPromoteArgs, _root: &ProjectRoot) -> AppResult<()> {
    Err(AppError::unimplemented("tx promote"))
}
