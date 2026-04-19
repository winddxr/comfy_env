# application/ (Command Orchestration Layer)

**Implementation targets:** [src/application/](../../src/application/), [src/cli.rs](../../src/cli.rs), [src/main.rs](../../src/main.rs)

## Responsibility

The `application/` module contains one file per command group (`pin.rs`, `tx.rs`, `node.rs`, etc.). Each file implements command handlers that **orchestrate** calls to infrastructure modules. Application code does not own business rules — it sequences module calls and manages control flow.

## What Belongs Here

- Argument parsing results → module calls
- Sequencing: precondition checks → backup → mutation → sync → smoke → finalize
- Control flow decisions: which error triggers restore vs. which is a clean exit
- User-facing output (success messages, error messages, formatted displays)

## What Does NOT Belong Here

- TOML editing logic → `toml_support/`
- File hashing, atomic writes → `fs_support/`
- uv/git subprocess invocation → `dependency_sync/`, `source_integration/`
- Backup/restore/drift-guard mechanics → `safety_guards/`
- JSON state CRUD → `state_ledger/`
- Platform differences → `platform/`
- Process spawning/termination → `runtime_executor/`

**Rule**: If you're writing logic that could be reused by another command, it belongs in a module, not in application code.

## Standard Command Handler Pattern

Every mutating command follows this skeleton:

```rust
fn cmd_pin_add(args: PinAddArgs, config: &RuntimeConfig) -> Result<()> {
    // 1. VALIDATE — preconditions, no I/O side effects
    validate_pin_specs(&args.specs)?;
    reject_torch_family(&args.specs)?;

    // 2. GATE — policy checks that block before any mutation
    // (not all commands have gates; promote commands check core impact here)

    // 3. BACKUP — create operation checkpoint
    let op_id = safety_guards::op_begin(OpKind::PinAdd, &reference, &root)?;

    // 4. MUTATE — perform the change in a staged workdir
    let workdir = dependency_sync::create_staged_workdir(&root, &work_base)?;
    toml_support::rewrite_dependency_group(&workdir.pyproject(), ...)?;
    let lock_result = dependency_sync::lock(&workdir)?;

    // 5. HANDLE LOCK FAILURE — restore or create conflict
    if !lock_result.success() {
        safety_guards::op_restore(&op_id, &root)?;
        dependency_sync::sync_prod(&root, config)?;
        safety_guards::op_finalize(&op_id, false)?;
        return Err(...)
    }

    // 6. PROMOTE — move staged truth to root
    dependency_sync::promote_workdir(&workdir, &root)?;

    // 7. SYNC — materialize into prod venv
    dependency_sync::sync_prod(&root, config)?;

    // 8. VERIFY — smoke test
    let smoke = safety_guards::run_smoke_test(&prod_env, config)?;
    if smoke.failed() {
        safety_guards::op_restore(&op_id, &root)?;
        dependency_sync::sync_prod(&root, config)?;
        safety_guards::op_finalize(&op_id, false)?;
        return Err(...)
    }

    // 9. FINALIZE — record success
    safety_guards::op_finalize(&op_id, true)?;
    Ok(())
}
```

### Variations by command type:

| Command type | Has gate? | Has backup? | Has staged workdir? | Has smoke test? |
|-------------|-----------|-------------|--------------------|-----------------| 
| `pin add/remove` | No | Yes | Yes | Yes |
| `install torch/install` | No | Yes | No (direct uv add) | Yes |
| `node add` | No | No | No | No |
| `node remove` | No | Yes | Yes | Yes |
| `tx run` | No | No | Yes (candidate, not prod) | No (run IS the test) |
| `tx promote` | Core gate | Yes | Yes | Yes |
| `update promote` | Core gate | Yes | No (uses staged snapshot) | Yes |
| `env import` | Platform check | Yes | No (overwrites from bundle) | Yes |
| `undo` | Drift guard | Yes | No (restores from backup) | No |
| `run/stop` | No | No | No | No |
| Read-only (`status`, `inspect`, `list`) | No | No | No | No |

### Non-Mutating Command Patterns

Not all commands follow the mutating skeleton. Three other patterns exist:

**Read-only pattern** (`status`, `op list`, `op inspect`, `tx inspect`, `pin list`, `help`):
```rust
fn cmd_status(config: &RuntimeConfig) -> Result<()> {
    let data = state_ledger::load_relevant_state()?;
    format_and_print(data);
    Ok(())
}
```
No backup, no restore, no sync. Just read state files and format output.

**Candidate transaction pattern** (`tx run`, `update run`):
```rust
fn cmd_tx_run(args: TxRunArgs, config: &RuntimeConfig) -> Result<()> {
    // 1. VALIDATE
    // 2. CREATE transaction record (status=running)
    // 3. STAGE workdir (copy truth, add deps, lock)
    // 4. SYNC to candidate env (not prod)
    // 5. RUN ComfyUI in candidate env with timeout
    // 6. CAPTURE freeze/diff/core_impact
    // 7. UPDATE transaction (status=completed|failed|needs_resolution)
    // No backup, no prod mutation — candidate env is disposable
}
```
Key difference: no `op_begin`, no prod sync, no smoke test. The run itself is the observation.

**Export pattern** (`env export`):
```rust
fn cmd_env_export(args: ExportArgs, config: &RuntimeConfig) -> Result<()> {
    // 1. VALIDATE (truth files exist, plugins have source dirs)
    // 2. CREATE output directory
    // 3. COPY truth files + registry + snapshots
    // 4. WRITE manifest with checksums
    // No backup needed — export is read-only from project perspective
}
```

## Restore Sequence

When any step after `op_begin` fails, the command handler is responsible for:

```rust
// Standard restore sequence — ALWAYS this order
safety_guards::op_restore(&op_id, &root)?;   // 1. restore files
dependency_sync::sync_prod(&root, config)?;    // 2. re-sync venv
safety_guards::op_finalize(&op_id, false)?;    // 3. mark op failed
```

This is NOT done inside `safety_guards` — it's the command handler's job. This avoids circular dependencies between modules.

## Unimplemented Commands

During migration, commands not yet implemented in Rust must return:

```
error: command '<name>' is not yet implemented
```

Exit code: non-zero (conventionally 2 for usage/not-implemented, 1 for runtime errors).

## Exit Codes

| Code | Meaning |
|------|---------|
| 0 | Success |
| 1 | Runtime error (adapter failure, restore failure, mutation failure) |
| 2 | Usage error or unimplemented command |

## Output Conventions

- Success: brief confirmation to stdout (e.g., "pin added: numpy==1.26.4")
- Errors: prefixed with `error:` to stderr
- Warnings: prefixed with `warning:` to stderr  
- Structured data (inspect, list, status): formatted to stdout
- No output on success for read-only commands that produce data (the data IS the output)

## Dependencies

All modules. The application layer is the top of the call graph — it calls into everything but nothing calls into it (except `cli.rs` which dispatches to command handlers).

## File Layout

```
src/application/
├── mod.rs          — re-exports
├── init.rs         — gov init
├── install.rs      — gov install, gov install torch
├── pin.rs          — gov pin add/list/remove
├── node.rs         — gov node add/remove
├── tx.rs           — gov tx run/inspect/abort/promote
├── update.rs       — gov update run/inspect/abort/promote/resolve
├── resolve.rs      — gov resolve
├── env.rs          — gov env export/import
├── ops.rs          — gov op list/inspect
├── undo.rs         — gov undo
├── runtime.rs      — gov run/stop
└── status.rs       — gov status/help
```
