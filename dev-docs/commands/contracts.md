# Command Behavioral Contracts

This directory defines the **language-agnostic behavioral specifications** for every `gov` command. These are product-level contracts — they describe *what* each command does, not *how* it's implemented.

## How to Use

- **Implementers**: These specs are the authority for Rust implementation. If the code disagrees with this doc, fix the code.
- **Reviewers**: Verify that implementations satisfy every step in the success and failure paths.
- **AI agents**: Read the relevant command spec before implementing or modifying a command.

## Shared Protocols

The following protocols are referenced by name in individual command specs. Each command doc will say e.g. "Execute [Staged Workdir Protocol]" — look here for the full definition.

---

### Staged Workdir Protocol

Used by commands that mutate `pyproject.toml` and need to verify lockability before committing.

```
1. Create temporary workdir (copy of project root truth files)
2. Perform mutation on workdir copies (add/remove deps, edit groups)
3. Run `uv lock --python <configured_python>` in workdir
4. IF lock fails → return lock_failure (caller decides: conflict artifact or restore)
5. IF lock succeeds → workdir is "staged" and ready for promotion to root
```

Workdir contains at minimum: `pyproject.toml`, `uv.lock` (output of lock step).

---

### Backup/Restore/Finalize Protocol

Used by all commands that mutate production truth files. Creates a recoverable checkpoint.

```
op_begin(kind, reference):
  1. Generate op_id (timestamp + random hex)
  2. Create backup directory: state/ops/<op_id>/backup/
  3. Compute pre-hashes: sha256(pyproject.toml), sha256(uv.lock), sha256(plugins.json)
  4. Copy current truth files to backup (with existence markers)
  5. If operation affects custom_nodes: backup affected directories
  6. Write state/ops/<op_id>/meta.json with status="running", pre_hashes, backup paths
  7. Return op_id

op_finalize(op_id, success):
  IF success:
    1. Compute post-hashes
    2. Update meta.json: status="success", post_hashes, undoable=true
  IF failure:
    1. Update meta.json: status="failed", undoable=false

op_restore(op_id):
  1. Restore truth files from backup (respecting existence markers)
  2. Re-sync prod env to match restored truth
  3. If custom_nodes were backed up: restore them
```

**Existence markers**: Track whether a file existed before backup. On restore, if file didn't exist originally, delete it rather than restoring an empty file.

---

### Undo Drift Guard Protocol

Used by `undo` to prevent restoring into a state that has been modified since the operation.

```
1. Load target operation's post_hashes from meta.json
2. Compute current hashes of truth files
3. IF current_hashes != post_hashes:
     Exit with error: "files have changed since operation; undo blocked"
4. IF match: proceed with restore
```

This prevents undo from silently overwriting manual edits or subsequent operations.

---

### Core Impact Gate Protocol

Used by `tx promote` and `update promote` to gate changes that affect core packages.

```
1. Load policy.core_packages from config.toml
2. Compute impact_set = normalize(diff.added ∪ diff.removed) ∩ core_packages
3. IF impact_set is empty: proceed without gate
4. IF impact_set is non-empty:
     IF --approve-core flag present: proceed (optionally log --reason)
     ELSE: exit with policy error listing affected packages
```

---

### Smoke Test Protocol

Used after prod sync to verify environment health.

```
1. Load smoke test config:
   - If tx.smoke_test.program is configured: use structured command
   - Else: default to { program: venv_python, args: ["-c", "import sys; print(sys.version)"] }
2. Resolve program path through venv Python locator if program == "python"
3. Execute with timeout (tx.timeout_seconds)
4. IF exit_code == 0: pass
5. IF exit_code != 0 OR timeout: fail
```

**Structured command model** (replaces Bash-era shell string):
```toml
[tx.smoke_test]
program = "python"
args = ["-c", "import sys; print(sys.version)"]
```

Platform note: `program = "python"` is resolved via `venv_python()` — never hardcoded path.

---

### Lock Conflict Protocol

Used when `uv lock` fails during promote or resolve flows.

```
1. Capture lock stderr output
2. Write conflict artifact: state/conflicts/<txid>.json
   - txid, node_id/subject, timestamp
   - raw_log path (state/conflicts/<txid>.lock.log)
   - detected_packages (extracted from lock error)
   - input_hint (suggested resolution format)
3. Update transaction status → "needs_resolution"
4. Return conflict state (not a fatal error — caller reports to user)
```

---

### Prod Sync Protocol

Used after truth files are updated to materialize changes into the production venv.

```
1. Determine prod env path from config (runtime.prod_env)
2. Set UV_PROJECT_ENVIRONMENT=<prod_env_path>
3. Run `uv sync --python <configured_python> --locked --exact --all-groups`
4. IF sync fails: return sync_failure (caller decides: restore or report)
```

---

## State Machines

### Transaction Lifecycle

```
                    ┌─────────────┐
                    │   running   │
                    └──────┬──────┘
                           │
              ┌────────────┴────────────┐
              ▼                          ▼
      ┌─────────────┐          ┌─────────────┐
      │  completed  │          │   failed    │
      └──────┬──────┘          └─────────────┘
             │                         │
    ┌────────┼────────┐                │
    ▼        ▼        ▼                ▼
promoted  needs_    aborted         aborted
          resolution
             │
             ▼
          resolved ──→ promoted | promote_failed
```

Valid transitions:
- `running` → `completed` | `failed`
- `completed` → `promoted` | `needs_resolution` | `aborted`
- `failed` → `aborted`
- `needs_resolution` → `resolved` | `aborted`
- `resolved` → `promoted` | `needs_resolution` | `promote_failed`

### Operation Lifecycle

```
running → success | failed
success → undone (via undo command)
```

---

## Shared Type Definitions

| Type | Format | Example |
|------|--------|---------|
| TxId | `<UTC_timestamp>-<8_hex_chars>` | `20260419T120000Z-a1b2c3d4` |
| OpId | `<UTC_timestamp>-op-<8_hex_chars>` | `20260419T120000Z-op-e5f6a7b8` |
| NodeId | Git repo basename or explicit `--id` | `comfyui-manager` |
| GroupName | Normalized: lowercase, hyphens only | `node-comfyui-manager` |
| PythonMinor | `<major>.<minor>` | `3.11` |

---

## Error Taxonomy

| Category | When | Effect |
|----------|------|--------|
| Usage error | Invalid arguments/flags | Exit before any I/O |
| Precondition error | Missing config, missing files, wrong state | Exit before mutation |
| Policy error | Core impact gate, undo not undoable | Exit before mutation |
| Adapter error | uv/git/python subprocess failure | Restore if post-backup, then exit |
| Restore error | Backup restoration fails | Log prominently, leave state for manual recovery |

---

## Cross-Platform Rules

### Venv Python Locator

| Platform | Path pattern |
|----------|-------------|
| Linux | `<venv_root>/bin/python` |
| Windows | `<venv_root>/Scripts/python.exe` |

Centralized in `venv_python(venv_root) → PathBuf`. Never hardcoded in command logic.

### Process Termination

Product semantics: graceful stop with timeout, then forced termination.
- Linux: SIGTERM → wait → SIGKILL
- Windows: platform-native graceful → wait → TerminateProcess

### Path Normalization

- Paths in state files use forward slashes regardless of platform
- Absolute path detection accounts for drive letters on Windows
- `--comfyui-dir` must be absolute (platform-appropriate check)

---

## Command Index

| Group | Commands | Spec |
|-------|----------|------|
| Bootstrap | `init` | [init.md](init.md) |
| Install | `install`, `install torch` | [install.md](install.md) |
| Pins | `pin add`, `pin list`, `pin remove` | [pin.md](pin.md) |
| Nodes | `node add`, `node remove` | [node.md](node.md) |
| Transactions | `tx run`, `tx inspect`, `tx abort`, `tx promote` | [tx.md](tx.md) |
| Conflict resolution | `resolve` | [resolve.md](resolve.md) |
| Updates | `update run`, `update inspect`, `update abort`, `update promote`, `update resolve` | [update.md](update.md) |
| Environment | `env export`, `env import` | [env.md](env.md) |
| Operations | `op list`, `op inspect`, `undo` | [ops.md](ops.md) |
| Runtime | `run`, `stop` | [runtime.md](runtime.md) |
| Status | `status`, `help` | [status.md](status.md) |
