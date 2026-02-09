# 03 运行方式与日常操作

## 1. 环境前置

1. Linux（验收主平台）。
2. 可用命令：`bash`, `uv`, `git`, `python3`。
3. ComfyUI 目录存在且包含 `main.py`。

## 2. 首次初始化

```bash
cd comfy_env
./bin/gov init
./bin/gov status
```

结果预期：

1. 生成或更新 `uv.lock`。
2. 创建 `.venv-prod`。
3. `status` 能显示 prod 和事务状态摘要。

## 3. 新插件接入

```bash
./bin/gov node add <git_url> [--ref <sha/tag>] [--id <node_id>]
./bin/gov tx run <node_id>
./bin/gov tx inspect <txid>
```

如果 tx 可晋升：

```bash
./bin/gov tx promote <txid>
```

如果冲突：

```bash
./bin/gov resolve <txid>
./bin/gov tx promote <txid>
```

## 4. 核心包变更审批

当事务命中核心包影响时，必须使用：

```bash
./bin/gov tx promote <txid> --approve-core --reason "<why>"
```

## 5. 卸载插件与清理

```bash
./bin/gov node remove <node_id>
# 如果同时删除插件目录代码：
./bin/gov node remove <node_id> --purge-code
```

## 6. 回滚

```bash
./bin/gov snapshot list
./bin/gov rollback <snapshot_id>
```

## 7. 推荐日常流程

1. 每次插件变更都走 `tx run -> tx inspect -> promote`。
2. 遇到冲突优先 `resolve`，不要手改 `pyproject.toml`。
3. 任何异常先 `snapshot list`，再执行 rollback。

## 8. 禁止事项

1. 不要直接编辑 `state/transactions/*.json`。
2. 不要绕过 `gov` 在 root 直接 `uv add` 作为常规流程。
3. 不要在未留快照时执行破坏性修改。
