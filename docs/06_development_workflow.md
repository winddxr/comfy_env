# 06 开发规范与变更流程

## 1. 改动前原则

1. 先查命令行为是否已在 `04_cli_reference.md` 定义。
2. 先查数据字段是否已在 `05_data_contracts.md` 定义。
3. 凡涉及 `promote/remove/rollback` 的改动，必须包含恢复路径。

## 2. 安全改动模式

1. 先在 `state/work/<txid>` 做变更与 lock。
2. lock 成功后再原子替换 root 真相文件。
3. 失败时第一动作是恢复前置 snapshot。

## 3. 代码修改清单

每次修改 `bin/gov` 必做：

1. 更新 `cmd_help`。
2. 更新 `04_cli_reference.md`。
3. 若新增字段，更新 `05_data_contracts.md`。
4. 补充至少一个失败场景测试到 `07`。

## 4. 提交前检查

1. 文档链接有效。
2. 命令与文档一致。
3. 状态机流转无断裂。
4. 回滚链路未被破坏。

## 5. 推荐分支策略

1. `feature/<topic>` 做功能。
2. `docs/<topic>` 做纯文档。
3. 合并前执行 `07` 的核心验收用例。

## 6. 常见高风险改动

1. 直接改写 root `pyproject.toml`。
2. 跳过 `uv lock` 就进入 prod sync。
3. 删除 snapshot 逻辑。
4. 变更 tx 状态值但未同步文档与解析。
