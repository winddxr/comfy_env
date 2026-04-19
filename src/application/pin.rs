use anyhow::{Context, Result, anyhow, bail};

use crate::cli::PinMutateArgs;
use crate::dependency_sync::{self, UvClient};
use crate::domain::{AppError, AppResult, OpId, ProjectRoot};
use crate::safety_guards;
use crate::state_ledger::OpKind;
use crate::toml_support::{self, RuntimeConfig};

const TORCH_FAMILY: [&str; 3] = ["torch", "torchvision", "torchaudio"];
const RECOMMENDED_PINS: [&str; 2] = ["numpy", "transformers"];

pub fn cmd_pin_add(args: PinMutateArgs, root: &ProjectRoot) -> AppResult<()> {
    let config = require_config(root)?;
    let python = config_python(&config)?;
    let specs = validate_pin_specs(&args.specs)?;
    warn_for_non_recommended(&specs);
    let uv = UvClient::new(python);

    let op_id = safety_guards::op_begin(OpKind::PinAdd, &specs.join(", "), root)?;
    let staged = dependency_sync::create_staged_workdir(root)?;

    let mutation = (|| -> Result<()> {
        toml_support::rewrite_dependency_group(
            &staged.pyproject_toml(),
            "overrides",
            toml_support::GroupEditMode::UpsertExact,
            &specs,
        )?;

        let lock = uv.lock(staged.path())?;
        if !lock.success {
            bail!("pin add failed during lock: {}", lock.summary_line());
        }

        dependency_sync::promote_workdir(&staged, root)?;

        let sync = dependency_sync::sync_prod(root, &config)?;
        if !sync.success {
            bail!("prod sync failed during pin add: {}", sync.summary_line());
        }

        let smoke = safety_guards::run_smoke_test(&root.as_path().join(&config.runtime.prod_env), &config)?;
        if !smoke.passed {
            bail!("smoke test failed during pin add: {}", smoke.detail);
        }

        Ok(())
    })();

    match mutation {
        Ok(()) => {
            safety_guards::op_finalize(&op_id, true, None, root)?;
            println!("Pins added.");
            Ok(())
        }
        Err(error) => rollback_and_fail(&op_id, root, &config, error),
    }
}

pub fn cmd_pin_list(root: &ProjectRoot) -> AppResult<()> {
    let pins = toml_support::read_dependency_group(&root.pyproject_toml(), "overrides")?;
    if pins.is_empty() {
        println!("No pins in overrides group.");
        return Ok(());
    }

    for pin in pins {
        println!("{pin}");
    }
    Ok(())
}

pub fn cmd_pin_remove(args: PinMutateArgs, root: &ProjectRoot) -> AppResult<()> {
    let config = require_config(root)?;
    let python = config_python(&config)?;
    let packages = validate_remove_specs(&args.specs)?;
    let current = toml_support::read_dependency_group(&root.pyproject_toml(), "overrides")?;
    ensure_packages_exist(&current, &packages)?;
    let uv = UvClient::new(python);

    let op_id = safety_guards::op_begin(OpKind::PinRemove, &packages.join(", "), root)?;
    let staged = dependency_sync::create_staged_workdir(root)?;

    let mutation = (|| -> Result<()> {
        let remove_specs = packages.iter().map(ToString::to_string).collect::<Vec<_>>();
        toml_support::rewrite_dependency_group(
            &staged.pyproject_toml(),
            "overrides",
            toml_support::GroupEditMode::RemoveNames,
            &remove_specs,
        )?;

        let lock = uv.lock(staged.path())?;
        if !lock.success {
            bail!("pin remove failed during lock: {}", lock.summary_line());
        }

        dependency_sync::promote_workdir(&staged, root)?;

        let sync = dependency_sync::sync_prod(root, &config)?;
        if !sync.success {
            bail!("prod sync failed during pin remove: {}", sync.summary_line());
        }

        let smoke = safety_guards::run_smoke_test(&root.as_path().join(&config.runtime.prod_env), &config)?;
        if !smoke.passed {
            bail!("smoke test failed during pin remove: {}", smoke.detail);
        }

        Ok(())
    })();

    match mutation {
        Ok(()) => {
            safety_guards::op_finalize(&op_id, true, None, root)?;
            println!("Pins removed.");
            Ok(())
        }
        Err(error) => rollback_and_fail(&op_id, root, &config, error),
    }
}

fn require_config(root: &ProjectRoot) -> AppResult<RuntimeConfig> {
    toml_support::read_config(&root.config_toml())?
        .ok_or_else(|| AppError::usage("config.toml is required for mutating pin commands"))
}

fn config_python(config: &RuntimeConfig) -> AppResult<&str> {
    config
        .runtime
        .python
        .as_deref()
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| AppError::usage("config.toml is missing runtime.python"))
}

fn validate_pin_specs(specs: &[String]) -> AppResult<Vec<String>> {
    if specs.is_empty() {
        return Err(AppError::usage("at least one pin is required"));
    }

    for spec in specs {
        if !toml_support::is_exact_pin(spec) {
            return Err(AppError::usage(format!("invalid pin format: {spec}")));
        }
        reject_torch_family(&toml_support::normalize_package_name(spec))?;
    }

    Ok(dedupe_last_wins(specs))
}

fn validate_remove_specs(specs: &[String]) -> AppResult<Vec<String>> {
    if specs.is_empty() {
        return Err(AppError::usage("at least one package is required"));
    }

    let mut packages = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for spec in specs {
        let normalized = toml_support::normalize_package_name(spec);
        if normalized.is_empty() {
            return Err(AppError::usage(format!("invalid package name: {spec}")));
        }
        reject_torch_family(&normalized)?;
        if seen.insert(normalized.clone()) {
            packages.push(normalized);
        }
    }
    Ok(packages)
}

fn reject_torch_family(name: &str) -> AppResult<()> {
    if TORCH_FAMILY.contains(&name) {
        return Err(AppError::usage(
            "torch-family packages are managed by 'gov install torch'",
        ));
    }
    Ok(())
}

fn warn_for_non_recommended(specs: &[String]) {
    for spec in specs {
        let normalized = toml_support::normalize_package_name(spec);
        if !RECOMMENDED_PINS.contains(&normalized.as_str()) {
            eprintln!("WARNING: pinning non-recommended package: {spec}");
        }
    }
}

fn dedupe_last_wins(specs: &[String]) -> Vec<String> {
    let mut latest = std::collections::HashMap::new();
    for (index, spec) in specs.iter().enumerate() {
        latest.insert(toml_support::normalize_package_name(spec), (index, spec.clone()));
    }
    let mut ordered = latest.into_values().collect::<Vec<_>>();
    ordered.sort_by_key(|(index, _)| *index);
    ordered.into_iter().map(|(_, spec)| spec).collect()
}

fn ensure_packages_exist(current: &[String], requested: &[String]) -> AppResult<()> {
    let existing = current
        .iter()
        .map(|spec| toml_support::normalize_package_name(spec))
        .collect::<std::collections::HashSet<_>>();

    for package in requested {
        if !existing.contains(package) {
            return Err(AppError::usage(format!(
                "The dependency `{package}` could not be found in `dependency-groups.overrides`"
            )));
        }
    }
    Ok(())
}

fn rollback_and_fail(
    op_id: &OpId,
    root: &ProjectRoot,
    config: &RuntimeConfig,
    error: anyhow::Error,
) -> AppResult<()> {
    let note = error.to_string();

    let restore = safety_guards::op_restore(op_id, root)
        .with_context(|| format!("failed to restore after error: {note}"));
    let sync = restore.and_then(|_| {
        dependency_sync::sync_prod(root, config)
            .with_context(|| format!("failed to re-sync prod after restore: {note}"))
    });
    let finalize = safety_guards::op_finalize(op_id, false, Some(note.clone()), root);

    if let Err(finalize_error) = finalize {
        return Err(AppError::from(anyhow!(
            "{note}; additionally failed to finalize operation: {finalize_error:#}"
        )));
    }

    if let Err(restore_error) = sync {
        return Err(AppError::from(anyhow!(
            "{note}; additionally rollback failed: {restore_error:#}"
        )));
    }

    Err(AppError::from(anyhow!(note)))
}
