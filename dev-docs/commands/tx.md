# tx (Plugin Transactions)

## `tx run`

### Synopsis

```
gov tx run <node_id> [--timeout <seconds>]
```

### Purpose

Create a candidate transaction that observes the dependency impact of a plugin. Installs the plugin's requirements into a candidate environment and runs ComfyUI to verify compatibility.

### Arguments

| Flag | Required | Description |
|------|----------|-------------|
| `<node_id>` | Yes | Plugin identifier (must be registered) |
| `--timeout` | No | Override timeout in seconds (default: config tx.timeout_seconds) |

### Preconditions

- Plugin must exist in `plugins.json`
- Plugin source directory must exist (`custom_nodes/<node_id>/`)
- Plugin must have a `requirements.txt` in its source directory (`custom_nodes/<node_id>/requirements.txt`). If no requirements file exists, the plugin is treated as having no additional dependencies — the transaction proceeds with an empty dependency set for the plugin group.
- `config.toml` must exist
- No existing `running` transaction for this node (prevents duplicates)

### Reads

- `config.toml` — python, candidate_root, comfyui_dir, timeout
- `state/plugins.json` — plugin record (install path, group name)
- Plugin's `requirements.txt` — dependency specs
- `pyproject.toml`, `uv.lock` — current truth for staging

### Writes

- `state/transactions/<txid>.json` — new transaction record
- `.venv-candidate/<txid>/` — candidate environment
- `state/work/<workdir>/` — staged workdir for dependency resolution
- `state/logs/<txid>.stdout.log`, `state/logs/<txid>.stderr.log` — run output

### Success Path

```
1. Generate txid
2. Load plugin record from plugins.json
3. Read plugin requirements (from plugin source dir)
4. Create transaction record (status="running", kind="plugin")
5. Copy truth files to staged workdir
6. Add plugin requirements to workdir pyproject.toml:
   - Create/update dependency-groups.<group_name>
   - `uv add --group <group> --python <py> --no-sync <spec>` for each dep
7. Lock workdir: `uv lock --python <py>`
8. Create candidate env: sync workdir to .venv-candidate/<txid>/
9. Capture pre-freeze: `uv pip freeze` in candidate env
10. Run ComfyUI in candidate env:
    - Command: venv_python <comfyui_dir>/main.py
    - With timeout
    - Capture stdout/stderr to log files
    - Record exit code
11. Capture post-freeze: `uv pip freeze` in candidate env
12. Compute diff (added/removed packages between pre and post freeze)
13. Compute core_impact: diff ∩ policy.core_packages
14. Update transaction record:
    - status = "completed" (if exit 0) or "failed" (if non-zero/timeout)
    - pre_freeze, post_freeze, diff, core_impact, logs, run_exit_code
```

### Failure Path

```
IF plugin not found: exit with precondition error
IF lock fails during workdir staging:
  - Write conflict artifact via [Lock Conflict Protocol]
  - Update tx status → "needs_resolution"
  - Clean up candidate env
  - Return (not a fatal error — user resolves then re-promotes)
IF candidate sync fails: update tx status → "failed", clean up
IF ComfyUI run times out: record RunOutcome::TimedOut, status → "failed"
IF ComfyUI run exits non-zero: record exit code, status → "failed"
```

### State Transitions

- Creates: transaction `running` → `completed` | `failed` | `needs_resolution`
- `running` → `needs_resolution`: lock fails during workdir staging (before ComfyUI runs)
- `running` → `completed`: ComfyUI exits 0
- `running` → `failed`: ComfyUI exits non-zero or timeout

### Cleanup Ownership

- **Candidate env** (`.venv-candidate/<txid>/`): cleaned up by `tx abort` or `tx promote` (on success). Not cleaned on `tx run` failure — preserved for inspection.
- **Staged workdir** (`state/work/...`): cleaned up by `tx abort` or `tx promote`. Not cleaned on needs_resolution — needed for resolve retry.

### Platform Notes

- Candidate env Python located via `venv_python()`
- Timeout uses platform-native child process timeout (not Unix `timeout` command)
- Log capture uses stdout/stderr pipes (cross-platform)

---

## `tx inspect`

### Synopsis

```
gov tx inspect <txid>
```

### Purpose

Display the full details of a transaction record.

### Preconditions

- Transaction must exist in `state/transactions/`

### Reads

- `state/transactions/<txid>.json`
- Optionally: `state/conflicts/<txid>.json` (if status is needs_resolution)

### Writes

Nothing.

### Success Path

```
1. Load transaction record
2. Display: txid, kind, subject, status, timestamps
3. Display: diff (added/removed packages)
4. Display: core_impact (if non-empty)
5. Display: run logs summary (exit code, timeout status)
6. IF needs_resolution: display conflict summary and resolution hint
7. IF promoted: display promotion details (op_id, reason)
```

---

## `tx abort`

### Synopsis

```
gov tx abort <txid>
```

### Purpose

Cancel a transaction and clean up its candidate environment.

### Preconditions

- Transaction must exist
- Transaction status must be one of: `completed`, `failed`, `needs_resolution`, `resolved`
- Cannot abort an already `promoted` or `aborted` transaction

### Reads

- `state/transactions/<txid>.json`

### Writes

- `state/transactions/<txid>.json` — status updated to "aborted"
- `.venv-candidate/<txid>/` — deleted
- `state/work/<workdir>/` — deleted (if exists)

### Success Path

```
1. Load transaction record
2. Verify status is abortable
3. Delete candidate environment directory
4. Delete staged workdir (if exists)
5. Update transaction status → "aborted"
```

---

## `tx promote`

### Synopsis

```
gov tx promote <txid> [--approve-core --reason "<text>"] [--allow-failed-run]
```

### Purpose

Promote a completed transaction's dependency changes into production truth and sync the prod env.

### Arguments

| Flag | Required | Description |
|------|----------|-------------|
| `<txid>` | Yes | Transaction to promote |
| `--approve-core` | Conditional | Required if core_impact is non-empty |
| `--reason` | No | Documentation for core approval |
| `--allow-failed-run` | No | Allow promoting even if run status was "failed" |

### Preconditions

- Transaction must exist with status `completed` or `resolved`
- If status is `failed`: requires `--allow-failed-run`
- Core impact gate: if core_impact non-empty, requires `--approve-core`

### Reads

- `state/transactions/<txid>.json` — diff, core_impact, staged workdir path
- `config.toml` — python, prod env, core_packages
- `pyproject.toml`, `uv.lock` — current production truth

### Writes

- `pyproject.toml` — adds plugin dependency group entries
- `uv.lock` — re-locked with new dependencies
- `.venv-prod/` — synced
- `state/transactions/<txid>.json` — status → "promoted", promotion payload
- `state/ops/<op_id>/` — new operation record

### Success Path

```
1. Load transaction record
2. Verify status is promotable
3. IF run was failed AND no --allow-failed-run: exit with policy error
4. Execute [Core Impact Gate Protocol]
5. Generate promotion plan:
   - Classify additions as "direct" (from plugin requirements) or "override"
6. op_begin(kind="plugin_promote", reference=txid)
7. Copy truth to staged workdir
8. Apply promotion plan to workdir:
   - Add direct deps to plugin's dependency group
   - Add override deps to overrides group (if any resolution pins exist)
9. Lock workdir via [Staged Workdir Protocol]
10. IF lock fails:
    - Execute [Lock Conflict Protocol]
    - Update tx status → "needs_resolution"
    - op_restore → op_finalize(failed)
    - Return conflict state
11. Copy workdir truth → root (pyproject.toml, uv.lock)
12. Sync prod env via [Prod Sync Protocol]
13. Smoke test via [Smoke Test Protocol]
14. Update transaction: status → "promoted", promotion.op_id = op_id
15. Update plugins.json: set managed_deps for plugin
16. Clean up candidate env
17. op_finalize(success)
```

### Failure Path

```
IF policy gate blocks: exit before mutation
IF lock conflict: set needs_resolution, op_restore, return conflict
IF sync fails: op_restore → re-sync prod → op_finalize(failed)
    → tx status → "promote_failed"
IF smoke fails: op_restore → re-sync prod → op_finalize(failed)
    → tx status → "promote_failed"
```

### State Transitions

- Transaction: `completed`/`resolved` → `promoted` | `needs_resolution` | `promote_failed`
- Operation: created `running` → `success` | `failed`
