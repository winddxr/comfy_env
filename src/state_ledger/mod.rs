#![allow(dead_code)]

use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result, anyhow, bail};
use serde::{Deserialize, Serialize};

use crate::domain::{OpId, OpStatus, ProjectRoot, TxStatus};
use crate::platform;

#[derive(Debug, Clone, Default, Deserialize)]
pub struct PluginRecord {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub enabled: bool,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct TransactionRecord {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub kind: String,
    #[serde(default)]
    pub status: Option<TxStatus>,
    #[serde(default)]
    pub subject: String,
    #[serde(default)]
    pub created_at: Option<String>,
    #[serde(default)]
    pub updated_at: Option<String>,
}

impl TransactionRecord {
    pub fn sort_key(&self) -> Option<&str> {
        self.updated_at
            .as_deref()
            .or(self.created_at.as_deref())
            .or(Some(self.id.as_str()))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OpKind {
    PinAdd,
    PinRemove,
    ManualUndo,
}

impl std::fmt::Display for OpKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let value = match self {
            Self::PinAdd => "pin_add",
            Self::PinRemove => "pin_remove",
            Self::ManualUndo => "manual_undo",
        };
        f.write_str(value)
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TrackedFileHashes {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pre_sha256: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub post_sha256: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OperationRecord {
    pub op_id: String,
    pub kind: OpKind,
    pub reference: String,
    pub status: OpStatus,
    pub started_at: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ended_at: Option<String>,
    #[serde(default)]
    pub files: BTreeMap<String, TrackedFileHashes>,
    pub backup_dir: String,
    #[serde(default)]
    pub undoable: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub undo_reference: Option<String>,
}

impl OperationRecord {
    pub fn sort_key(&self) -> &str {
        self.ended_at
            .as_deref()
            .or(Some(self.started_at.as_str()))
            .unwrap_or(self.op_id.as_str())
    }

    pub fn backup_dir_path(&self) -> PathBuf {
        platform::from_state_path(&self.backup_dir)
    }
}

pub fn load_plugins(root: &ProjectRoot) -> Result<Vec<PluginRecord>> {
    let path = root.plugins_registry();
    let Some(text) = crate::fs_support::read_text_if_exists(&path)? else {
        return Ok(Vec::new());
    };

    let records = serde_json::from_str::<Vec<PluginRecord>>(&text)
        .with_context(|| format!("failed to parse {}", path.display()))?;
    Ok(records)
}

pub fn load_transactions(root: &ProjectRoot) -> Result<Vec<TransactionRecord>> {
    let dir = root.transactions_dir();
    if !dir.exists() {
        return Ok(Vec::new());
    }

    let mut records = Vec::new();
    for entry in fs::read_dir(&dir).with_context(|| format!("failed to read {}", dir.display()))? {
        let entry = entry.with_context(|| format!("failed to inspect {}", dir.display()))?;
        let path = entry.path();
        if !matches!(path.extension().and_then(|value| value.to_str()), Some("json")) {
            continue;
        }

        let text = fs::read_to_string(&path)
            .with_context(|| format!("failed to read {}", path.display()))?;
        let record = serde_json::from_str::<TransactionRecord>(&text)
            .with_context(|| format!("failed to parse {}", path.display()))?;
        records.push(record);
    }

    records.sort_by(|left, right| right.sort_key().cmp(&left.sort_key()));
    Ok(records)
}

pub fn next_op_id() -> Result<OpId> {
    let duration = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| anyhow!("failed to read system time: {error}"))?;
    let millis = duration.as_millis();
    let suffix = ((duration.as_nanos() as u64) ^ (std::process::id() as u64)) as u32;
    Ok(OpId(format!("{millis:013}-op-{suffix:08x}")))
}

pub fn create_operation(root: &ProjectRoot, op: &OperationRecord) -> Result<()> {
    let path = op_meta_path(root, &OpId(op.op_id.clone()));
    write_operation_path(&path, op)
}

pub fn load_operation(root: &ProjectRoot, op_id: &OpId) -> Result<OperationRecord> {
    let path = op_meta_path(root, op_id);
    let text = fs::read_to_string(&path)
        .with_context(|| format!("failed to read {}", path.display()))?;
    let record = serde_json::from_str::<OperationRecord>(&text)
        .with_context(|| format!("failed to parse {}", path.display()))?;
    Ok(record)
}

pub fn save_operation(root: &ProjectRoot, op: &OperationRecord) -> Result<()> {
    let path = op_meta_path(root, &OpId(op.op_id.clone()));
    write_operation_path(&path, op)
}

pub fn list_operations(root: &ProjectRoot) -> Result<Vec<OperationRecord>> {
    let dir = root.ops_dir();
    if !dir.exists() {
        return Ok(Vec::new());
    }

    let mut operations = Vec::new();
    for entry in fs::read_dir(&dir).with_context(|| format!("failed to read {}", dir.display()))? {
        let entry = entry.with_context(|| format!("failed to inspect {}", dir.display()))?;
        if !entry
            .file_type()
            .with_context(|| format!("failed to inspect {}", entry.path().display()))?
            .is_dir()
        {
            continue;
        }
        let path = entry.path().join("meta.json");
        if !path.exists() {
            continue;
        }
        let text = fs::read_to_string(&path)
            .with_context(|| format!("failed to read {}", path.display()))?;
        let record = serde_json::from_str::<OperationRecord>(&text)
            .with_context(|| format!("failed to parse {}", path.display()))?;
        operations.push(record);
    }

    operations.sort_by(|left, right| right.sort_key().cmp(left.sort_key()));
    Ok(operations)
}

pub fn mark_operation_undone(
    root: &ProjectRoot,
    op_id: &OpId,
    undo_reference: &OpId,
) -> Result<()> {
    let mut record = load_operation(root, op_id)?;
    if record.status != OpStatus::Success {
        bail!("operation {} is not successful", op_id.0);
    }
    record.status = OpStatus::Undone;
    record.ended_at = Some(now_string()?);
    record.undo_reference = Some(undo_reference.0.clone());
    save_operation(root, &record)
}

pub fn now_string() -> Result<String> {
    let duration = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| anyhow!("failed to read system time: {error}"))?;
    Ok(format!(
        "{}.{:03}Z",
        duration.as_secs(),
        duration.subsec_millis()
    ))
}

fn op_meta_path(root: &ProjectRoot, op_id: &OpId) -> PathBuf {
    root.ops_dir().join(&op_id.0).join("meta.json")
}

fn write_operation_path(path: &PathBuf, op: &OperationRecord) -> Result<()> {
    let bytes = serde_json::to_vec_pretty(op)?;
    crate::fs_support::atomic_write(path, &bytes)
}
