# 06 开发工作流

## 1. 变更原则

1. 涉及 `promote/remove/undo` 的改动必须包含失败恢复路径。
2. 命令行为变更必须同步 `docs/04_cli_reference.md`。
3. 数据契约变更必须同步 `docs/05_data_contracts.md`。
4. `env export/env import` 行为变更还必须同步 `README.md`、`docs/11_env_export_import_handoff.md` 和相关 `dev-docs/*` 规范文档。

## 2. 实现流程

1. 先在 workdir 或 candidate 环境完成计算与验证。
2. 仅在确认后替换 root 真相文件。
3. destructive 前创建 op backup。
4. 失败第一动作：恢复 pre-op backup。
5. `env import` 这类整体恢复路径，必须先校验 bundle 完整性与 staging lock，再改写 root truth。
6. 若 destructive 同时覆盖了 `custom_nodes` 源码目录，也必须有对应的失败恢复路径。
7. 若 `env import` 默认会清理 bundle 外节点目录，则被清理的目录也属于必须可回滚的恢复范围。
8. `env export` 若导出运行态源码快照，必须明确哪些工作树内容保留、哪些 VCS 元数据被过滤，并保持文档与测试一致。

## 3. 评审清单

1. 是否引入了新的不可恢复分支？
2. 是否破坏了 `undo` 的哈希一致性约束？
3. 是否保持 `pyproject group` 作为依赖权威？
4. 是否确保仅跟踪 `*.template/*.example`，并避免提交本地真相与运行态文件？
5. 若改动涉及 `env import/export`，是否仍保持目标机 `comfyui_dir` 为本地配置、bundle 为运输载体、导入为 exact restore？
