# AGENTS.md

## Project Overview

`comfy_env` is a sidecar governance CLI for ComfyUI custom-node dependency changes. Optimize for correctness, auditability, and rollback safety over speed; a broken venv is worse than a slow command. All production dependency mutations must go through a candidate transaction before promotion.

The CLI is implemented in **Rust** as a single binary `gov`. Target platforms: **Linux + Windows**.

## Tech Stack

- **Language**: Rust (single crate, single binary)
- **CLI**: `clap` with derive macros
- **TOML editing**: `toml_edit` (format-preserving)
- **JSON state**: `serde` + `serde_json`
- **Error handling**: `anyhow` at command layer; typed boundaries on critical mutation paths
- **External tools** (subprocess calls, not embedded): `uv`, `git`, Python
- **Local truth files**: `config.toml`, `pyproject.toml`, `uv.lock`
- **Durable state**: `state/*.json`, `state/transactions/*.json`, `state/ops/*`

## Commands

- Build: `cargo build --release`
- Test: `cargo test`
- Test (legacy Linux behavioral baseline): `bash tests/test_gov_cli.sh`
- Help: `gov help`
- Status: `gov status`
- Hero flow: `gov node add <url>` → `gov tx run` → `gov tx inspect` → `gov tx promote`

## Architecture

- `gov` (Rust binary) is the **sole entry point**. No Bash wrapper, dispatcher, or shim.
- `pyproject.toml` + `uv.lock` are the dependency source of truth; `state/plugins.json` is registry metadata, not dependency truth.
- Candidate transactions are the only supported way to observe dependency impact before promotion into `.venv-prod`.
- Cross-platform abstractions (venv Python locator, process runner, path normalization, atomic writes) live in `src/platform/` and `src/fs_support/`.
- See `dev-docs/adr/003-rust-rewrite-plan.md` for full architecture and module layout.

## Coding Conventions

- One module per subsystem under `src/`: `application/`, `domain/`, `state_ledger/`, `safety_guards/`, `dependency_sync/`, `source_integration/`, `runtime_executor/`, `platform/`, `toml_support/`, `fs_support/`.
- Command implementations live in `src/application/<command>.rs`.
- Use newtypes for IDs: `TxId`, `OpId`, `NodeId`, `GroupName`.
- Use enums for state: `TxStatus`, `OpStatus`, `RunOutcome` — never bare strings.
- Error handling: `anyhow::Result` at command layer; typed return values on critical mutation paths that indicate whether side effects occurred and whether restore is needed.
- External tools (`uv`, `git`, `python`) wrapped in client structs returning `CmdResult`. Never call `Command::new()` directly from application code.
- TOML editing via `toml_edit` — modify only the target node, do not reformat the file.
- File writes: temp file → fsync → atomic rename. Never "delete then record".
- No hardcoded `bin/python` — use venv Python locator from `platform/`.
- Paths in state files use forward slashes regardless of platform.

## Boundaries

- Target platforms: Linux + Windows. macOS is not actively tested but should work where possible.
- Do not embed `uv`, `git`, or Python as libraries. Keep them as external CLI dependencies.
- Never edit `*.template`, `state/*`, `.venv-prod/`, or `.venv-candidate/` by hand; use the existing ledger and sync flows.
- Do not bypass transaction, backup, or promote safety paths for prod-affecting changes.
- State file schema changes: append-only optional fields, no renames, read path tolerant of missing optional fields.
- No Unix-specific runtime dependencies. The binary must not shell out to bash, coreutils, or any Unix-only commands.
- Bundle cross-platform import is rejected in v1 (platform mismatch → error).

## Verification

- Run `cargo test` before considering work complete.
- Run `cargo test` on **both Linux and Windows** for cross-platform work.
- Run `bash tests/test_gov_cli.sh` on Linux to verify behavioral compatibility (legacy baseline only, not cross-platform oracle).
- For new commands, exercise one happy path and one failure path on both platforms.
- For dependency-mutation changes, verify real `uv lock` and `uv sync` behavior in a project layout.
- For remove/promote/undo changes, verify backup and restore behavior rather than relying on code inspection alone.

## Migration Status

Vertical slice migration — each slice delivers a complete command group:

- [ ] Slice 0: Scaffold + cross-platform infrastructure (`platform/`, `fs_support/`, `toml_support/`, `state_ledger/`, clients, `cli.rs` with stubs)
- [ ] Slice 1: `pin add` / `pin list` / `pin remove` / `undo` / `op list` / `op inspect`
- [ ] Slice 2: `install torch` / `install` / `status`
- [ ] Slice 3: `node add` / `node remove` / `tx run` / `tx inspect` / `tx abort` / `tx promote` / `resolve`
- [ ] Slice 4: `update run` / `update inspect` / `update abort` / `update promote` / `update resolve`
- [ ] Slice 5: `env export` / `env import`
- [ ] Slice 6: `run` / `stop` / `help` / `init`

## Read Only When Needed

Do not read every document listed below by default. Read only the document(s) that match the current task.

- If you are working on the rewrite plan, migration strategy, or cross-platform design, read `dev-docs/adr/003-rust-rewrite-plan.md`.
- If you need background on language selection rationale, read `dev-docs/adr/002-rewrite-language-selection.md`.
- If you need background on global pin management design, read `dev-docs/adr/001-global-pin-management.md`.
- For archived Bash-era design docs (subsystem specs, contracts, policies, key flows), see `dev-docs-old/`. These reflect the Bash implementation and are retained as reference, not as active design authority.
