# Application Core Spec

## Metadata

- Status: Active
- Last Reviewed: 2026-03-01

## Scope & Boundary

The application core is the single CLI entrypoint in `bin/gov`. It parses commands, validates user input, sequences subsystems, and decides when to persist state, block unsafe actions, or exit with errors. It owns the surface decomposition S1-S6:

- S1 Environment and state: `init`, `status`
- S2 Node lifecycle: `node add`, `node remove`
- S3 Transaction execution: `tx run`, `tx inspect`, `tx abort`
- S4 Promotion and conflict handling: `tx promote`, `resolve`
- S5 Audit and reversal: `op list`, `op inspect`, `undo`
- S6 Runtime orchestration entrypoint: `run`, `stop`

It does not own `uv`, `git`, or ComfyUI semantics. Those remain adapter concerns.

## Domain Model

- Command Session: one CLI invocation routed by `main()`.
- Use Case: a single command family plus its guarded recovery path.
- Transaction Intent: a plugin-specific candidate observation and later promotion candidate.
- Operation Intent: a backup-protected destructive mutation.
- Runtime Session: a foreground ComfyUI process launched from `.venv-prod`.

## Use-Case Catalog

- `UC-001` Manage plugin through transaction: add source, run candidate, inspect, promote, and optionally resolve conflicts.
- `UC-002` Remove plugin with reversible state: remove dependency group, resync prod, optionally purge code, preserve undoable state.
- `UC-003` Undo successful operation: validate hashes, restore backup, resync prod, mark prior op as undone.
- `UC-004` Initialize local governance state: create layout, seed local `pyproject.toml`, lock, sync prod.
- `UC-005` Inspect local governance state: summarize env existence and pending transactions.
- `UC-006` Run and stop production ComfyUI: optionally sync, exec foreground process, stop through PID file.

## Key Flows & Failure Recovery

- `core#KF-001` Bootstrap local state
  - Trigger: `cmd_init`.
  - Success: ensure layout, seed `pyproject.toml` if needed, `uv lock`, exact sync into prod env.
  - Failure: missing `uv` or templates exits before partial runtime state is considered valid.
- `core#KF-002` Register or remove plugin node
  - Trigger: `cmd_node_add`, `cmd_node_remove`.
  - Success: clone/register metadata, or remove group-backed dependencies and registry record.
  - Failure: remove path restores from op backup before returning.
- `core#KF-003` Record transaction
  - Trigger: `cmd_tx_run`.
  - Success: materialize candidate env, freeze pre/post package sets, run ComfyUI, write transaction JSON.
  - Failure: runtime failure still yields a persisted transaction with `status=failed`.
- `core#KF-004` Resolve or abort transaction
  - Trigger: `cmd_tx_abort`, `cmd_resolve`.
  - Success: abort removes candidate env and marks `aborted`; resolve merges pins and retries lock.
  - Failure: unresolved lock leaves transaction in `needs_resolution` with a fresh conflict report.
- `core#KF-005` Promote guarded diff
  - Trigger: `cmd_tx_promote`.
  - Success: validate status, enforce core-impact approval, create backup, apply plan in workdir, sync prod, smoke test, finalize operation and transaction.
  - Failure: any lock, sync, or smoke failure restores pre-op truth and marks the transaction with explicit promote failure state.
- `core#KF-006` Start or stop runtime
  - Trigger: `cmd_run`, `cmd_stop`.
  - Success: optionally sync prod, write PID, `exec` ComfyUI, later stop via TERM then KILL fallback.
  - Failure: missing env, lock, entrypoint, or PID are explicit command errors.

## Internal Components / Collaboration

- Command router: `main()` dispatches top-level verbs and subcommands.
- Command handlers: `cmd_*` functions implement each user-visible use case.
- State helpers: transaction and operation helper functions are called by command handlers but remain separate logical subsystem contracts.
- External adapters: handlers call `uv`, `git`, filesystem, timeout, and signal mechanisms through helper functions or direct commands.

## State & Lifecycle

- Command sessions are ephemeral and end with process exit.
- Transactions follow the ledger lifecycle: `running -> completed|failed -> promoted|needs_resolution|resolved|promote_failed`, with `aborted` as explicit termination.
- Operations follow: `running -> success|failed -> undone` for the original op when an undo succeeds.
- Runtime PID lifecycle is `absent -> running -> stale|removed`.

## Error Boundary

- Domain/Application errors:
  - bad command shape, missing IDs, invalid transaction state, missing plugin metadata, missing PID
- Infrastructure errors:
  - `uv` non-zero exit, `git` clone failure, missing `main.py`, smoke failure, process timeout
- Translation rule:
  - infrastructure failures become command failure plus state mutation only when recovery is explicitly completed or a durable artifact is written

## Dependencies

- Allowed:
  - `bin/gov` shell logic
  - State Ledger helpers
  - Safety Guard helpers
  - `uv`, `git`, shell utilities, Python snippets
- Forbidden:
  - Assuming plugin source tree content is authoritative dependency truth
  - Skipping backup/restore on destructive flows

## Code Anchors

| Doc ID | path | symbol | line |
|---|---|---|---|
| UC-004 | `bin/gov` | `cmd_init` | 1258 |
| UC-005 | `bin/gov` | `cmd_status` | 1944 |
| UC-001 | `bin/gov` | `cmd_node_add` | 1276 |
| UC-002 | `bin/gov` | `cmd_node_remove` | 1369 |
| UC-001 | `bin/gov` | `cmd_tx_run` | 1484 |
| UC-001 | `bin/gov` | `cmd_tx_promote` | 1772 |
| UC-001 | `bin/gov` | `cmd_resolve` | 1634 |
| UC-003 | `bin/gov` | `cmd_undo` | 1079 |
| UC-006 | `bin/gov` | `cmd_run` | 1990 |
| UC-006 | `bin/gov` | `cmd_stop` | 2068 |
| ROUTE-001 | `bin/gov` | `main` | 2131 |

## Internal Contracts

Contracts are split into [contracts.md](./contracts.md) because the command surface and state schema are consumed across multiple logical modules and evolve separately from the core narrative.
