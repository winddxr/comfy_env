# Comfy Environment Governance (`comfy_env`)

ComfyUI 依赖治理系统。
它通过 `Candidate 事务 -> Promote 晋升 -> Operation 备份/Undo 撤销` 机制，把插件依赖变更从“直接污染生产环境”改成“先观察、再决定、可恢复”。

## 1. 项目目标

1. 可复现：本地 `pyproject.toml + uv.lock` 可重建依赖环境。
2. 可审计：每次事务与操作都有状态、日志与元数据。
3. 可恢复：`promote/remove` 失败自动回退；成功操作可通过 `undo` 撤销。
4. 可治理：核心包变更需要显式批准。

## 2. 命令总览

1. `./bin/gov init`
2. `./bin/gov status`
3. `./bin/gov node add <git_url> [--ref <sha/tag>] [--id <node_id>]`
4. `./bin/gov node remove <node_id> [--purge-code]`
5. `./bin/gov tx run <node_id> [--timeout <seconds>]`
6. `./bin/gov tx inspect <txid>`
7. `./bin/gov tx abort <txid>`
8. `./bin/gov tx promote <txid> [--approve-core --reason "..."] [--allow-failed-run]`
9. `./bin/gov resolve <txid>`
10. `./bin/gov op list`
11. `./bin/gov op inspect <op_id>`
12. `./bin/gov undo <op_id>`

说明：`gov run` 仍是占位命令。

## 3. 目录结构

```text
comfy_env/
├── bin/
│   └── gov                     # 主 CLI
├── config.toml.template        # 配置模板（跟踪）
├── pyproject.toml.template     # 依赖模板（跟踪）
├── config.toml                 # 本地配置（不跟踪）
├── pyproject.toml              # 本地依赖真相（不跟踪）
├── uv.lock                     # 本地锁文件（不跟踪）
├── .venv-prod/                 # 生产环境
├── .venv-candidate/            # 事务候选环境
├── state/
│   ├── plugins.json            # 本地插件注册表（v2，不进 Git）
│   ├── ops/                    # 操作备份与元数据（本地）
│   ├── transactions/           # 事务记录
│   ├── conflicts/              # 冲突报告
│   ├── logs/                   # 运行日志
│   └── work/                   # 临时工作区
└── docs/                       # 文档体系
```

## 4. 核心行为

1. `tx run` 在 Candidate 环境运行并记录依赖差异。
2. `tx promote` 在工作区应用计划并 lock，成功后更新 root 真相并同步 prod。
3. `node remove` 以 `pyproject.toml` 的 `dependency-groups.<group>` 为权威执行依赖 GC。
4. `promote/remove` 开始前自动创建 `state/ops/<op_id>/backup/`。
5. 任一步骤失败自动恢复 pre-op 文件并重建 prod。
6. `undo <op_id>` 仅在哈希一致时允许撤销，避免覆盖漂移状态。

## 5. plugins.json v2

每条记录包含：

1. `id`
2. `git_url`
3. `ref`
4. `install_relpath`
5. `group`
6. `managed_deps`
7. `enabled`
8. `created_at`
9. `updated_at`

约束：

1. `install_relpath` 存储相对 `comfyui_dir` 的路径。
2. `managed_deps` 是冗余缓存，权威来源始终是 `pyproject.toml` 的 group。
3. `--purge-code` 删除插件代码后不可逆；`undo` 不恢复被 purge 的目录。

## 6. Git 跟踪边界

1. 跟踪：`*.template`、`*.example`、代码与文档。
2. 不跟踪：`config.toml`、`pyproject.toml`、`uv.lock` 与 `state/*` 运行态产物。
3. `gov init` 在 `pyproject.toml` 缺失时会自动从 `pyproject.toml.template` 生成本地文件。

## 7. 常用流程

### 7.1 新插件接入

```bash
./bin/gov node add <git_url> [--ref <sha/tag>] [--id <node_id>]
./bin/gov tx run <node_id>
./bin/gov tx inspect <txid>
./bin/gov tx promote <txid>
```

冲突时：

```bash
./bin/gov resolve <txid>
./bin/gov tx promote <txid>
```

### 7.2 卸载插件

```bash
./bin/gov node remove <node_id>
./bin/gov node remove <node_id> --purge-code
```

### 7.3 撤销一次成功操作

```bash
./bin/gov op list
./bin/gov undo <op_id>
```

## 8. 文档导航

1. `docs/04_cli_reference.md`
2. `docs/05_data_contracts.md`
3. `docs/10_quick_start_for_llm.md`
