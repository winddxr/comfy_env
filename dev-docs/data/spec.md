# Data Sovereignty And Integration Spec

## Metadata

- Status: Active
- Last Reviewed: 2026-03-01

## Source Of Truth By Domain

- Dependency truth:
  - `pyproject.toml`
  - `uv.lock`
- Plugin registry truth:
  - `state/plugins.json`
- Transaction truth:
  - `state/transactions/<txid>.json`
- Operation truth:
  - `state/ops/<op_id>/meta.json`
  - `state/ops/<op_id>/backup/*`
- Conflict truth:
  - `state/conflicts/<txid>.json`
- Runtime liveness hint:
  - `state/comfyui.pid`

## Ownership Rules

- Dependency truth is authoritative for what prod should contain.
- Plugin registry is authoritative for known plugin metadata, but not for actual dependency removal content.
- `managed_deps` is a cache field and can be rebuilt from dependency groups.
- Transaction and operation records are the audit trail; they should not be edited manually.

## Integration Boundaries

- `uv` reads and rewrites dependency truth and materializes envs from it.
- `git` touches plugin source trees only.
- ComfyUI runtime reads source trees and active environments but does not own governance records.

## Consistency Model

- There is no distributed consensus; consistency is local and sequential by convention.
- Workdirs are temporary staging copies used to prevent partially written root truth during lock generation.
- Operation backups are the recovery anchor when root truth mutation fails.

## Data Risks

- Concurrent local commands can race because the system has no lockfile protocol.
- Manual edits to local truth can invalidate undo assumptions and intentionally trigger the hash guard.
