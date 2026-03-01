# Dependency Sync Spec

## Metadata

- Status: Active
- Last Reviewed: 2026-03-01

## Scope & External System Profile

This adapter translates governance intent into `uv` operations across local truth files and virtual environments. It is the only logical module that should decide how to perform:

- `uv lock`
- `uv sync --locked --exact --all-groups`
- `uv add --group ...`
- `uv remove --group ...`
- `uv pip freeze`

The external system is the `uv` CLI plus the filesystem locations for `.venv-prod` and `.venv-candidate`.

## Data Mapping (Port/API/Event)

- Input ports:
  - root `pyproject.toml`
  - root `uv.lock`
  - workdir copies of those files
  - dependency-group names
  - promotion plan files (`direct_additions`, `override_additions`)
  - environment paths returned by `prod_env_path()` and `candidate_root_path()`
- Output ports:
  - updated lock file
  - materialized prod/candidate environments
  - freeze snapshots for diffing
  - lock logs used for conflict analysis

## Error Translation (Infra -> Domain/Application)

- `uv lock` failure in promote/remove workdirs becomes:
  - conflict when promotion plan cannot solve
  - failed remove when dependency GC cannot lock
- `uv sync` failure in prod becomes:
  - restore-required destructive failure
- `uv sync` in candidate run becomes:
  - command failure before runtime execution proceeds

## Integration Behaviors / Key Flows

- `dependency-sync#KF-001` Initialize prod environment
  - Root `uv lock`, then exact sync into prod env.
- `dependency-sync#KF-002` Materialize candidate observation env
  - Create candidate env path, exact sync, freeze before and after runtime execution.
- `dependency-sync#KF-003` Apply promotion/remove plan in workdir
  - Clone current truth into a workdir, mutate through `uv add/remove`, then `uv lock`.
- `dependency-sync#KF-004` Rebuild prod after guarded mutation
  - Exact sync into prod after new truth is copied back to root.

## Runtime / Connectivity Constraints

- Requires `uv` to be installed and on `PATH`.
- Candidate and prod envs are local filesystem directories, not remote or shared runtimes.
- Lock/sync cost is proportional to local resolver and package install behavior; the adapter does not promise low latency.

## Schema / DDL

- Not applicable. The adapter owns command translation, not persistent schema.

## Code Anchors

| Doc ID | path | symbol | line |
|---|---|---|---|
| DS-001 | `bin/gov` | `write_group_deps` | 361 |
| DS-002 | `bin/gov` | `collect_freeze_file` | 487 |
| DS-003 | `bin/gov` | `build_workdir_for_tx` | 660 |
| DS-004 | `bin/gov` | `apply_plan_in_workdir` | 672 |
| DS-005 | `bin/gov` | `cmd_init` | 1258 |
| DS-006 | `bin/gov` | `cmd_node_remove` | 1369 |
| DS-007 | `bin/gov` | `cmd_tx_run` | 1484 |
| DS-008 | `bin/gov` | `cmd_tx_promote` | 1772 |
| DS-009 | `bin/gov` | `cmd_run` | 1990 |
