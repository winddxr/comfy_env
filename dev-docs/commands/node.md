# node

## `node add`

### Synopsis

```
gov node add <git_url> [--ref <sha|tag|branch>] [--id <node_id>]
```

### Purpose

Register a ComfyUI plugin by cloning its Git repository into `custom_nodes/`. Does not install dependencies — that requires a subsequent `tx run`.

### Arguments

| Flag | Required | Description |
|------|----------|-------------|
| `<git_url>` | Yes | Git repository URL |
| `--ref` | No | Git ref to checkout (default: repository default branch) |
| `--id` | No | Override node_id (default: basename of git_url without `.git`) |

### Preconditions

- `config.toml` must exist
- `git` must be available
- `comfyui_dir` must exist
- Node with same id must not already exist in plugins.json

### Reads

- `config.toml` — comfyui_dir
- `state/plugins.json` — check for duplicate id

### Writes

- `custom_nodes/<node_id>/` — cloned source tree
- `state/plugins.json` — appends new plugin record

### Success Path

```
1. Derive node_id from git_url basename (or use --id)
2. Verify node_id not already registered
3. Determine install path: <comfyui_dir>/custom_nodes/<node_id>
4. `git clone <git_url> <install_path>`
5. IF --ref provided: `git -C <install_path> checkout <ref>`
6. Generate group name: "node-<normalized_node_id>"
7. Append to plugins.json:
   {
     id: node_id,
     git_url: git_url,
     ref: ref or "HEAD",
     install_relpath: "custom_nodes/<node_id>",
     group: "node-<normalized>",
     enabled: true,
     managed_deps: [],
     created_at: <utc_timestamp>,
     updated_at: <utc_timestamp>
   }
```

### Failure Path

```
IF duplicate node_id: exit with error (no mutation)
IF git clone fails: clean up partial clone directory, exit
IF git checkout fails: clean up cloned directory, exit
```

### Platform Notes

- `install_relpath` uses forward slashes in plugins.json regardless of platform
- Git clone target path uses platform-native separators for the filesystem operation

---

## `node remove`

### Synopsis

```
gov node remove <node_id> [--purge-code]
```

### Purpose

Remove a plugin's dependency group from `pyproject.toml`, re-lock, sync prod, and update the registry. Optionally delete source code.

### Arguments

| Flag | Required | Description |
|------|----------|-------------|
| `<node_id>` | Yes | Plugin identifier |
| `--purge-code` | No | Also delete `custom_nodes/<node_id>/` directory |

### Preconditions

- `config.toml` must exist
- Plugin must exist in `plugins.json`

### Reads

- `config.toml` — python, prod env, comfyui_dir
- `state/plugins.json` — plugin record (group name, install path)
- `pyproject.toml` — dependency group for the plugin

### Writes

- `pyproject.toml` — removes `dependency-groups.<group_name>` section
- `uv.lock` — re-locked
- `.venv-prod/` — synced
- `state/plugins.json` — removes plugin record
- `custom_nodes/<node_id>/` — deleted if `--purge-code`

### Success Path

```
1. Load plugin record from plugins.json
2. op_begin(kind="node_remove", reference=node_id)
3. Copy truth to staged workdir
4. Remove dependency group from workdir pyproject.toml:
   - Remove all packages: `uv remove --group <group> --frozen <pkg>` for each
   - Remove the group section itself
5. Lock workdir via [Staged Workdir Protocol]
6. IF lock succeeds:
   a. Copy workdir truth → root
   b. Sync prod env via [Prod Sync Protocol]
   c. Remove plugin entry from plugins.json
   d. IF --purge-code: delete custom_nodes/<node_id>/
   e. op_finalize(success)
```

### Failure Path

```
IF plugin not found: exit with error (no mutation)
IF lock fails: op_restore → re-sync prod → op_finalize(failed)
IF sync fails: op_restore → re-sync prod → op_finalize(failed)
```

### Platform Notes

- Directory deletion must handle read-only files (Git on Windows sets some files read-only)
- Symlinks/junctions in custom_nodes: delete the link, not the target
