# Comfy Env 开发与测试总览

适用范围：`comfy_env/` 下依赖治理系统（`bin/gov`、本地 `pyproject.toml/uv.lock`、`state/`）。

## 1. 当前模型

`comfy_env` 使用：

1. Candidate 事务（`tx run`）
2. Promote 晋升（`tx promote`）
3. Operation 备份与 Undo 撤销（`op/undo`）

## 2. 已实现命令

1. `init`, `status`
2. `node add/remove`
3. `tx run/inspect/abort/promote`
4. `resolve`
5. `op list/inspect`
6. `undo`

## 3. 测试重点

1. Promote/Remove 失败自动恢复 pre-op。
2. Undo 哈希一致性校验。
3. `plugins.json v2` 迁移与字段约束。
4. `--purge-code` 不可逆行为说明。

## 4. 维护要求

1. CLI 变更同步 `docs/04`。
2. 数据字段变更同步 `docs/05`。
3. 仓库只跟踪模板/范例，运行状态与本地真相文件不进入 Git。
