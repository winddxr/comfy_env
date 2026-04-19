use anyhow::{Context, anyhow};

use crate::cli::UndoArgs;
use crate::dependency_sync;
use crate::domain::{AppError, AppResult, OpId, OpStatus, ProjectRoot};
use crate::safety_guards::{self, DriftCheck};
use crate::state_ledger::{self, OpKind};
use crate::toml_support::{self, RuntimeConfig};

pub fn cmd_undo(args: UndoArgs, root: &ProjectRoot) -> AppResult<()> {
    let config = require_config(root)?;
    let target_id = OpId(args.op_id);
    let target = state_ledger::load_operation(root, &target_id)?;

    if target.status != OpStatus::Success || !target.undoable {
        return Err(AppError::usage(format!(
            "operation {} is not undoable",
            target_id.0
        )));
    }

    match safety_guards::check_undo_drift(&target_id, root)? {
        DriftCheck::Clean => {}
        DriftCheck::Drifted(files) => {
            return Err(AppError::usage(format!(
                "hash drift detected: {}",
                files.join(", ")
            )));
        }
    }

    let undo_op = safety_guards::op_begin(OpKind::ManualUndo, &format!("undo:{}", target_id.0), root)?;

    let undo_result = (|| -> anyhow::Result<()> {
        safety_guards::op_restore(&target_id, root)?;
        let sync = dependency_sync::sync_prod(root, &config)?;
        if !sync.success {
            return Err(anyhow!(
                "prod sync failed during undo: {}",
                sync.summary_line()
            ));
        }
        state_ledger::mark_operation_undone(root, &target_id, &undo_op)?;
        Ok(())
    })();

    match undo_result {
        Ok(()) => {
            safety_guards::op_finalize(&undo_op, true, None, root)?;
            println!("Undo completed: {}", target_id.0);
            Ok(())
        }
        Err(error) => rollback_failed_undo(&undo_op, root, &config, error),
    }
}

fn require_config(root: &ProjectRoot) -> AppResult<RuntimeConfig> {
    toml_support::read_config(&root.config_toml())?
        .ok_or_else(|| AppError::usage("config.toml is required for undo"))
}

fn rollback_failed_undo(
    undo_op: &OpId,
    root: &ProjectRoot,
    config: &RuntimeConfig,
    error: anyhow::Error,
) -> AppResult<()> {
    let note = error.to_string();
    let restore = safety_guards::op_restore(undo_op, root)
        .with_context(|| format!("failed to restore after undo error: {note}"));
    let sync = restore.and_then(|_| {
        dependency_sync::sync_prod(root, config)
            .with_context(|| format!("failed to re-sync prod after undo restore: {note}"))
    });
    let finalize = safety_guards::op_finalize(undo_op, false, Some(note.clone()), root);

    if let Err(finalize_error) = finalize {
        return Err(AppError::from(anyhow!(
            "{note}; additionally failed to finalize undo operation: {finalize_error:#}"
        )));
    }

    if let Err(restore_error) = sync {
        return Err(AppError::from(anyhow!(
            "{note}; additionally rollback failed: {restore_error:#}"
        )));
    }

    Err(AppError::from(anyhow!(note)))
}
