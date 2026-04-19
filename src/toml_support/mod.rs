use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use serde::Deserialize;
use toml_edit::{Array, DocumentMut, Item, Table, Value, value};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeConfig {
    pub paths: ConfigPaths,
    pub runtime: ConfigRuntime,
    pub tx: TxConfig,
    pub policy: PolicyConfig,
    pub ops: OpsConfig,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigPaths {
    pub comfyui_dir: Option<PathBuf>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigRuntime {
    pub python: Option<String>,
    pub prod_env: PathBuf,
    pub candidate_root: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TxConfig {
    pub timeout_seconds: u32,
    pub smoke_test: Option<SmokeTestConfig>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct SmokeTestConfig {
    pub program: String,
    pub args: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PolicyConfig {
    pub core_packages: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpsConfig {
    pub retention_count: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GroupEditMode {
    UpsertExact,
    RemoveNames,
}

#[derive(Debug, Deserialize)]
struct RawConfig {
    paths: Option<RawConfigPaths>,
    runtime: Option<RawConfigRuntime>,
    tx: Option<RawTxConfig>,
    policy: Option<RawPolicyConfig>,
    ops: Option<RawOpsConfig>,
}

#[derive(Debug, Deserialize)]
struct RawConfigPaths {
    comfyui_dir: Option<PathBuf>,
}

#[derive(Debug, Deserialize)]
struct RawConfigRuntime {
    python: Option<String>,
    prod_env: Option<PathBuf>,
    candidate_root: Option<PathBuf>,
}

#[derive(Debug, Deserialize)]
struct RawTxConfig {
    timeout_seconds: Option<u32>,
    smoke_test_cmd: Option<String>,
    smoke_test: Option<SmokeTestConfig>,
}

#[derive(Debug, Deserialize)]
struct RawPolicyConfig {
    core_packages: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
struct RawOpsConfig {
    retention_count: Option<u32>,
}

pub fn read_config(path: &Path) -> Result<Option<RuntimeConfig>> {
    let Some(text) = crate::fs_support::read_text_if_exists(path)? else {
        return Ok(None);
    };

    let raw = toml_edit::de::from_str::<RawConfig>(&text)
        .with_context(|| format!("failed to parse {}", path.display()))?;
    Ok(Some(runtime_config_from_raw(raw)))
}

pub fn read_dependency_group(path: &Path, group: &str) -> Result<Vec<String>> {
    let Some(text) = crate::fs_support::read_text_if_exists(path)? else {
        return Ok(Vec::new());
    };
    let doc = parse_pyproject(path, &text)?;
    let Some(item) = dependency_groups_table(&doc).and_then(|table| table.get(group)) else {
        return Ok(Vec::new());
    };
    let Some(array) = item.as_array() else {
        bail!("dependency-groups.{group} is not an array");
    };

    Ok(array
        .iter()
        .filter_map(|value| value.as_str().map(ToOwned::to_owned))
        .collect())
}

pub fn rewrite_dependency_group(
    path: &Path,
    group: &str,
    mode: GroupEditMode,
    specs: &[String],
) -> Result<()> {
    let Some(text) = crate::fs_support::read_text_if_exists(path)? else {
        bail!("missing {}", path.display());
    };
    let mut doc = parse_pyproject(path, &text)?;
    let groups = dependency_groups_table_mut(&mut doc)?;

    let existing_values = groups
        .get(group)
        .and_then(Item::as_array)
        .map(|array| array.iter().cloned().collect::<Vec<_>>())
        .unwrap_or_default();

    let mut output = Vec::new();
    match mode {
        GroupEditMode::UpsertExact => {
            let requested = dedupe_exact_specs(specs);
            let names = requested
                .iter()
                .map(|spec| normalize_package_name(spec))
                .collect::<HashSet<_>>();

            for entry in existing_values {
                if entry
                    .as_str()
                    .is_some_and(|spec| names.contains(&normalize_package_name(spec)))
                {
                    continue;
                }
                output.push(entry);
            }
            for spec in requested {
                output.push(Value::from(spec));
            }
        }
        GroupEditMode::RemoveNames => {
            let remove = specs
                .iter()
                .map(|name| normalize_package_name(name))
                .collect::<HashSet<_>>();
            for entry in existing_values {
                if entry
                    .as_str()
                    .is_some_and(|spec| remove.contains(&normalize_package_name(spec)))
                {
                    continue;
                }
                output.push(entry);
            }
        }
    }

    let mut array = Array::new();
    for value in output {
        array.push_formatted(value);
    }

    match groups.get_mut(group) {
        Some(item) => {
            *item = value(array);
        }
        None => {
            groups.insert(group, Item::Value(Value::Array(array)));
        }
    }

    crate::fs_support::atomic_write(path, doc.to_string().as_bytes())?;
    Ok(())
}

pub fn normalize_package_name(spec: &str) -> String {
    let trimmed = spec.trim();
    if trimmed.is_empty() {
        return String::new();
    }

    let token = trimmed
        .split(['<', '>', '=', '!', '~', ';', '[', '@'])
        .next()
        .unwrap_or(trimmed)
        .trim();

    let mut normalized = String::with_capacity(token.len());
    let mut previous_dash = false;
    for ch in token.chars() {
        let mapped = if matches!(ch, '-' | '_' | '.') {
            '-'
        } else {
            ch.to_ascii_lowercase()
        };
        if mapped == '-' {
            if previous_dash {
                continue;
            }
            previous_dash = true;
            normalized.push(mapped);
        } else {
            previous_dash = false;
            normalized.push(mapped);
        }
    }

    normalized.trim_matches('-').to_string()
}

pub fn is_exact_pin(spec: &str) -> bool {
    let Some((name, version)) = spec.split_once("==") else {
        return false;
    };

    !normalize_package_name(name).is_empty()
        && !version.trim().is_empty()
        && !version.contains(char::is_whitespace)
        && !name.contains('@')
}

fn runtime_config_from_raw(raw: RawConfig) -> RuntimeConfig {
    let legacy_smoke = raw
        .tx
        .as_ref()
        .and_then(|tx| tx.smoke_test_cmd.as_deref())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(parse_legacy_smoke_test);

    RuntimeConfig {
        paths: ConfigPaths {
            comfyui_dir: raw.paths.and_then(|paths| paths.comfyui_dir),
        },
        runtime: ConfigRuntime {
            python: raw.runtime.as_ref().and_then(|runtime| runtime.python.clone()),
            prod_env: raw
                .runtime
                .as_ref()
                .and_then(|runtime| runtime.prod_env.clone())
                .unwrap_or_else(|| PathBuf::from(".venv-prod")),
            candidate_root: raw
                .runtime
                .as_ref()
                .and_then(|runtime| runtime.candidate_root.clone())
                .unwrap_or_else(|| PathBuf::from(".venv-candidate")),
        },
        tx: TxConfig {
            timeout_seconds: raw
                .tx
                .as_ref()
                .and_then(|tx| tx.timeout_seconds)
                .unwrap_or(120),
            smoke_test: raw.tx.and_then(|tx| tx.smoke_test).or(legacy_smoke),
        },
        policy: PolicyConfig {
            core_packages: raw
                .policy
                .and_then(|policy| policy.core_packages)
                .unwrap_or_else(default_core_packages),
        },
        ops: OpsConfig {
            retention_count: raw.ops.and_then(|ops| ops.retention_count).unwrap_or(100),
        },
    }
}

fn parse_legacy_smoke_test(command: &str) -> SmokeTestConfig {
    let mut parts = command
        .split_whitespace()
        .filter(|part| !part.is_empty())
        .map(ToOwned::to_owned)
        .collect::<Vec<_>>();
    let program = parts.first().cloned().unwrap_or_else(|| "python".to_string());
    if !parts.is_empty() {
        parts.remove(0);
    }
    SmokeTestConfig {
        program,
        args: parts,
    }
}

fn default_core_packages() -> Vec<String> {
    vec![
        "torch".to_string(),
        "torchvision".to_string(),
        "torchaudio".to_string(),
        "xformers".to_string(),
        "triton".to_string(),
        "onnxruntime".to_string(),
        "onnxruntime-gpu".to_string(),
        "numpy".to_string(),
    ]
}

fn dedupe_exact_specs(specs: &[String]) -> Vec<String> {
    let mut latest_by_name = HashMap::new();
    for (index, spec) in specs.iter().enumerate() {
        latest_by_name.insert(normalize_package_name(spec), (index, spec.clone()));
    }

    let mut deduped = latest_by_name.into_values().collect::<Vec<_>>();
    deduped.sort_by_key(|(index, _)| *index);
    deduped.into_iter().map(|(_, spec)| spec).collect()
}

fn parse_pyproject(path: &Path, text: &str) -> Result<DocumentMut> {
    text.parse::<DocumentMut>()
        .with_context(|| format!("failed to parse {}", path.display()))
}

fn dependency_groups_table(doc: &DocumentMut) -> Option<&Table> {
    doc.as_table()
        .get("dependency-groups")
        .and_then(Item::as_table)
}

fn dependency_groups_table_mut(doc: &mut DocumentMut) -> Result<&mut Table> {
    if doc["dependency-groups"].is_none() {
        doc["dependency-groups"] = Item::Table(Table::new());
    }

    doc["dependency-groups"]
        .as_table_mut()
        .context("dependency-groups must be a table")
}

#[cfg(test)]
mod tests {
    use super::{GroupEditMode, is_exact_pin, normalize_package_name, rewrite_dependency_group};
    use crate::fs_support::read_text_if_exists;

    fn temp_pyproject(contents: &str) -> tempfile::TempDir {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::write(dir.path().join("pyproject.toml"), contents).expect("write pyproject");
        dir
    }

    #[test]
    fn normalize_package_names_matches_policy() {
        assert_eq!(normalize_package_name("NumPy"), "numpy");
        assert_eq!(normalize_package_name("foo_bar.baz"), "foo-bar-baz");
        assert_eq!(normalize_package_name("demo @ https://x"), "demo");
    }

    #[test]
    fn exact_pin_requires_pkg_and_version() {
        assert!(is_exact_pin("numpy==1.26.4"));
        assert!(!is_exact_pin("numpy>=1.26"));
        assert!(!is_exact_pin("==1.26.4"));
        assert!(!is_exact_pin("numpy=="));
    }

    #[test]
    fn rewrite_preserves_quoted_overrides_key() {
        let dir = temp_pyproject(
            r#"[dependency-groups]
"overrides" = []
"#,
        );
        let path = dir.path().join("pyproject.toml");
        rewrite_dependency_group(
            &path,
            "overrides",
            GroupEditMode::UpsertExact,
            &[String::from("transformers==4.44.0")],
        )
        .expect("rewrite");

        let text = read_text_if_exists(&path)
            .expect("read")
            .expect("exists");
        assert!(text.contains("\"overrides\" = ["));
    }

    #[test]
    fn rewrite_dedupes_existing_entries_and_uses_last_wins() {
        let dir = temp_pyproject(
            r#"[dependency-groups]
overrides = [
  "numpy==1.26.3",
  "numpy==1.26.4",
  "pillow==10.0.0",
]
"#,
        );
        let path = dir.path().join("pyproject.toml");
        rewrite_dependency_group(
            &path,
            "overrides",
            GroupEditMode::UpsertExact,
            &[String::from("numpy==1.26.1"), String::from("numpy==1.26.2")],
        )
        .expect("rewrite");

        let text = read_text_if_exists(&path)
            .expect("read")
            .expect("exists");
        assert!(text.contains("numpy==1.26.2"));
        assert!(!text.contains("numpy==1.26.1"));
        assert!(!text.contains("numpy==1.26.3"));
        assert!(!text.contains("numpy==1.26.4"));
        assert!(text.contains("pillow==10.0.0"));
    }

    #[test]
    fn rewrite_remove_names_removes_all_duplicates() {
        let dir = temp_pyproject(
            r#"[dependency-groups]
overrides = [
  "numpy==1.26.3",
  "numpy==1.26.4",
  "pillow==10.0.0",
]
"#,
        );
        let path = dir.path().join("pyproject.toml");
        rewrite_dependency_group(
            &path,
            "overrides",
            GroupEditMode::RemoveNames,
            &[String::from("numpy")],
        )
        .expect("rewrite");

        let text = read_text_if_exists(&path)
            .expect("read")
            .expect("exists");
        assert!(!text.contains("numpy==1.26.3"));
        assert!(!text.contains("numpy==1.26.4"));
        assert!(text.contains("pillow==10.0.0"));
    }
}
