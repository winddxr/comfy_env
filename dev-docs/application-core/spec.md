# Application Core Spec

## Metadata

- Status: Active
- Last Reviewed: 2026-04-07

## Scope & Boundary

The application core is the single CLI entrypoint in `bin/gov`. It parses commands, validates user input, sequences subsystems, and decides when to persist state, block unsafe actions, or exit with errors. It now owns eight user-visible surfaces:

- S1 Environment bootstrap: `init`
- S2 Managed dependency install: `install`, `install torch`, `status`
- S3 Node lifecycle: `node add`, `node remove`
- S4 Plugin transaction execution: `tx run`, `tx inspect`, `tx abort`
- S5 Promotion and conflict handling: `tx promote`, `resolve`, `update promote`, `update resolve`
- S6 Core dependency update transactions: `update run`, `update inspect`, `update abort`
- S7 Environment handoff: `env export`, `env import`
- S8 Audit, reversal, and runtime control: `op list`, `op inspect`, `undo`, `run`, `stop`

It does not own `uv`, `git`, or ComfyUI semantics. Those remain adapter concerns.

## Domain Model

- Command Session: one CLI invocation routed by `main()`.
- Plugin Transaction Intent: a plugin-specific candidate observation and later promotion candidate.
- Core Update Transaction Intent: a staged `requirements.txt`-driven candidate snapshot for ComfyUI base dependencies.
- Environment Handoff Intent: export verified locked truth plus runtime source snapshots, or exact-restore them on another machine.
- Operation Intent: a backup-protected destructive mutation.
- Runtime Session: a foreground ComfyUI process launched from `.venv-prod`.

## Use-Case Catalog

- `UC-001` Manage plugin through transaction: add source, run candidate, inspect, promote, and optionally resolve conflicts.
- `UC-002` Remove plugin with reversible state: remove dependency group, resync prod, optionally purge code, preserve undoable state.
- `UC-003` Undo successful operation: validate hashes, restore backup, resync prod, mark prior op as undone.
- `UC-004` Initialize local governance state: create layout, write config, seed local `pyproject.toml`, lock, sync prod.
- `UC-005` Inspect local governance state: summarize config readiness, env readiness, and pending transactions.
- `UC-006` Run and stop production ComfyUI: optionally sync, exec foreground process, stop through PID file.
- `UC-007` Bootstrap local runtime prerequisites: persist `paths.comfyui_dir` and `runtime.python`.
- `UC-008` Install managed runtime dependencies: install torch first, then import ComfyUI `requirements.txt` into dependency truth.
- `UC-009` Transactional update of ComfyUI core requirements: stage new `requirements.txt`, observe candidate, resolve conflicts, promote, and allow undo.
- `UC-010` Export and import environment bundle: hand off locked truth, plugin registry, and runtime source snapshots through a directory bundle.

## Key Flows & Failure Recovery

- `core#KF-001` Bootstrap local state
  - Trigger: `cmd_init`.
  - Success: ensure layout, resolve `--python` selectors to a local interpreter when needed, normalize `runtime.python` to a canonical minor line, seed `pyproject.toml` if needed, sync `project.requires-python` plus `[tool.uv].environments`, `uv lock`, exact sync into prod env.
  - Failure: missing required init flags or missing tools exits before partial runtime state is considered valid.
- `core#KF-002` Install managed torch runtime
  - Trigger: `cmd_install_torch`.
  - Success: stage `dependency-groups.torch`, copy truth to root, sync prod, run torch import smoke test, record undoable op.
  - Failure: sync or smoke failure restores pre-op truth.
- `core#KF-003` Install managed core requirements
  - Trigger: `cmd_install_core`.
  - Success: read `requirements.txt`, stage `dependency-groups.core`, sync prod, smoke test, record undoable op.
  - Failure: sync or smoke failure restores pre-op truth.
- `core#KF-004` Register or remove plugin node
  - Trigger: `cmd_node_add`, `cmd_node_remove`.
  - Success: clone/register metadata, or remove group-backed dependencies and registry record.
  - Failure: remove path restores from op backup before returning.
- `core#KF-005` Record plugin transaction
  - Trigger: `cmd_tx_run`.
  - Success: materialize candidate env, freeze pre/post package sets, run ComfyUI, write plugin transaction JSON.
  - Failure: runtime failure still yields a persisted transaction with `status=failed`.
- `core#KF-006` Record core update transaction
  - Trigger: `cmd_update_run`.
  - Success: stage a workdir from `requirements.txt`, materialize candidate env, freeze prod vs candidate, run ComfyUI, write `kind=core_update` transaction JSON.
  - Failure: lock conflicts write a conflict report and `needs_resolution`; candidate sync or runtime failures still persist the transaction.
- `core#KF-007` Resolve or abort transaction
  - Trigger: `cmd_tx_abort`, `cmd_resolve`, `cmd_update_abort`, `cmd_update_resolve`.
  - Success: abort removes candidate artifacts; resolve merges pins and retries lock.
  - Failure: unresolved lock leaves the transaction in `needs_resolution` with a fresh conflict report.
- `core#KF-008` Promote guarded diff
  - Trigger: `cmd_tx_promote`, `cmd_update_promote`.
  - Success: validate status, enforce core-impact approval, create backup, sync prod, smoke test, finalize operation and transaction.
  - Failure: any lock, sync, or smoke failure restores pre-op truth and marks the transaction with explicit promote failure state.
- `core#KF-009` Start or stop runtime
  - Trigger: `cmd_run`, `cmd_stop`.
  - Success: optionally sync prod, write PID, `exec` ComfyUI, later stop via TERM then KILL fallback.
  - Failure: missing env, lock, entrypoint, or PID are explicit command errors.
- `core#KF-010` Hand off environment bundle
  - Trigger: `cmd_env_export`, `cmd_env_import`.
  - Success: export copies locked truth plus runtime `custom_nodes` snapshots into a verified directory bundle; import validates manifest/runtime compatibility, stages truth, exact-syncs prod, restores `custom_nodes`, updates target-local config, then smoke-tests.
  - Failure: export blocks on missing bundle inputs or source directories; import restores pre-op truth and affected `custom_nodes` before finalizing failure.

## Internal Components / Collaboration

- Command router: `main()` dispatches top-level verbs and subcommands.
- Command handlers: `cmd_*` functions implement each user-visible use case.
- State helpers: transaction and operation helper functions are called by command handlers but remain separate logical subsystem contracts.
- External adapters: handlers call `uv`, `git`, filesystem, timeout, and signal mechanisms through helper functions or direct commands.

## State & Lifecycle

- Plugin transactions follow: `running -> completed|failed -> promoted|needs_resolution|resolved|promote_failed`, with `aborted` as explicit termination.
- Core update transactions use the same status family but are distinguished by `kind=core_update`.
- Operations follow: `running -> success|failed -> undone` for the original op when an undo succeeds.
- Runtime PID lifecycle is `absent -> running -> stale|removed`.

## Error Boundary

- Domain/Application errors:
  - bad command shape, missing IDs, missing required init flags, invalid transaction kind/state, missing plugin metadata, missing staged workdir, missing PID
- Infrastructure errors:
  - `uv` non-zero exit, `git` clone failure, missing `main.py`, smoke failure, process timeout
- Translation rule:
  - infrastructure failures become command failure plus state mutation only when recovery is explicitly completed or a durable artifact is written

## Code Anchors

| Doc ID | path | symbol | line |
|---|---|---|---|
| UC-004 | `bin/gov` | `cmd_init` | 2763 |
| UC-008 | `bin/gov` | `cmd_install_torch` | 3444 |
| UC-008 | `bin/gov` | `cmd_install_core` | 3514 |
| UC-009 | `bin/gov` | `cmd_update_run` | 3592 |
| UC-009 | `bin/gov` | `cmd_update_promote` | 3907 |
| UC-005 | `bin/gov` | `cmd_status` | 4210 |
| UC-001 | `bin/gov` | `cmd_node_add` | 2813 |
| UC-002 | `bin/gov` | `cmd_node_remove` | 2906 |
| UC-001 | `bin/gov` | `cmd_tx_run` | 3008 |
| UC-001 | `bin/gov` | `cmd_tx_promote` | 3298 |
| UC-010 | `bin/gov` | `cmd_env_export` | 4037 |
| UC-010 | `bin/gov` | `cmd_env_import` | 4103 |
| UC-006 | `bin/gov` | `cmd_run` | 4311 |
| ROUTE-001 | `bin/gov` | `main` | 4462 |

## Internal Contracts

Contracts are split into [contracts.md](./contracts.md) because the command surface and state schema are consumed across multiple logical modules and evolve separately from the core narrative.
