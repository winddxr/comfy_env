# update (Core Update Transactions)

**Implementation target:** [src/application/update.rs](../../src/application/update.rs)

## `update run`

### Synopsis

```
gov update run [--requirements-file <path>] [--timeout <seconds>]
```

### Purpose

Create a core update transaction that stages updated ComfyUI requirements, locks them in a candidate environment, and runs ComfyUI to verify compatibility.

### Arguments

| Flag | Required | Description |
|------|----------|-------------|
| `--requirements-file` | No | Path to requirements.txt (default: `<comfyui_dir>/requirements.txt`) |
| `--timeout` | No | Override timeout seconds |

### Preconditions

- `config.toml` must exist
- Torch group must be populated
- Requirements file must exist

### Reads

- `config.toml` — python, candidate_root, comfyui_dir, timeout
- Requirements file — new dependency specs
- `pyproject.toml`, `uv.lock` — current truth

### Writes

- `state/transactions/<txid>.json` — new transaction (kind="core_update")
- `.venv-candidate/<txid>/` — candidate environment
- `state/work/<workdir>/` — staged workdir
- `state/logs/<txid>.stdout.log`, `state/logs/<txid>.stderr.log`

### Success Path

```
1. Generate txid
2. Read requirements file
3. Filter out torch family packages
4. Record source: requirements path + sha256
5. Create transaction record (status="running", kind="core_update")
6. Copy truth to staged workdir
7. Replace dependency-groups.core in workdir with filtered requirements
8. Lock workdir: `uv lock --python <py>`
9. Create candidate env: sync workdir to .venv-candidate/<txid>/
10. Capture pre/post freeze
11. Run ComfyUI in candidate env (with timeout)
12. Compute diff and core_impact
13. Update transaction: status → "completed" | "failed"
```

### Failure Path

Same structure as `tx run`:
- Lock failure → "needs_resolution" + conflict artifact
- Sync failure → status "failed"
- Run timeout/error → status "failed"

---

## `update inspect`

### Synopsis

```
gov update inspect <txid>
```

Identical behavior to `tx inspect` — displays transaction details. Additionally shows:
- Source requirements file path and hash
- Staged workdir path

---

## `update resolve`

### Synopsis

```
gov update resolve <txid> [--pin <pkg==version>]... [--pins-file <path>]
```

### Purpose

Resolve a lock conflict in a core update transaction by providing additional pins.

### Arguments

| Flag | Required | Description |
|------|----------|-------------|
| `<txid>` | Yes | Transaction to resolve |
| `--pin` | No | One or more pin specs (repeatable) |
| `--pins-file` | No | File with one pin spec per line |

At least one of `--pin` or `--pins-file` must be provided.

### Preconditions

- Transaction must exist with status `needs_resolution`
- Transaction kind must be `core_update`
- At least one pin must be provided

### Reads

- `state/transactions/<txid>.json`
- `pyproject.toml`, `uv.lock` — current truth
- Pins file (if `--pins-file`)

### Writes

- `state/transactions/<txid>.json` — resolution_pins updated, status may change
- `state/work/<workdir>/` — re-staged with pins applied

### Success Path

```
1. Load transaction record
2. Verify status == "needs_resolution"
3. Collect pins from --pin flags and/or --pins-file
4. Merge new pins into transaction's resolution_pins (append, last-wins dedup)
5. Copy truth to staged workdir
6. Apply original core update + all resolution pins to workdir:
   - Replace core group with requirements
   - Add/update resolution pins in overrides group
7. Lock workdir: `uv lock --python <py>`
8. IF lock succeeds:
   - Update tx status → "resolved"
   - Update tx resolution_pins
9. IF lock fails:
   - Write updated conflict artifact
   - tx remains "needs_resolution"
   - Return conflict state
```

---

## `update abort`

### Synopsis

```
gov update abort <txid>
```

Same semantics as `tx abort`. Cancels core update transaction, cleans up candidate env and workdir.

---

## `update promote`

### Synopsis

```
gov update promote <txid> [--approve-core --reason "<text>"] [--allow-failed-run]
```

### Purpose

Promote a core update transaction's staged snapshot to production.

### Preconditions

Same as `tx promote` — status must be `completed` or `resolved`, core impact gate applies.

### Reads

- `state/transactions/<txid>.json`
- `config.toml`
- Staged workdir truth files

### Writes

- `pyproject.toml` — core group replaced with staged content
- `uv.lock` — from staged workdir
- `.venv-prod/` — synced
- `state/transactions/<txid>.json` — status → "promoted"
- `state/ops/<op_id>/` — new operation record

### Success Path

```
1. Load transaction, verify promotable
2. Execute [Core Impact Gate Protocol]
3. op_begin(kind="core_promote", reference=txid)
4. Copy staged workdir truth → root (pyproject.toml, uv.lock)
5. Sync prod env via [Prod Sync Protocol]
6. Smoke test via [Smoke Test Protocol]
7. Update tx status → "promoted"
8. Clean up candidate env and workdir
9. op_finalize(success)
```

### Failure Path

```
IF core gate blocks: exit before mutation
IF sync fails: op_restore → re-sync prod → tx "promote_failed" → op_finalize(failed)
IF smoke fails: op_restore → re-sync prod → tx "promote_failed" → op_finalize(failed)
```
