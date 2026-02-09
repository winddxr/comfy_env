# 05 数据与状态契约

## 1. 真相层文件

1. `pyproject.toml`
2. `uv.lock`
3. `state/plugins.json`

这三者是 rollback 恢复范围。

## 2. plugins.json 结构（逻辑）

每个插件记录至少包含：

1. `id`
2. `git_url`
3. `ref`
4. `path`
5. `group`（通常 `node-<id>`）
6. `enabled`
7. `managed_deps`
8. `last_txid`
9. `last_snapshot`

## 3. 事务 JSON 结构

`state/transactions/<txid>.json` 核心字段：

1. 基础：`txid`, `node_id`, `started_at`, `ended_at`, `status`
2. 环境：`candidate_env`
3. 差异：`pre_freeze`, `post_freeze`, `diff.added`, `diff.removed`, `core_impact`
4. 日志：`logs.stdout`, `logs.stderr`, `logs.run_exit_code`
5. 冲突与晋升：`conflict_report`, `resolution_pins`, `promotion_plan`, `promotion`

## 4. 状态机

允许状态集合：

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

## 5. 快照索引契约

`state/snapshots_index.json`：

1. 顶层键：`snapshots`（数组）
2. 项目键：`id`, `created_at`, `reason`, `ref`

## 6. 向后兼容规则

1. 新字段只能追加，不得无迁移重命名旧字段。
2. 解析方必须容忍未知字段。
3. 修改字段语义时必须更新文档和迁移说明。

## 7. 日志与临时文件

1. 运行日志：`state/logs/`
2. 冲突报告：`state/conflicts/`
3. 工作区：`state/work/`

这些属于过程文件，不属于 rollback 真相恢复范围。
