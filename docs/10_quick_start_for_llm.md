# 10 LLM 快速上手（5-10 分钟）

> 目标：让新接手的 LLM 在最短时间理解项目目的、关键机制和操作路径，避免误改。

## 1. 先记住这三句话

1. 这是一个 ComfyUI 依赖治理项目，核心是 **Candidate 事务** 和 **Promote 晋升**。
2. 生产真相只有三类：`pyproject.toml`、`uv.lock`、`state/plugins.json`。
3. 任何高风险动作都必须有快照前置，并支持回滚。

## 2. 你正在维护什么

目录：`comfy_env/`  
核心脚本：`comfy_env/bin/gov`

关键子目录：

1. `state/transactions/`：事务记录
2. `state/conflicts/`：冲突报告
3. `state/work/`：临时工作区
4. `snapshots/`：回滚快照

## 3. 关键工作流（最常用）

## 3.1 新插件接入

```bash
./bin/gov node add <git_url> [--id <node_id>]
./bin/gov tx run <node_id>
./bin/gov tx inspect <txid>
./bin/gov tx promote <txid>
```

若 promote 冲突：

```bash
./bin/gov resolve <txid>
./bin/gov tx promote <txid>
```

## 3.2 卸载插件

```bash
./bin/gov node remove <node_id>
# 或删除插件代码目录
./bin/gov node remove <node_id> --purge-code
```

## 3.3 回滚恢复

```bash
./bin/gov snapshot list
./bin/gov rollback <snapshot_id>
```

## 4. 你必须遵守的规则

1. 不要直接手改 `state/transactions/*.json`。
2. 不要绕过 `gov` 直接把试验依赖写进 root 真相文件。
3. 变更 `gov` 行为时，必须同步更新文档：
   1. `docs/04_cli_reference.md`
   2. `docs/05_data_contracts.md`
   3. `docs/07_test_strategy_and_acceptance.md`

## 5. 状态机速记

常见事务状态：

1. `running`
2. `completed`
3. `failed`
4. `needs_resolution`
5. `resolved`
6. `promoted`
7. `promote_failed`
8. `aborted`

典型流转：

1. `running -> completed|failed`
2. `completed -> promoted|needs_resolution`
3. `needs_resolution -> resolved -> promoted`

## 6. 快速排错路径

1. 先看 `gov` 命令退出码和 stderr。
2. 再看：
   1. `state/logs/`
   2. `state/conflicts/<txid>.json`
   3. `state/transactions/<txid>.json`
3. 异常无法快速修复时，优先 `rollback` 到最近稳定 snapshot。

## 7. 最小阅读清单（按优先级）

1. `docs/comfy_env_dev_test_guide.md`（总纲）
2. `docs/02_architecture_and_mechanisms.md`（机制）
3. `docs/04_cli_reference.md`（命令）
4. `docs/05_data_contracts.md`（数据）
5. `docs/07_test_strategy_and_acceptance.md`（验收）

## 8. 交付前自检（给 LLM）

1. 你改动的命令，`cmd_help` 是否同步更新？
2. 新增/变更字段，`05_data_contracts.md` 是否同步？
3. 是否给出至少 1 个成功路径和 1 个失败恢复路径验证步骤？
4. 是否确认回滚链路没有被破坏？
