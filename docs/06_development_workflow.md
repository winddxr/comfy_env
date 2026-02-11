# 06 开发工作流

## 1. 变更原则

1. 涉及 `promote/remove/undo` 的改动必须包含失败恢复路径。
2. 命令行为变更必须同步 `docs/04_cli_reference.md`。
3. 数据契约变更必须同步 `docs/05_data_contracts.md`。

## 2. 实现流程

1. 先在 workdir 或 candidate 环境完成计算与验证。
2. 仅在确认后替换 root 真相文件。
3. destructive 前创建 op backup。
4. 失败第一动作：恢复 pre-op backup。

## 3. 评审清单

1. 是否引入了新的不可恢复分支？
2. 是否破坏了 `undo` 的哈希一致性约束？
3. 是否保持 `pyproject group` 作为依赖权威？
4. 是否确保仅跟踪 `*.template/*.example`，并避免提交本地真相与运行态文件？
