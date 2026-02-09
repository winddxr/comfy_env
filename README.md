# Comfy Environment Governance (`comfy_env`)

ComfyUI 依赖治理系统。  
它通过 `Candidate 事务 -> Promote 晋升 -> Snapshot 回滚` 机制，把插件依赖变更从“直接污染生产环境”改成“先观察、再决定、可恢复”。

## 1. 这个项目解决什么问题

ComfyUI 插件生态常见问题：

1. 插件安装/更新引发依赖冲突，导致环境突然不可用。
2. 依赖来源不清晰，难以追踪“哪个操作改坏了环境”。
3. 卸载插件后依赖残留，环境越来越脏。
4. 失败后无法快速回退到稳定状态。

`comfy_env` 的目标：

1. 可复现：`pyproject.toml + uv.lock` 可重建环境。
2. 可审计：每次事务有日志、差异、状态。
3. 可回滚：关键操作前后有快照，支持恢复。
4. 可治理：核心包变更需要显式批准。

## 2. 面向谁

1. 人类开发者：需要稳定管理 ComfyUI 及插件依赖。
2. 后续接手 LLM：需要快速理解项目意图、机制与操作边界。

## 3. 当前功能状态

已实现命令（按 `bin/gov` 当前实现）：

1. `gov init`  
用途：初始化目录布局，生成/更新 `uv.lock`，并创建或同步 prod 虚拟环境。  
用法：`./bin/gov init`  
参数：无。  
关键行为：执行 `uv lock` 和 `UV_PROJECT_ENVIRONMENT=<prod_env> uv sync --locked --exact`。

2. `gov status`  
用途：查看当前治理状态摘要。  
用法：`./bin/gov status`  
参数：无。  
关键输出：`prod_env_exists`、`lock_exists`、事务总数、待处理事务数、最近事务列表。

3. `gov node add`  
用途：通过 Git 安装插件，并写入 `state/plugins.json` 注册信息。  
用法：`./bin/gov node add <git_url> [--ref <sha/tag>] [--id <node_id>]`  
参数：  
`<git_url>` 必填，插件仓库地址。  
`--ref` 可选，检出指定 tag/commit/branch。  
`--id` 可选，插件 ID；未提供时默认取仓库名（去掉 `.git`）。  
关键行为：克隆到 `<comfyui_dir>/custom_nodes/<node_id>`，注册 `group=node-<node_id>`，初始化 `managed_deps` 等字段。

4. `gov node remove`  
用途：移除插件注册与其受管依赖，并执行依赖 GC。  
用法：`./bin/gov node remove <node_id> [--purge-code]`  
参数：  
`<node_id>` 必填，待卸载插件 ID。  
`--purge-code` 可选，同时删除插件代码目录。  
关键行为：创建 `remove_pre` 快照，在工作区执行 `uv remove --group node-<id>` + `uv lock`，成功后同步 prod；失败自动回滚至快照。

5. `gov tx run`  
用途：在 Candidate 环境运行插件首次启动事务，采集依赖变化与日志。  
用法：`./bin/gov tx run <node_id> [--timeout <seconds>]`  
参数：  
`<node_id>` 必填，目标插件。  
`--timeout` 可选，覆盖 `config.toml` 的 `tx.timeout_seconds`。  
关键行为：创建 `state/transactions/<txid>.json`、candidate venv、pre/post freeze、stdout/stderr 日志；根据运行退出码标记 `completed/failed`。

6. `gov tx inspect`  
用途：查看单个事务的结构化摘要。  
用法：`./bin/gov tx inspect <txid>`  
参数：`<txid>` 必填。  
关键输出：状态、前后时间、diff 统计、核心包影响、冲突报告路径、promote 状态、日志路径、运行退出码。

7. `gov tx abort`  
用途：中止事务并清理 candidate 环境。  
用法：`./bin/gov tx abort <txid>`  
参数：`<txid>` 必填。  
关键行为：删除事务对应 candidate venv，并把事务状态改为 `aborted`（不改动 prod）。

8. `gov resolve`  
用途：对冲突事务交互输入版本 pin 并重试解算。  
用法：`./bin/gov resolve <txid>`  
参数：`<txid>` 必填。  
交互输入：逐行输入 `pkg==version`，空行结束。  
关键行为：仅接受 `needs_resolution/promote_failed/resolved` 状态事务，更新 `resolution_pins`，在工作区重试 lock，成功标记 `resolved`，失败重新写冲突报告并保持 `needs_resolution`。

9. `gov tx promote`  
用途：把候选事务晋升到 prod（带门禁、快照、失败回退）。  
用法：`./bin/gov tx promote <txid> [--approve-core --reason "..."] [--allow-failed-run]`  
参数：  
`<txid>` 必填。  
`--approve-core` 可选，显式批准核心包变更。  
`--reason` 可选，记录审批理由（建议与 `--approve-core` 搭配）。  
`--allow-failed-run` 可选，允许 `failed` 状态事务继续尝试 promote。  
关键行为：先做 `promote_pre` 快照，再在 `state/work/<txid>/` 应用计划并 lock；lock 冲突时写 `state/conflicts/<txid>.json` 并转 `needs_resolution`；成功后原子替换根目录真相文件、同步 prod、执行 smoke test、写 `promote_post` 快照；任一步失败自动恢复 pre 快照并标记 `promote_failed`。

10. `gov snapshot list`  
用途：列出快照索引。  
用法：`./bin/gov snapshot list`  
参数：无。  
关键输出：`snapshot_id`、创建时间、reason、ref。

11. `gov snapshot inspect`  
用途：查看单个快照元数据。  
用法：`./bin/gov snapshot inspect <snapshot_id>`  
参数：`<snapshot_id>` 必填。  
关键行为：输出 `snapshots/<snapshot_id>/meta.json`；快照不存在时返回错误码 2。

12. `gov rollback`  
用途：回滚依赖真相与插件注册到指定快照，并重建 prod。  
用法：`./bin/gov rollback <snapshot_id>`  
参数：`<snapshot_id>` 必填。  
关键行为：先自动创建 `rollback_pre` 快照，再恢复目标快照并执行 prod `uv sync --locked --exact`；若同步失败，自动恢复 `rollback_pre`。

说明：

1. `gov run` 目前仍为占位命令，会返回非 0，不承担完整启动编排。

## 4. 核心目录结构

```text
comfy_env/
├── bin/
│   └── gov                     # 主 CLI
├── config.toml                 # 配置
├── pyproject.toml              # 声明真相
├── uv.lock                     # 锁定真相
├── .venv-prod/                 # 生产环境
├── .venv-candidate/            # 事务候选环境
├── snapshots/                  # 回滚快照
├── state/
│   ├── plugins.json            # 插件注册
│   ├── snapshots_index.json    # 快照索引
│   ├── transactions/           # 事务记录
│   ├── conflicts/              # 冲突报告
│   ├── logs/                   # 运行日志
│   └── work/                   # 临时工作区
└── docs/                       # 文档体系
```

## 5. 运行前置条件

验收目标平台：Linux。

必须：

1. `bash`
2. `uv`
3. `git`
4. `python3`（或 `python`）
5. ComfyUI 目录存在，且包含 `main.py`

## 6. 最小上手流程

在 `comfy_env/` 目录执行：

```bash
./bin/gov init
./bin/gov status
```

## 7. 常用工作流

## 7.1 新插件接入

```bash
./bin/gov node add <git_url> [--ref <sha/tag>] [--id <node_id>]
./bin/gov tx run <node_id>
./bin/gov tx inspect <txid>
./bin/gov tx promote <txid>
```

如果 promote 冲突：

```bash
./bin/gov resolve <txid>
./bin/gov tx promote <txid>
```

## 7.2 卸载插件并做依赖 GC

```bash
./bin/gov node remove <node_id>
```

同时删除插件代码目录：

```bash
./bin/gov node remove <node_id> --purge-code
```

## 7.3 回滚恢复

```bash
./bin/gov snapshot list
./bin/gov rollback <snapshot_id>
```

## 8. 关键机制（简版）

1. `tx run` 在 Candidate 环境运行，记录前后依赖变化。
2. `tx promote` 在工作区尝试应用变更并 lock，成功后才进入 Prod。
3. lock 冲突会进入 `needs_resolution`，通过 `resolve` 输入版本 pin 重试。
4. 关键动作有 snapshot 前置，失败优先回滚。

## 9. 关键约束（请严格遵守）

1. 不要直接手改 `state/transactions/*.json`。
2. 不要绕过 `gov` 在 root 直接写依赖作为常规流程。
3. 修改命令行为后，必须同步更新 `docs/04_cli_reference.md`。
4. 修改数据字段后，必须同步更新 `docs/05_data_contracts.md`。

## 10. 故障处理入口

常见排障路径：

1. 先看命令退出码和 stderr。
2. 再看 `state/logs/` 与 `state/conflicts/`。
3. 再看 `state/transactions/<txid>.json`。
4. 无法快速修复时优先执行 `rollback`。

完整排障文档：

1. `docs/08_troubleshooting_and_limits.md`

## 11. 文档导航（给人类/LLM）

总纲：

1. `docs/comfy_env_dev_test_guide.md`

快速上手（给接手 LLM）：

1. `docs/10_quick_start_for_llm.md`

详细文档：

1. `docs/01_project_objectives.md`
2. `docs/02_architecture_and_mechanisms.md`
3. `docs/03_runtime_and_operations.md`
4. `docs/04_cli_reference.md`
5. `docs/05_data_contracts.md`
6. `docs/06_development_workflow.md`
7. `docs/07_test_strategy_and_acceptance.md`
8. `docs/08_troubleshooting_and_limits.md`
9. `docs/09_release_checklist.md`
