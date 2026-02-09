# 09 发布检查清单

## 1. 文档一致性

1. 总纲链接全部可访问。
2. `04_cli_reference.md` 与 `bin/gov` 命令一致。
3. `05_data_contracts.md` 与当前 JSON 字段一致。

## 2. 功能验收

1. Batch-1 回归通过。
2. Batch-2 冲突/晋升链路通过。
3. Batch-3 卸载/回滚链路通过。
4. Snapshot retention 验证通过。

## 3. 故障恢复验证

1. Promote 失败可自动恢复。
2. Rollback 失败可恢复 rollback_pre。

## 4. 发布前操作

1. 记录版本号与变更摘要。
2. 保存关键验收日志。
3. 确认没有未文档化的行为改动。

## 5. 发布后观察

1. 关注冲突率（needs_resolution 次数）。
2. 关注回滚成功率。
3. 关注误删依赖/误保留依赖问题。
