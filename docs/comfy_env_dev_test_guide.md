# Comfy Env Development & Testing Guide

> This document is the standalone execution guide for future development and validation of `comfy_env`.
> Implementation baseline: `comfy_env/bin/gov`, `comfy_env/config.toml`, `comfy_env/pyproject.toml`.

## 1. Goals and Scope

This guide defines:

1. Architecture and file/data contracts.
2. Command-level behavior and failure semantics.
3. Development workflow for changing `gov` safely.
4. Full testing strategy (functional, failure, rollback, regression).

Out of scope:

1. Modifying ComfyUI source code.
2. Managing non-Python system dependencies (CUDA drivers, OS packages).

## 2. Runtime Requirements

Target runtime for acceptance:

1. Linux with `bash`, `uv`, `git`, `python3` available.
2. ComfyUI path exists and contains `main.py`.
3. `comfy_env` directory writable.

Optional:

1. `timeout` command for transaction runtime limit.
2. Network access for dependency resolution.

## 3. Directory and File Contracts

## 3.1 Core layout

```text
comfy_env/
├── bin/
│   └── gov
├── config.toml
├── pyproject.toml
├── uv.lock
├── .venv-prod/
├── .venv-candidate/
├── snapshots/
└── state/
    ├── plugins.json
    ├── snapshots_index.json
    ├── transactions/
    ├── conflicts/
    ├── logs/
    └── work/
```

## 3.2 Truth files

1. `pyproject.toml`: declaration truth.
2. `uv.lock`: lock truth.
3. `state/plugins.json`: plugin registry truth.

Rollback restores exactly these three (not plugin code tree).

## 3.3 Snapshot contract

Each snapshot directory must contain:

1. `pyproject.toml`
2. `uv.lock`
3. `plugins.json`
4. `meta.json` with `id`, `created_at`, `reason`, `ref`

Index file: `state/snapshots_index.json`.

## 3.4 Transaction contract

`state/transactions/<txid>.json` contains at minimum:

1. identity: `txid`, `node_id`, `started_at`, `ended_at`
2. env: `candidate_env`
3. status: `running|completed|failed|aborted|needs_resolution|resolved|promoted|promote_failed`
4. diff: `pre_freeze`, `post_freeze`, `diff.added`, `diff.removed`, `core_impact`
5. logs: `stdout`, `stderr`, `run_exit_code`
6. promotion fields: `conflict_report`, `resolution_pins`, `promotion_plan`, `promotion`

## 4. Command Behavior Specification

## 4.1 `gov init`

1. Ensures layout files/directories exist.
2. Executes `uv lock`.
3. Creates/syncs prod env with `uv sync --locked --exact` targeting `.venv-prod`.

Failure semantics:

1. Missing `uv` => hard fail.
2. Lock or sync failure => hard fail, no partial success reported.

## 4.2 `gov node add`

1. Clones node repo into `<comfyui_dir>/custom_nodes/<node_id>`.
2. Registers node in `plugins.json` with `group=node-<id>`, `managed_deps=[]`.

Failure semantics:

1. Existing target dir => fail.
2. Clone failure => fail, registry not updated.

## 4.3 `gov tx run`

1. Creates candidate env `.venv-candidate/<txid>` from current truth.
2. Runs ComfyUI entry (`main.py`) using candidate python.
3. Captures pre/post freeze and writes tx JSON.
4. Sets status `completed` or `failed` based on run exit code.

Failure semantics:

1. Missing node record => fail.
2. Missing `main.py` => run_exit_code=2, tx status `failed`.

## 4.4 `gov resolve`

1. Only for tx status in `needs_resolution|promote_failed|resolved`.
2. Reads existing conflict report summary (if present).
3. Accepts interactive pins `pkg==version`.
4. Merges pins into tx `resolution_pins`.
5. Rebuilds working plan and retries lock in isolated workdir.
6. On success => tx status `resolved`; on failure => `needs_resolution` and new conflict report.

Pin policy:

1. Pins are fed through override path (`dependency-groups.overrides`) during resolution attempt.

## 4.5 `gov tx promote`

1. Valid statuses: `completed|resolved`; `failed` allowed with `--allow-failed-run`.
2. If `core_impact` non-empty, require `--approve-core`.
3. Creates pre-promote snapshot.
4. Builds promotion plan from tx diff + plugin requirements hints.
5. Applies plan in isolated workdir (`uv add` node group + overrides, then `uv lock`).
6. On lock success, atomically copies workdir truth files to root.
7. Rebuilds prod via `uv sync --locked --exact`.
8. Runs smoke test:
   1. configured `tx.smoke_test_cmd`, or
   2. default `python -c "import sys; print(sys.version)"`.
9. On success => post-promote snapshot + tx status `promoted` + plugin metadata update.
10. On failure => restore pre snapshot + tx status `promote_failed`.

## 4.6 `gov node remove`

1. Creates pre-remove snapshot.
2. Reads node `managed_deps` and removes from node group in workdir.
3. Locks and promotes updated truth.
4. Rebuilds prod with exact sync for GC.
5. Removes plugin record from `plugins.json`.
6. Optional `--purge-code` removes plugin directory from custom_nodes.

## 4.7 `gov rollback`

1. Creates pre-rollback snapshot.
2. Restores target snapshot truth files.
3. Rebuilds prod env.
4. On rebuild failure, restores pre-rollback snapshot and retries prod sync.

## 4.8 `gov snapshot list|inspect`

1. `list`: prints ordered index entries.
2. `inspect`: prints snapshot `meta.json`.

Snapshot retention:

1. Automatically prunes to `rollback.snapshot_retention` (default 50).

## 5. Development Workflow (for future contributors)

## 5.1 Safe modification rules

1. Never mutate root truth files directly inside new features; use workdir + promote pattern.
2. Always write tx status transitions explicitly.
3. Every destructive operation (`promote`, `remove`, `rollback`) must have a snapshot checkpoint before mutation.
4. Keep rollback scope stable: dependency truth + plugin registry only.

## 5.2 When adding a new command

1. Define CLI syntax in help block first.
2. Add input validation and explicit usage errors.
3. Add state/file side effects section in this guide.
4. Add at least one happy-path and one failure-path test case.

## 5.3 Data compatibility policy

1. New keys in tx/plugins JSON must be additive.
2. Existing keys should not be renamed without migration script.
3. If migration is needed, create `gov migrate` (future) rather than silent rewrite.

## 6. Testing Strategy

## 6.1 Test levels

1. Static checks:
   1. function presence, command routing, expected files.
2. Integration checks:
   1. command-level behavior with real `uv` and temp node repos.
3. End-to-end checks:
   1. candidate run -> resolve/promote -> remove -> rollback chain.

## 6.2 Test environment bootstrap (recommended)

1. Prepare a minimal fake ComfyUI:

```bash
mkdir -p /tmp/comfyui/custom_nodes
cat >/tmp/comfyui/main.py <<'PY'
print("fake comfyui start")
PY
```

2. Configure path:

```bash
# edit comfy_env/config.toml
[paths]
comfyui_dir = "/tmp/comfyui"
```

3. Initialize:

```bash
cd comfy_env
./bin/gov init
```

## 6.3 Acceptance test matrix

## A. Batch-1 regression

1. `gov init` creates `.venv-prod` and `uv.lock`.
2. `gov tx run` creates tx JSON with required fields.
3. `gov tx inspect` returns stable summary.
4. `gov tx abort` removes candidate env and marks status.

## B. Batch-2 promote/resolve

1. Promote success:
   1. prepare tx in `completed` state with non-core changes.
   2. run `gov tx promote <txid>`.
   3. expect status `promoted`, pre/post snapshots created.
2. Promote conflict:
   1. craft conflicting deps in plan inputs.
   2. run promote.
   3. expect status `needs_resolution` and `state/conflicts/<txid>.json` exists.
3. Resolve retry:
   1. run `gov resolve <txid>` and input valid pins.
   2. expect status `resolved` on successful lock.
4. Core gate:
   1. tx has `core_impact`.
   2. promote without `--approve-core` must fail.

## C. Batch-3 remove/rollback

1. Node remove GC:
   1. plugin has `managed_deps`.
   2. run `gov node remove <node_id>`.
   3. expect pre-remove snapshot and prod exact sync success.
2. Rollback:
   1. run `gov rollback <snapshot_id>`.
   2. expect truth files restored and prod rebuilt.
3. Snapshot retention:
   1. create >50 snapshots via repeated operations.
   2. expect index and directories retain latest 50 only.

## D. Failure recovery

1. Force prod sync failure during promote:
   1. break lock compatibility intentionally.
   2. expect auto-restore to pre-promote snapshot.
2. Force rollback sync failure:
   1. make restore snapshot invalid.
   2. expect auto-restore to pre-rollback snapshot.

## 6.4 Minimal CI command set (Linux)

```bash
set -e
cd comfy_env
./bin/gov init
./bin/gov status
# add scripted integration tests here as shell scenarios
```

## 7. Troubleshooting Runbook

## 7.1 `python3/python is required`

1. Install Python 3.10+.
2. Ensure `python3` in PATH.

## 7.2 `command not found: uv`

1. Install uv.
2. Ensure shell PATH contains uv binary.

## 7.3 `transaction not found`

1. Verify txid in `state/transactions/`.
2. Use `gov status` and `gov tx inspect` for discovery.

## 7.4 promote repeatedly enters `needs_resolution`

1. Open conflict report in `state/conflicts/<txid>.json`.
2. Run `gov resolve <txid>` and pin exact versions.
3. Retry `gov tx promote`.

## 7.5 rollback fails after restore

1. Check network/index availability for dependency fetch.
2. Inspect lock integrity in snapshot.
3. Re-run rollback after environment/network stabilization.

## 8. Known Limitations

1. `gov run` remains a placeholder (startup orchestration not finalized).
2. Promotion plan uses heuristics (`requirements*.txt` hints) and tx diff; not a full semantic dependency classifier.
3. Conflict report parser is lightweight; it summarizes raw lock logs but does not guarantee precise root-cause extraction.
4. Windows host validation is limited; acceptance target remains Linux.

## 9. Release Checklist

1. Docs updated:
   1. this guide
   2. `comfy_env_plan_v3.md`
   3. `comfy_env/README.md`
2. Command help block reflects actual behavior.
3. All acceptance matrix scenarios executed on Linux.
4. Snapshot prune verified.
5. Rollback dry-run verified.

## 10. Future Work (non-blocking)

1. Add machine-readable `gov status --json`.
2. Add `gov test` builtin for scripted acceptance checks.
3. Add schema validation for tx/plugins/snapshot JSON files.
4. Add optional plugin code snapshot mode for full-tree rollback.
