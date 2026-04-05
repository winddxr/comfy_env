# Source Integration Spec

## Metadata

- Status: Active
- Last Reviewed: 2026-03-01

## Scope & External System Profile

This adapter manages plugin source acquisition and path translation between registry metadata and actual filesystem paths under `ComfyUI/custom_nodes`.

The external systems are:

- remote Git repositories
- the local `git` CLI
- the ComfyUI source tree referenced by `paths.comfyui_dir`

## Data Mapping (Port/API/Event)

- Input ports:
  - `git_url`
  - optional `ref`
  - `node_id`
  - plugin `install_relpath`
  - configured `comfyui_dir`
- Output ports:
  - a cloned working tree at `custom_nodes/<node_id>`
  - optional checked-out ref
  - absolute path resolution for remove/promote flows

## Error Translation (Infra -> Domain/Application)

- existing target path becomes a user-visible "node target already exists" error
- failed clone/checkout becomes command failure before registry write
- missing plugin metadata during later flows becomes "plugin metadata missing" and blocks the caller

## Integration Behaviors / Key Flows

- `source-integration#KF-001` Clone and register source
  - clone the repository into `custom_nodes`, optionally checkout a ref, then let the core persist registry metadata
- `source-integration#KF-002` Resolve install path
  - convert `install_relpath` into an absolute path under configured ComfyUI root
- `source-integration#KF-003` Purge source on explicit request
  - delete the plugin path only when `--purge-code` is passed during remove
- `source-integration#KF-004` Export source snapshot bundle
  - copy each registered node source tree into bundle `custom_nodes/<node_id>/`
- `source-integration#KF-005` Restore source snapshot from bundle
  - restore each bundled node snapshot to target `install_relpath`, and purge bundle 外的 `custom_nodes/*` 目录 during exact import

## Runtime / Connectivity Constraints

- Requires `git` on `PATH` for add flows.
- Assumes the configured ComfyUI directory is writable by the local operator.
- `comfyui_dir` is target-machine local configuration and is never restored from bundle payload.
- The adapter does not validate plugin code safety or contents.

## Schema / DDL

- Not applicable. Persistent plugin metadata is owned by the State Ledger, not this adapter.

## Code Anchors

| Doc ID | path | symbol | line |
|---|---|---|---|
| SI-001 | `bin/gov` | `comfyui_dir_path` | 147 |
| SI-002 | `bin/gov` | `plugin_install_abs_path` | 354 |
| SI-003 | `bin/gov` | `plugin_get_meta` | 535 |
| SI-004 | `bin/gov` | `cmd_node_add` | 1276 |
| SI-005 | `bin/gov` | `cmd_node_remove` | 1369 |
