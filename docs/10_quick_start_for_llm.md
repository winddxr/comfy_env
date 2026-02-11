# 10 给接手 LLM 的快速上手

## 1. 先建立心智模型

1. 依赖真相是本地文件：`pyproject.toml` 和 `uv.lock`（都不进 Git）。
2. 仓库只分发模板/范例：`pyproject.toml.template`、`config.toml.template`、`state/plugins.json.example`。
3. `state/plugins.json` 是本地注册表，不进入 Git，不作为跨机真相。
4. 恢复模型是 `operation backup + undo`。
5. `promote/remove` 前会生成 `state/ops/<op_id>/backup/`。
6. `undo <op_id>` 不是“回到历史某时刻”，而是“撤销某次具体操作”。

## 2. 首次落地（模板到本地文件）

```bash
cp config.toml.template config.toml
./bin/gov init
```

说明：

1. `gov init` 会在缺失时自动生成 `pyproject.toml`，来源是 `pyproject.toml.template`。
2. `uv.lock` 不提供模板，由 `uv lock` 自动生成。

## 3. 核心目录与用途

1. `bin/gov`：主 CLI。
2. `state/transactions/`：`tx run` 生成的事务记录。
3. `state/ops/`：`promote/remove/undo` 操作元数据和备份。
4. `state/conflicts/`：lock 冲突报告。
5. `state/logs/`：命令执行日志。

## 4. 命令速查（用途 + 用法）

### 4.1 环境与状态

1. `./bin/gov init`
用途：初始化布局；若 `pyproject.toml` 缺失则从 `pyproject.toml.template` 自动生成，再同步 prod 环境。
2. `./bin/gov status`
用途：快速看事务总量、待处理数量、最近事务。

### 4.2 插件注册与移除

1. `./bin/gov node add <git_url> [--ref <sha/tag>] [--id <node_id>]`
用途：克隆插件并写入 `state/plugins.json`。
2. `./bin/gov node remove <node_id> [--purge-code]`
用途：移除插件注册并做依赖 GC。
关键点：
3. 默认不删插件代码目录。
4. `--purge-code` 会物理删除目录，undo 不恢复代码目录。

### 4.3 事务与晋升

1. `./bin/gov tx run <node_id> [--timeout <seconds>]`
用途：在 candidate 环境运行并生成事务。
2. `./bin/gov tx inspect <txid>`
用途：查看事务状态、diff、日志路径。
3. `./bin/gov tx promote <txid> [--approve-core --reason "..."] [--allow-failed-run]`
用途：把事务结果晋升到 prod。
关键点：
4. 晋升时自动创建 operation backup。
5. 失败会自动恢复 pre-op 文件并写失败状态。

### 4.4 冲突处理

1. `./bin/gov resolve <txid>`
用途：输入 `pkg==version` 修复 lock 冲突，然后重试 promote。

### 4.5 操作审计与撤销

1. `./bin/gov op list`
用途：列出操作记录（`kind/status/ref/undoable`）。
2. `./bin/gov op inspect <op_id>`
用途：查看操作详情（哈希、backup 路径、note）。
3. `./bin/gov undo <op_id>`
用途：撤销指定成功操作。
阻断条件：
4. 目标 op 不是 `success + undoable=true`。
5. 当前文件哈希与目标 op 的 `post_sha256` 不一致。

## 5. 推荐工作流（标准路径）

### 5.1 新插件接入

```bash
./bin/gov node add <git_url> [--id <node_id>]
./bin/gov tx run <node_id>
./bin/gov tx inspect <txid>
./bin/gov tx promote <txid>
```

成功判定：

1. `tx inspect` 显示 `status: promoted`。
2. promote 输出含 `op_id`。

### 5.2 冲突修复

```bash
./bin/gov resolve <txid>
./bin/gov tx promote <txid>
```

成功判定：

1. `resolve` 输出 `status: resolved`。
2. 重新 promote 成功并产出 `op_id`。

### 5.3 卸载与撤销

```bash
./bin/gov node remove <node_id>
./bin/gov op list
./bin/gov undo <remove_op_id>
```

成功判定：

1. remove 输出 `op_id` 且对应 op `status=success`。
2. undo 后目标 remove op 变 `status=undone`。

## 6. 输出字段怎么读

1. `txid`：事务 ID，对应 `state/transactions/<txid>.json`。
2. `op_id`：操作 ID，对应 `state/ops/<op_id>/meta.json`。
3. `promotion.op_id`：某次事务晋升关联的操作 ID。
4. `undoable`：该 op 当前是否允许执行 undo。
5. `note`：失败原因或撤销说明。

## 7. 常见失败与处理

1. `tx run` 失败：看 `state/logs/<txid>.stderr.log`。
2. `tx promote` 失败：先看 `state/conflicts/`，再看 `op inspect <op_id>` 的 `note`。
3. `node remove` 失败：看 `state/logs/remove-<node_id>.lock.log`。
4. `undo` 失败且提示 `target operation is not undoable`：说明目标 op 不可撤销，换可撤销 op。
5. `undo` 失败且提示哈希不一致：当前状态已偏离，先定位后再决定是否继续。

## 8. 接手时最小核查清单

1. `./bin/gov help` 包含 `op list/op inspect/undo`。
2. `docs/04_cli_reference.md` 与 `docs/05_data_contracts.md` 与实现一致。
3. `.gitignore` 已忽略 `config.toml`、`pyproject.toml`、`uv.lock`、`state/plugins.json` 与运行态目录。
