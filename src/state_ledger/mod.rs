#![allow(dead_code)]

use std::fs;

use anyhow::Context;
use serde::Deserialize;

use crate::domain::{ProjectRoot, TxStatus};

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

pub fn load_plugins(root: &ProjectRoot) -> anyhow::Result<Vec<PluginRecord>> {
    let path = root.plugins_registry();
    let Some(text) = crate::fs_support::read_text_if_exists(&path)? else {
        return Ok(Vec::new());
    };

    let records = serde_json::from_str::<Vec<PluginRecord>>(&text)
        .with_context(|| format!("failed to parse {}", path.display()))?;
    Ok(records)
}

pub fn load_transactions(root: &ProjectRoot) -> anyhow::Result<Vec<TransactionRecord>> {
    let dir = root.transactions_dir();
    if !dir.exists() {
        return Ok(Vec::new());
    }

    let mut records = Vec::new();
    for entry in fs::read_dir(&dir).with_context(|| format!("failed to read {}", dir.display()))? {
        let entry = entry.with_context(|| format!("failed to inspect {}", dir.display()))?;
        let path = entry.path();
        if !matches!(
            path.extension().and_then(|value| value.to_str()),
            Some("json")
        ) {
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
