use std::path::{Path, PathBuf};

use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct ConfigToml {
    pub paths: Option<ConfigPaths>,
    pub runtime: Option<ConfigRuntime>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ConfigPaths {
    pub comfyui_dir: Option<PathBuf>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ConfigRuntime {
    pub python: Option<String>,
    pub prod_env: Option<PathBuf>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PyProjectToml {
    #[serde(rename = "dependency-groups")]
    pub dependency_groups: Option<DependencyGroups>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DependencyGroups {
    pub core: Option<Vec<String>>,
    pub torch: Option<Vec<String>>,
    pub overrides: Option<Vec<String>>,
}

impl DependencyGroups {
    pub fn group(&self, name: &str) -> Option<&Vec<String>> {
        match name {
            "core" => self.core.as_ref(),
            "torch" => self.torch.as_ref(),
            "overrides" => self.overrides.as_ref(),
            _ => None,
        }
    }
}

pub fn load_config(path: &Path) -> anyhow::Result<Option<ConfigToml>> {
    load_toml(path)
}

pub fn load_pyproject(path: &Path) -> anyhow::Result<Option<PyProjectToml>> {
    load_toml(path)
}

fn load_toml<T>(path: &Path) -> anyhow::Result<Option<T>>
where
    T: for<'de> Deserialize<'de>,
{
    let Some(text) = crate::fs_support::read_text_if_exists(path)? else {
        return Ok(None);
    };

    let parsed = toml_edit::de::from_str(&text)?;
    Ok(Some(parsed))
}
