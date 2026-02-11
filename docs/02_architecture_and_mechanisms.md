# 02 架构与实现机制

## 1. 总体架构

`comfy_env` 采用三层模型：

1. Local Truth Layer：`pyproject.toml` + `uv.lock`（本地文件，不进 Git）
2. Candidate Layer：`.venv-candidate/<txid>`（事务观察环境）
3. Prod Layer：`.venv-prod`（生产运行环境）

`state/plugins.json` 与 `state/ops/*` 是本地操作状态，也不进入 Git。

## 2. 关键机制

### 2.1 Transaction

1. `tx run` 创建 candidate 环境并执行 ComfyUI。
2. 记录 `pre_freeze/post_freeze`、日志与差异。
3. 事务状态驱动后续 promote/resolve/abort。

### 2.2 Promote

1. 在 `state/work/<txid>` 应用 promotion plan 并 lock。
2. promote 开始前创建 `state/ops/<op_id>/backup/`。
3. 成功后原子替换根真相并同步 prod，写入 op 元数据。
4. 任一步失败自动恢复 pre-op 并标记 `promote_failed`。

### 2.3 Remove

1. 读取插件注册并定位 `group`。
2. 以 `pyproject.toml` 的 `dependency-groups.<group>` 为权威执行 GC。
3. remove 开始前创建 op backup，失败自动恢复。

### 2.4 Undo

1. `undo <op_id>` 仅对 `success && undoable` 的操作生效。
2. 先校验当前文件哈希与目标 op 的 `post_sha256` 一致。
3. 通过校验后恢复目标 op backup 并重建 prod。

## 3. 安全边界

1. 插件代码目录不是依赖真相的一部分。
2. `--purge-code` 的代码删除不可逆，undo 不恢复被删目录。
3. 任何 destructive 操作失败都先恢复 pre-op 文件。
