# Architecture Overview

## System Identity

`comfy_env` (`gov`) is a sidecar governance CLI that controls how ComfyUI custom-node dependency changes are observed, promoted, rolled back, and run.

- **Language**: Rust, single binary, single crate
- **Platforms**: Linux + Windows
- **External dependencies**: `uv`, `git`, Python (all via subprocess, never embedded)

## Invariants

1. All production dependency mutations are anchored in local truth files (`pyproject.toml`, `uv.lock`) first, then applied to `.venv-prod`.
2. A transaction is the only supported observation unit for plugin dependency impact before promotion.
3. Destructive state changes (`promote`, `remove`, `undo`) must create or use operation backups before finalizing.
4. Lock conflicts must surface as explicit conflict artifacts and a resolvable transaction state, not silent partial success.
5. Core package drift is policy-gated and cannot promote without explicit approval.
6. `state/plugins.json` is registry metadata; dependency-group content in `pyproject.toml` is authoritative for actual dependencies.
7. `env export` / `env import` bundles are transport artifacts; after import, local truth remains `pyproject.toml + uv.lock`.
8. Bundle source snapshots exclude VCS admin metadata (`.git/`).
9. The Rust binary is the sole entry point. No shell wrappers or dispatchers in the product.
10. Cross-platform behavior is defined at the product semantics level, not left as implementation detail.

## Module Map

```
src/
├── main.rs                 → binary entry, panic handler
├── cli.rs                  → clap definitions, command dispatch
├── application/            → command implementations (one file per command group)
├── domain/                 → newtypes, enums, shared domain records
├── state_ledger/           → CRUD for transactions, operations, plugins, conflicts
├── safety_guards/          → backup/restore, drift guard, core impact gate
├── dependency_sync/        → UvClient, staged workdir, freeze, lock check
├── source_integration/     → GitClient, plugin path mapping
├── runtime_executor/       → process execution, PID tracking, log capture
├── platform/               → venv Python locator, process termination, PID check
├── toml_support/           → format-preserving config.toml and pyproject.toml editing
└── fs_support/             → atomic write, hashing, path normalization, dir operations
```

## Data Sovereignty

| Domain | Source of Truth | Location |
|--------|----------------|----------|
| Dependencies | `pyproject.toml` + `uv.lock` | project root |
| Configuration | `config.toml` | project root |
| Plugin registry | `state/plugins.json` | state dir |
| Transactions | `state/transactions/*.json` | state dir |
| Operations | `state/ops/<op_id>/meta.json` + `backup/` | state dir |
| Conflicts | `state/conflicts/*.json` | state dir |
| Runtime liveness hint | `state/comfyui.pid` | state dir |
| Logs | `state/logs/*` | state dir |

`state/comfyui.pid` is an operational hint, not a durable source of truth; runtime liveness must always be confirmed against the host process layer.

## External Tool Contracts

| Tool | Used For | Invoked Via |
|------|----------|-------------|
| `uv` | lock, sync, add, remove, pip freeze, export, python find | `UvClient` |
| `git` | clone, checkout | `GitClient` |
| Python | smoke tests, ComfyUI runtime | `PythonClient`, `RuntimeClient` |

All tool invocations go through client structs that return `CmdResult`. No raw `Command::new()` in application code.

## Cross-Platform Rules

See ADR-003 §Platform Abstraction for full details. Summary:

- Venv Python: `<venv>/bin/python` (Linux) vs `<venv>/Scripts/python.exe` (Windows)
- Process termination: graceful stop with timeout, then forced termination; Linux uses SIGTERM→SIGKILL, Windows uses platform-native graceful/forced primitives
- Paths in state files: always forward slashes
- Atomic writes: rename-based on both platforms, platform-specific fsync
- Bundle import: cross-platform import rejected in v1
- Smoke test: structured command (`program` + `args`), not shell string

## Key Flows

1. **Plugin onboarding**: `node add` → `tx run` → `tx inspect` → `tx promote`
2. **Lock conflict resolution**: promote fails → `resolve` with pins → re-promote
3. **Plugin removal + undo**: `node remove` → `undo` with backup restore
4. **Bootstrap**: `init` → `install torch` → `install` → `update run` → `update promote`
5. **Environment transfer**: `env export` → `env import` (same platform only in v1)

## Command Behavioral Contracts

For detailed step-by-step specifications of each command (preconditions, mutations, failure paths, state transitions), see [commands/contracts.md](commands/contracts.md).
