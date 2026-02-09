# Comfy Env 文档总纲

> 本文档是 `comfy_env` 项目的总导航。后续开发、测试、交接请先读本页，再按链接阅读分文档。  
> 适用范围：`comfy_env/` 目录内的依赖治理系统（`bin/gov`、`config.toml`、`pyproject.toml`、`state/`、`snapshots/`）。

## 1. 先读什么

1. 项目目标与边界：[`01_project_objectives.md`](./01_project_objectives.md)
2. 架构与实现机制：[`02_architecture_and_mechanisms.md`](./02_architecture_and_mechanisms.md)
3. 运行方式与日常操作：[`03_runtime_and_operations.md`](./03_runtime_and_operations.md)
4. CLI 命令手册：[`04_cli_reference.md`](./04_cli_reference.md)
5. 数据与状态契约：[`05_data_contracts.md`](./05_data_contracts.md)
6. 开发规范与变更流程：[`06_development_workflow.md`](./06_development_workflow.md)
7. 测试策略与验收矩阵：[`07_test_strategy_and_acceptance.md`](./07_test_strategy_and_acceptance.md)
8. 故障排查与已知限制：[`08_troubleshooting_and_limits.md`](./08_troubleshooting_and_limits.md)
9. 发布检查清单：[`09_release_checklist.md`](./09_release_checklist.md)
10. LLM 快速上手：[`10_quick_start_for_llm.md`](./10_quick_start_for_llm.md)

## 2. 项目一句话说明

`comfy_env` 通过 “Candidate 事务 + Promote 晋升 + Snapshot 回滚” 的机制，治理 ComfyUI 及插件依赖，目标是可复现、可回滚、可审计，并降低插件依赖冲突带来的爆炸风险。

## 3. 角色分工（给后续 LLM/开发者）

1. 想知道项目为什么存在：看 `01`。
2. 想理解系统怎么工作：看 `02` + `05`。
3. 想直接跑起来：看 `03` + `04`。
4. 想改代码：看 `06`，再按 `07` 跑测试。
5. 想排障：看 `08`。
6. 想发布：看 `09`。

## 4. 当前实现状态（对齐代码）

1. 已有：`init`, `node add`, `tx run/inspect/abort`, `resolve`, `tx promote`, `node remove`, `rollback`, `snapshot list/inspect`, `status`。
2. `gov run` 仍是占位命令，不承担完整启动编排。

## 5. 文档维护规则

1. 修改命令行为时，必须同步更新 `04`、`05`、`07`。
2. 修改状态字段或 JSON 结构时，必须同步更新 `05` 并追加迁移说明。
3. 每次版本发布前，必须完成 `09` 全项勾选。
