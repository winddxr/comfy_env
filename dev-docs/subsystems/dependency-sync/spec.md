# Dependency Sync Spec

## Metadata

- Status: Active
- Last Reviewed: 2026-03-01

## Scope & External System Profile

This adapter translates governance intent into `uv` operations across local truth files and virtual environments. It owns:

- `uv lock --python ...`
- `uv lock --check --python ...`
- `uv sync --python ... --locked --exact --all-groups`
- `uv export --format pylock.toml --locked --all-groups`
- `uv add --group ...`
- `uv remove --group ...`
- `uv pip freeze`

The external system is the `uv` CLI plus the filesystem locations for `.venv-prod` and `.venv-candidate`.

## Data Mapping (Port/API/Event)

- Input ports:
  - root `pyproject.toml`
  - root `uv.lock`
  - workdir copies of those files
  - `runtime.python` as a canonical minor line
  - `project.requires-python`
  - `[tool.uv].environments`
  - dependency-group names (`core`, `torch`, plugin groups, `overrides`)
  - ComfyUI `requirements.txt`
  - torch index URL
  - promotion/update staged workdirs
  - environment paths returned by `prod_env_path()` and `candidate_root_path()`
- Output ports:
  - updated lock file
  - materialized prod/candidate environments
  - freeze snapshots for diffing
  - lock logs used for conflict analysis

## Error Translation (Infra -> Domain/Application)

- `uv lock` failure in promote/remove/update workdirs becomes:
  - conflict when a plan cannot solve
  - failed remove when dependency GC cannot lock
- `uv sync` failure in prod becomes:
  - restore-required destructive failure
- `uv sync` in candidate run becomes:
  - command failure before runtime execution proceeds

## Integration Behaviors / Key Flows

- `dependency-sync#KF-001` Initialize prod environment
  - Normalize the requested Python to a minor line, sync `pyproject.toml` runtime constraints, then root `uv lock --python` and exact sync into prod env.
- `dependency-sync#KF-002` Install managed torch group
  - Stage `dependency-groups.torch` via `uv add`, then sync prod.
- `dependency-sync#KF-003` Install managed core group
  - Read `requirements.txt`, rewrite `dependency-groups.core`, then sync prod.
- `dependency-sync#KF-004` Materialize plugin candidate observation env
  - Create candidate env path, exact sync, freeze before and after runtime execution.
- `dependency-sync#KF-005` Materialize core update candidate observation env
  - Build a staged workdir from `requirements.txt`, exact sync it into a candidate env, then freeze prod vs candidate.
- `dependency-sync#KF-006` Apply promotion/remove plan in workdir
  - Clone current truth into a workdir, mutate through `uv add/remove`, then `uv lock`.
- `dependency-sync#KF-007` Rebuild prod after guarded mutation
  - Exact sync into prod after new truth is copied back to root.
- `dependency-sync#KF-008` Export bundle lock payload
  - Export `pylock.toml` from the current locked truth without re-locking.
- `dependency-sync#KF-009` Verify and stage imported truth
  - Copy bundle truth into a staging workdir, validate bundle Python/platform compatibility, run lock check, then exact sync prod from staging truth.

## Runtime / Connectivity Constraints

- Requires `uv` to be installed and on `PATH`.
- Candidate and prod envs are local filesystem directories, not remote or shared runtimes.
- Python selection is explicit and driven by `runtime.python`, while lock breadth is constrained by `project.requires-python` and `[tool.uv].environments`.

## Schema / DDL

- Not applicable. The adapter owns command translation, not persistent schema.

## Code Anchors

| Doc ID | path | symbol | line |
|---|---|---|---|
| DS-001 | `bin/gov` | `configured_python` | 262 |
| DS-002 | `bin/gov` | `lock_project_exact` | 272 |
| DS-003 | `bin/gov` | `sync_project_env_exact` | 284 |
| DS-004 | `bin/gov` | `set_dependency_group_exact` | 530 |
| DS-005 | `bin/gov` | `apply_plan_in_workdir` | 1027 |
| DS-006 | `bin/gov` | `cmd_init` | 1765 |
| DS-007 | `bin/gov` | `cmd_install_torch` | 2484 |
| DS-008 | `bin/gov` | `cmd_install_core` | 2554 |
| DS-009 | `bin/gov` | `cmd_update_run` | 2644 |
| DS-010 | `bin/gov` | `cmd_tx_promote` | 2309 |
| DS-011 | `bin/gov` | `cmd_run` | 3201 |
