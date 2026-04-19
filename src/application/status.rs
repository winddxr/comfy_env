use std::collections::BTreeMap;

use crate::dependency_sync::UvClient;
use crate::domain::{AppResult, CmdResult, ProjectRoot, TxStatus};
use crate::platform;
use crate::source_integration::GitClient;
use crate::state_ledger::{self, TransactionRecord};
use crate::toml_support::{self, ConfigToml, PyProjectToml};

pub fn cmd_status(root: &ProjectRoot) -> AppResult<()> {
    let config = toml_support::load_config(&root.config_toml())?;
    let pyproject = toml_support::load_pyproject(&root.pyproject_toml())?;
    let plugins = state_ledger::load_plugins(root)?;
    let transactions = state_ledger::load_transactions(root)?;
    let runtime_pid = read_runtime_pid(root);
    let runtime_running = runtime_pid.is_some_and(platform::is_process_alive);

    println!("project_root: {}", root.as_path().display());
    println!("config_ready: {}", config.is_some());
    println!("comfyui_dir: {}", config_comfyui_dir(config.as_ref()));
    println!("python: {}", config_python(config.as_ref()));
    println!(
        "prod_env_exists: {}",
        prod_env_exists(root, config.as_ref())
    );
    println!("torch_ready: {}", group_ready(pyproject.as_ref(), "torch"));
    println!("core_ready: {}", group_ready(pyproject.as_ref(), "core"));
    println!("plugins_registered: {}", plugins.len());
    println!(
        "plugins_enabled: {}",
        plugins.iter().filter(|plugin| plugin.enabled).count()
    );
    println!(
        "transactions_pending: {}",
        pending_transactions(&transactions)
    );
    println!("transactions_recent: {}", transactions.len().min(8));
    println!("runtime_running: {}", runtime_running);
    println!("tool_uv: {}", describe_probe(UvClient::version()));
    println!("tool_git: {}", describe_probe(GitClient::version()));
    println!("tool_python: {}", python_probe(config.as_ref()));

    print_transaction_counts(&transactions);
    print_recent_transactions(&transactions);
    print_notes(
        root,
        config.as_ref(),
        pyproject.as_ref(),
        runtime_pid,
        runtime_running,
    );

    Ok(())
}

fn config_comfyui_dir(config: Option<&ConfigToml>) -> String {
    config
        .and_then(|value| value.paths.as_ref())
        .and_then(|paths| paths.comfyui_dir.as_ref())
        .map(|path| path.display().to_string())
        .unwrap_or_else(|| "-".to_string())
}

fn config_python(config: Option<&ConfigToml>) -> String {
    config
        .and_then(|value| value.runtime.as_ref())
        .and_then(|runtime| runtime.python.as_ref())
        .cloned()
        .unwrap_or_else(|| "-".to_string())
}

fn prod_env_exists(root: &ProjectRoot, config: Option<&ConfigToml>) -> bool {
    let relative = config
        .and_then(|value| value.runtime.as_ref())
        .and_then(|runtime| runtime.prod_env.as_ref())
        .map(|path| root.as_path().join(path))
        .unwrap_or_else(|| root.prod_env_dir());

    relative.exists()
}

fn group_ready(pyproject: Option<&PyProjectToml>, group: &str) -> bool {
    pyproject
        .and_then(|value| value.dependency_groups.as_ref())
        .and_then(|groups| groups.group(group))
        .is_some_and(|items| !items.is_empty())
}

fn pending_transactions(transactions: &[TransactionRecord]) -> usize {
    transactions
        .iter()
        .filter(|tx| {
            matches!(
                tx.status,
                Some(TxStatus::Running | TxStatus::NeedsResolution | TxStatus::Resolved)
            )
        })
        .count()
}

fn describe_probe(result: anyhow::Result<CmdResult>) -> String {
    match result {
        Ok(output) if output.success => output.summary_line(),
        Ok(output) => format!(
            "unavailable (exit {}: {})",
            output.exit_code,
            output.summary_line()
        ),
        Err(error) => format!("unavailable ({})", error.root_cause()),
    }
}

fn python_probe(config: Option<&ConfigToml>) -> String {
    let Some(selector) = config
        .and_then(|value| value.runtime.as_ref())
        .and_then(|runtime| runtime.python.as_ref())
    else {
        return "managed by uv during init/run (deferred until config.toml exists)".to_string();
    };

    match UvClient::find_python(selector) {
        Ok(output) if output.success => {
            format!("uv resolved '{selector}' -> {}", output.summary_line())
        }
        Ok(output) => format!(
            "uv could not resolve '{selector}' (exit {}: {})",
            output.exit_code,
            output.summary_line()
        ),
        Err(error) => format!("uv probe failed for '{selector}' ({})", error.root_cause()),
    }
}

fn print_transaction_counts(transactions: &[TransactionRecord]) {
    let mut counts: BTreeMap<String, usize> = BTreeMap::new();

    for record in transactions {
        let key = record
            .status
            .as_ref()
            .map(ToString::to_string)
            .unwrap_or_else(|| "unknown".to_string());
        *counts.entry(key).or_default() += 1;
    }

    if counts.is_empty() {
        println!("transaction_counts: none");
        return;
    }

    let summary = counts
        .into_iter()
        .map(|(status, count)| format!("{status}={count}"))
        .collect::<Vec<_>>()
        .join(", ");
    println!("transaction_counts: {summary}");
}

fn print_recent_transactions(transactions: &[TransactionRecord]) {
    if transactions.is_empty() {
        println!("recent_transactions: none");
        return;
    }

    println!("recent_transactions:");
    for record in transactions.iter().take(8) {
        println!(
            "  - {} | {} | {} | {} | {}",
            display_value(&record.id),
            display_value(&record.kind),
            record
                .status
                .as_ref()
                .map(ToString::to_string)
                .unwrap_or_else(|| "unknown".to_string()),
            display_value(&record.subject),
            display_value(record.sort_key().as_deref().unwrap_or(""))
        );
    }
}

fn print_notes(
    root: &ProjectRoot,
    config: Option<&ConfigToml>,
    pyproject: Option<&PyProjectToml>,
    runtime_pid: Option<u32>,
    runtime_running: bool,
) {
    let mut notes = Vec::new();

    if config.is_none() {
        notes.push(
            "config.toml is missing; python resolution stays deferred until init".to_string(),
        );
    }
    if pyproject.is_none() {
        notes.push("pyproject.toml is missing; dependency truth is not initialized".to_string());
    }
    if !root.plugins_registry().exists() {
        notes.push("state/plugins.json is missing; plugin registry is not initialized".to_string());
    }
    if runtime_pid.is_some() && !runtime_running {
        notes.push("state/comfyui.pid exists but the process is not alive".to_string());
    }

    if notes.is_empty() {
        println!("notes: none");
        return;
    }

    println!("notes:");
    for note in notes {
        println!("  - {note}");
    }
}

fn read_runtime_pid(root: &ProjectRoot) -> Option<u32> {
    let text = crate::fs_support::read_text_if_exists(&root.pid_file())
        .ok()
        .flatten()?;
    text.trim().parse().ok()
}

fn display_value(value: &str) -> String {
    if value.trim().is_empty() {
        "-".to_string()
    } else {
        value.to_string()
    }
}
