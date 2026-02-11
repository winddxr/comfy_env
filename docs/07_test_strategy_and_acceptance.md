# 07 测试策略与验收矩阵

## 1. 测试分层

1. 静态检查：命令路由、函数存在、文件契约。
2. 集成测试：真实 `uv` 下命令行为。
3. 端到端：`tx run -> (resolve) -> promote -> remove -> undo`。

## 2. 核心验收矩阵

### 2.1 Promote 路径

1. 成功路径：状态到 `promoted`，并产出 `op_id`。
2. lock 冲突：状态到 `needs_resolution`，并生成 conflict report。
3. sync/smoke 失败：自动恢复 pre-op，状态到 `promote_failed`。

### 2.2 Remove 路径

1. 成功路径：插件记录删除，prod exact sync 成功。
2. group 漂移：仍以 `pyproject` group 为权威执行删除。
3. `--purge-code`：命令提示不可逆代码删除。

### 2.3 Undo 路径

1. 哈希匹配时可撤销成功 op。
2. 哈希不匹配时阻断并给出明确错误。
3. 撤销后目标 op 标记为 `undone`。

## 3. 判定标准

1. 退出码符合预期。
2. tx/op JSON 字段与状态流转一致。
3. 依赖真相与 prod 环境一致可重建。
