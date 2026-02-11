# 04 CLI 命令参考

> 以下为 `comfy_env/bin/gov` 当前实现语义。

## 1. 基础命令

### `gov init`

用途：初始化布局；若 `pyproject.toml` 缺失则从 `pyproject.toml.template` 生成；随后锁定依赖并创建/同步 prod 环境。

### `gov status`

用途：查看 prod、lock、事务总量与待处理事务。

## 2. 节点命令

### `gov node add <git_url> [--ref <sha/tag>] [--id <node_id>]`

用途：克隆插件并写入本地 `state/plugins.json` 注册表（v2）。

### `gov node remove <node_id> [--purge-code]`

用途：移除插件注册与依赖组，并执行依赖 GC。
关键行为：

1. 自动创建 operation backup（`state/ops/<op_id>/backup/`）。
2. 失败自动回退 pre-op 文件并重建 prod。
3. `--purge-code` 会物理删除插件目录；该代码删除不可通过 undo 恢复。

## 3. 事务命令

### `gov tx run <node_id> [--timeout <seconds>]`

用途：在 candidate 执行 ComfyUI 并记录事务。

### `gov tx inspect <txid>`

用途：输出事务摘要、差异统计、核心影响、日志路径。

### `gov tx abort <txid>`

用途：删除 candidate 并把事务标记为 `aborted`。

### `gov tx promote <txid> [--approve-core --reason "..."] [--allow-failed-run]`

用途：尝试晋升事务到 prod。
关键行为：

1. promote 开始前创建 operation backup。
2. lock 冲突写入 conflict report 并转 `needs_resolution`。
3. prod sync 或 smoke 失败时自动恢复 pre-op 文件。
4. 成功后写入 operation 元数据，支持后续 `undo`。

## 4. 冲突命令

### `gov resolve <txid>`

用途：交互输入版本 pin，重试 lock。
输入格式：`pkg==version`，空行结束。

## 5. 操作命令

### `gov op list`

用途：列出操作记录（状态、类型、引用对象、是否可撤销）。

### `gov op inspect <op_id>`

用途：查看 `state/ops/<op_id>/meta.json` 详情。

### `gov undo <op_id>`

用途：撤销指定成功操作。
关键约束：

1. 目标 op 必须 `status=success` 且 `undoable=true`。
2. 当前 `pyproject.toml`、`uv.lock`、`state/plugins.json` 的哈希必须匹配目标 op 的 `post_sha256`。
3. 不匹配则阻断，避免覆盖已漂移状态。

## 6. 占位命令

### `gov run`

当前为占位：返回非 0 并提示未完成启动编排。

## 7. 错误处理约定

1. 参数错误返回非 0，并输出 `Usage`。
2. 找不到事务或操作返回非 0。
3. destructive 操作失败时优先恢复 pre-op backup。
