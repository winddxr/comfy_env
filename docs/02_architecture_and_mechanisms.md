# 02 架构与实现机制

## 1. 总体架构

`comfy_env` 采用三层环境模型：

1. Truth Layer：`pyproject.toml` + `uv.lock` + `state/plugins.json`
2. Candidate Layer：`.venv-candidate/<txid>`（事务观察环境）
3. Prod Layer：`.venv-prod`（生产运行环境）

## 2. 关键机制

## 2.1 Transaction（事务）

1. `tx run` 创建 candidate 环境并执行 ComfyUI。
2. 记录 `pre_freeze`/`post_freeze` 差异与日志。
3. 事务状态驱动后续路径（promote/resolve/abort）。

## 2.2 Promote（晋升）

1. 读取事务差异生成 promotion plan。
2. 在 `state/work/<txid>` 工作区应用依赖变更并 `uv lock`。
3. lock 成功后才替换根目录真相文件并重建 prod。

## 2.3 Resolve（冲突修复）

1. lock 失败进入 `needs_resolution`。
2. 用户输入 `pkg==version` 作为 pin。
3. pin 进入 `dependency-groups.overrides` 路径重试解算。

## 2.4 Snapshot（快照）

1. 在关键动作前后自动创建 snapshot。
2. snapshot 记录真相文件和插件注册。
3. 超过保留数量自动裁剪最旧快照。

## 2.5 Rollback（回滚）

1. 恢复 snapshot 中的真相文件。
2. 重新执行 prod sync。
3. 若回滚失败，自动恢复到 rollback 前快照。

## 3. 关键路径

1. 新插件：`node add -> tx run -> (resolve) -> tx promote`
2. 卸载插件：`node remove`
3. 紧急恢复：`rollback <snapshot_id>`

## 4. 安全边界

1. 仅恢复依赖真相 + 插件注册，不恢复 `custom_nodes` 代码工作树。
2. 核心包命中 `core_impact` 时，promote 需 `--approve-core`。
3. 所有 destructive 操作要求快照前置。

## 5. 为什么这样设计

1. 插件 `install.py` 执行时机不可控，Candidate 隔离可避免直接污染 Prod。
2. 工作区加锁避免半写入状态。
3. 快照与状态机结合，可把失败转化为可恢复事件。
