# AGENTS.md

## Project Overview

`comfy_env` is a sidecar governance CLI for ComfyUI custom-node dependency changes. Optimize for correctness, auditability, and rollback safety over speed; a broken venv is worse than a slow command. All production dependency mutations must go through a candidate transaction before promotion.

## Tech Stack

- Linux-only runtime and test environment
- Bash entrypoint: `bin/gov` with `set -euo pipefail`
- Inline Python heredocs for TOML/JSON parsing
- `uv` for lock/sync/add/remove/freeze
- `git` for plugin clone/checkout
- Local truth: `config.toml`, `pyproject.toml`, `uv.lock`
- Durable state: `state/*.json`, `state/transactions/*.json`

## Commands

- Test: `bash tests/test_gov_cli.sh`
- Help: `bin/gov help`
- Status: `bin/gov status`
- Hero flow: `bin/gov node add <url>` → `bin/gov tx run` → `bin/gov tx inspect` → `bin/gov tx promote`

## Architecture

- `bin/gov` is the only entrypoint; command dispatch and subsystem orchestration stay there unless `dev-docs/conventions/code-map.md` changes too.
- `pyproject.toml` + `uv.lock` are the dependency source of truth; `state/plugins.json` is registry metadata, not dependency truth.
- Candidate transactions are the only supported way to observe dependency impact before promotion into `.venv-prod`.
- `dev-docs/` is the canonical design set; if `docs/` conflicts, follow `dev-docs/` and code.

## Coding Conventions

- Implement commands as `cmd_<name>()` functions dispatched from the `main()` case block.
- Keep helpers grouped by subsystem: state ledger, dependency sync, source integration, safety guards, runtime executor.
- Use minimal inline Python via `<<'PY'`; prefer small targeted snippets over Bash re-implementations of TOML/JSON logic.
- Use local variables inside functions; do not introduce mutable globals beyond declared `*_DIR` / `*_FILE` constants.
- Normalize dependency group names through `normalize_group_name()`.
- Guard external tools with `require_cmd` / `require_python` before use.

## Boundaries

- Treat this as a Linux-only project; do not add Windows-specific logic, instructions, or validation paths unless the project scope changes.
- Check current `uv` capabilities before inventing custom dependency-management logic.
- Never edit `*.template`, `state/*`, `.venv-prod/`, or `.venv-candidate/` by hand; use the existing ledger and sync flows.
- Do not add external tool dependencies beyond `uv`, `git`, and Python.
- Do not bypass transaction, backup, or promote safety paths for prod-affecting changes.

## Verification

- Run `bash tests/test_gov_cli.sh` before considering work complete.
- For new commands, exercise one happy path and one failure path manually.
- For dependency-mutation changes, verify real `uv lock` and `uv sync` behavior in a project layout.
- For remove/promote/undo changes, verify backup and restore behavior rather than relying on code inspection alone.

## Read Only When Needed

Do not read every document listed below by default. Read only the document(s) that match the current task.

- If you are changing CLI commands or command dispatch, read `dev-docs/application-core/spec.md` and `dev-docs/conventions/code-map.md`.
- If the task touches transaction records, conflict artifacts, or undo metadata, read `dev-docs/subsystems/state-ledger/spec.md`.
- If the task touches `tx promote`, `node remove`, `undo`, backups, or rollback behavior, read `dev-docs/subsystems/safety-guards/spec.md` and `dev-docs/policies/rollback-safety.md`.
- If the task changes dependency resolution, lockfiles, sync behavior, or `uv` flows, read `dev-docs/subsystems/dependency-sync/spec.md`.
- If the task changes plugin clone/checkout or source snapshot behavior, read `dev-docs/subsystems/source-integration/spec.md`.
- If the task changes runtime launch, stop, timeout, PID, or logs, read `dev-docs/subsystems/runtime-executor/spec.md`.
- If the task touches core-package approvals or override pin design, read `dev-docs/policies/core-impact-gate.md` and `dev-docs/adr/001-global-pin-management.md`.
- If the task changes environment export/import behavior, read `dev-docs/application-core/spec.md` for `UC-010` and `core#KF-011`.
- Only read `dev-docs/architecture-haiku.md` when the task spans multiple subsystems or requires repo-wide invariants first.
