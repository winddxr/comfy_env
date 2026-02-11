# 05 数据与状态契约

## 1. 本地真相层与运行状态边界

本地真相文件（不进入 Git）：

1. `pyproject.toml`
2. `uv.lock`

模板/范例文件（进入 Git）：

1. `pyproject.toml.template`
2. `state/plugins.json.example`
3. `config.toml.template`

本地运行状态（不进入 Git）：

1. `state/plugins.json`
2. `state/ops/`
3. `state/transactions/`
4. `state/logs/`
5. `state/conflicts/`
6. `state/work/`

## 2. plugins.json v2 结构

`state/plugins.json` 顶层为数组，每个插件项字段：

1. `id`
2. `git_url`
3. `ref`
4. `install_relpath`（相对 `comfyui_dir`）
5. `group`（通常 `node-<id>`）
6. `managed_deps`（冗余缓存）
7. `enabled`
8. `created_at`
9. `updated_at`

语义约束：

1. `managed_deps` 与实际删除逻辑不一致时，以 `pyproject.toml` 的 `dependency-groups.<group>` 为权威。
2. `path` / `last_txid` / `last_snapshot` 已废弃。

## 3. 事务 JSON 结构

`state/transactions/<txid>.json` 核心字段：

1. 基础：`txid`, `node_id`, `started_at`, `ended_at`, `status`
2. 环境：`candidate_env`
3. 差异：`pre_freeze`, `post_freeze`, `diff.added`, `diff.removed`, `core_impact`
4. 日志：`logs.stdout`, `logs.stderr`, `logs.run_exit_code`
5. 冲突与晋升：`conflict_report`, `resolution_pins`, `promotion_plan`, `promotion`

`promotion` 关键子字段：

1. `status`
2. `reason`
3. `op_id`
4. `pre_op_id`
5. `error`

## 4. Operation 元数据结构

`state/ops/<op_id>/meta.json`：

1. `op_id`
2. `kind`（`promote` / `remove` / `manual`）
3. `ref`
4. `status`（`running` / `success` / `failed` / `undone`）
5. `started_at`
6. `ended_at`
7. `files`：每个文件包含 `pre_sha256` / `post_sha256`
8. `backup_dir`
9. `undoable`
10. `note`

备份目录固定为：

1. `state/ops/<op_id>/backup/pyproject.toml`
2. `state/ops/<op_id>/backup/uv.lock`
3. `state/ops/<op_id>/backup/plugins.json`

## 5. 状态机

事务允许状态集合：

1. `running`
2. `completed`
3. `failed`
4. `aborted`
5. `needs_resolution`
6. `resolved`
7. `promoted`
8. `promote_failed`

常见流转：

1. `running -> completed|failed`
2. `completed -> promoted|needs_resolution`
3. `failed -> promote_failed|needs_resolution`（需 `--allow-failed-run`）
4. `needs_resolution -> resolved|needs_resolution`
5. `resolved -> promoted|needs_resolution`

## 6. 向后兼容规则

1. 新字段只能追加，不得无迁移重命名旧字段。
2. 解析方必须容忍未知字段。
3. 修改字段语义时必须更新文档和迁移说明。

## 7. 环境重建约定

`init` / `promote` / `remove` / `undo` 的 prod 同步使用：

1. `uv sync --locked --exact --all-groups`
2. `dependency-groups` 中受管依赖属于重建范围。
