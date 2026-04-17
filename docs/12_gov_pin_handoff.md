# 12 Gov Pin Handoff

## 1. 目标

为 `comfy_env` 增加独立的版本锁定管理能力，允许用户主动声明关键包的版本约束，而不必等到冲突解决流程才能写入 pin。

面向以下场景：

1. 锁定基础设施包版本（torch、numpy、transformers 等），防止后续插件安装或 `uv lock` 重新求解时意外升降级。
2. 在 `gov init` 降级 Python 后，主动 pin 住已知兼容的关键包版本。
3. 审查当前哪些 pin 生效，以及按需移除不再需要的 pin。

## 2. 现状分析

### 2.1 已有的 overrides 机制

当前 `overrides` dependency group 已经是 pin 的底层载体：

- `pyproject.toml` 模板预定义了 `[dependency-groups] overrides = []`。
- `append_pins_to_overrides_group`（[bin/gov:782](../bin/gov#L782)）通过 `uv add --group overrides --python "$py" --no-sync` 逐条写入 pin。
- `uv lock` 会尊重 `overrides` group 中的版本约束。

### 2.2 当前 pin 只在冲突解决流程中使用

pin 的写入路径目前仅有两条：

1. **插件事务冲突解决**：`gov resolve <txid>`（[bin/gov:3174](../bin/gov#L3174)）— 交互式输入 `pkg==version`。
2. **核心更新冲突解决**：`gov update resolve <txid>`（[bin/gov:3771](../bin/gov#L3771)）— 参数化 `--pin` / `--pins-file`。

两者都将 pin 写入事务记录的 `resolution_pins` 字段，并通过 `append_pins_to_overrides_group` 落盘到 `pyproject.toml`。

### 2.3 缺口

- 没有独立于事务的 pin 管理命令。
- 没有列出当前生效 pin 的方式。
- 没有移除单条 pin 的方式（只能手动编辑 `pyproject.toml`）。
- `overrides` group 中的条目没有来源标注（事务 resolve pin 与手动 pin 混在一起）。

## 3. 设计边界

### 3.1 In Scope

1. `gov pin add <pkg==version> [<pkg==version> ...]` — 添加 pin 到 `overrides` group 并重新 lock + sync。
2. `gov pin list` — 列出当前 `overrides` group 中的所有 pin。
3. `gov pin remove <pkg> [<pkg> ...]` — 从 `overrides` group 中移除指定包的 pin 并重新 lock + sync。

### 3.2 Out Of Scope

1. 不引入 pin 来源归属模型（不区分"手动 pin"与"resolve pin"）。
2. 不引入 pin 锁定强度分级（如 soft pin / hard pin）。
3. 不自动推荐 pin 版本（用户必须显式指定 `==version`）。
4. 不在 `gov pin add` 中运行候选测试（pin 只影响 lock + sync，不走事务流）。

## 4. 命令定义

### 4.1 `gov pin add`

```
gov pin add <spec>... [--no-sync]
```

- `<spec>`：一个或多个 `pkg==version` 格式的版本约束。
- `--no-sync`：只写入 `pyproject.toml` 和 `uv.lock`，不同步 `.venv-prod`。默认执行 lock + sync。
- 验证格式：复用现有正则 `^[A-Za-z0-9_.-]+==[^[:space:]]+$`。
- 操作保护：创建 operation 记录，备份 `pyproject.toml` 和 `uv.lock`，失败时恢复。
- 实现路径：对每个 spec 调用 `uv add --group overrides --python "$py" --no-sync`，然后 `lock_project_exact` + `sync_project_env_exact`。

### 4.2 `gov pin list`

```
gov pin list
```

- 从 `pyproject.toml` 解析 `[dependency-groups] overrides` 的内容并逐行输出。
- 无 pin 时输出提示信息。

### 4.3 `gov pin remove`

```
gov pin remove <pkg>... [--no-sync]
```

- `<pkg>`：一个或多个包名（不带版本号）。
- 从 `overrides` group 中移除匹配的条目。
- 操作保护：与 `pin add` 一致。
- 实现路径：对每个 pkg 调用 `uv remove --group overrides --python "$py" --no-sync`，然后 lock + sync。

## 5. 行为定义

### 5.1 `pin add` 流程

1. `ensure_layout` + `require_python`。
2. 验证所有 spec 格式。
3. `op_begin "pin_add" "$specs"`。
4. 备份 `pyproject.toml` 和 `uv.lock`。
5. 对每个 spec 执行 `uv add --group overrides --python "$py" --no-sync`。
6. `lock_project_exact`。
7. 若非 `--no-sync`，则 `sync_project_env_exact`。
8. `op_finalize` success 或 failure（failure 时恢复备份）。

### 5.2 `pin remove` 流程

1. `ensure_layout` + `require_python`。
2. 验证包名存在于当前 `overrides` group 中。
3. `op_begin "pin_remove" "$pkgs"`。
4. 备份 `pyproject.toml` 和 `uv.lock`。
5. 对每个 pkg 执行 `uv remove --group overrides --python "$py" --no-sync`。
6. `lock_project_exact`。
7. 若非 `--no-sync`，则 `sync_project_env_exact`。
8. `op_finalize` success 或 failure。

### 5.3 失败处理

- lock 失败（如 pin 与其他约束不兼容）：恢复备份的 `pyproject.toml` 和 `uv.lock`，输出冲突信息，`op_finalize` failed。
- sync 失败：恢复备份，`op_finalize` failed。
- 不自动进入 resolve 流程——`gov pin` 是确定性操作，失败意味着用户需要修改 spec。

## 6. 实现锚点

### 6.1 复用的现有 helper

| Helper | 行号 | 用途 |
|--------|------|------|
| `append_pins_to_overrides_group` | [bin/gov:782](../bin/gov#L782) | `pin add` 的核心写入逻辑 |
| `lock_project_exact` | [bin/gov:456](../bin/gov#L456) | lock |
| `sync_project_env_exact` | [bin/gov:468](../bin/gov#L468) | sync |
| `configured_python` | bin/gov | 获取当前 Python 版本 |
| `op_begin` / `op_finalize` | bin/gov | 操作记录与回滚 |
| pin 格式验证正则 | `^[A-Za-z0-9_.-]+==[^[:space:]]+$` | 格式检查 |

### 6.2 需要新增的代码

1. `cmd_pin_add` — `pin add` 命令处理函数。
2. `cmd_pin_list` — 从 `pyproject.toml` 解析 `overrides` group。
3. `cmd_pin_remove` — `pin remove` 命令处理函数。
4. 命令分发入口（[bin/gov:4464](../bin/gov#L4464) `main` case 块）：
   ```bash
   pin) shift; case "${1:-}" in
       add) shift; cmd_pin_add "$@" ;;
       list) shift; cmd_pin_list "$@" ;;
       remove) shift; cmd_pin_remove "$@" ;;
       *) echo "Usage: gov pin {add|list|remove} ..." >&2; exit 1 ;;
   esac ;;
   ```
5. `cmd_help` 中添加用法行（[bin/gov:4424](../bin/gov#L4424)）。

### 6.3 `pin list` 解析方式

从 `pyproject.toml` 提取 `overrides` group 内容。建议用内联 Python 解析 TOML，与现有代码风格一致（项目中已有多处 `"${PYTHON_BIN}" - <<'PY'` 模式）：

```python
import tomllib, sys
with open(sys.argv[1], "rb") as f:
    data = tomllib.load(f)
for pin in data.get("dependency-groups", {}).get("overrides", []):
    print(pin)
```

## 7. 对现有架构的影响

### 7.1 不变量遵守情况

| 架构不变量 | 影响 |
|------------|------|
| I1: 所有生产依赖变更先锚定本地真相 | ✅ `pin add/remove` 先改 `pyproject.toml` 再 lock + sync |
| I3: 破坏性操作需备份 | ✅ 通过 `op_begin/op_finalize` 保护 |
| I6: `pyproject.toml` 是依赖权威 | ✅ pin 写入 `overrides` group |

### 7.2 与事务 resolve pin 的关系

- `gov pin add` 与 `gov resolve` / `gov update resolve` 写入同一个 `overrides` group。
- 两者写入的 pin 不区分来源，这是 v1 的有意简化。
- `gov pin list` 会列出所有 pin，无论来源。

## 8. 测试计划

### 8.1 成功路径

1. `gov pin add numpy==1.26.4` — pin 出现在 `pyproject.toml overrides`，lock 和 venv 中版本一致。
2. `gov pin list` — 输出当前 pin。
3. `gov pin remove numpy` — pin 从 `overrides` 中消失，lock 重新求解。
4. 多包操作：`gov pin add torch==2.9.0 numpy==1.26.4` 一次添加多个。

### 8.2 失败路径

1. 格式错误的 spec（如 `numpy>=1.26`）应被拒绝。
2. 与已有约束不兼容的 pin 应 lock 失败并恢复。
3. remove 不存在的包名应提示。

### 8.3 边界路径

1. 重复 `pin add` 相同包不同版本 — `uv add` 会覆盖旧版本。
2. `pin add --no-sync` 只改 lock 不改 venv。
3. 与 `gov resolve` 写入的 pin 共存。

## 9. 后续需同步的文档

1. `docs/04_cli_reference.md`
2. `docs/10_quick_start_for_llm.md`
3. `dev-docs/application-core/contracts.md`
4. `dev-docs/application-core/spec.md`（新增 KF 条目）
5. `bin/gov` 内 `cmd_help` 帮助文本

## 10. 建议新会话的起手顺序

1. 读 `dev-docs/architecture-haiku.md` 建立全局心智模型。
2. 读本文件确认设计边界与行为定义。
3. 读 `bin/gov` 中以下锚点：
   - `append_pins_to_overrides_group`（行 782）
   - `lock_project_exact` / `sync_project_env_exact`（行 456 / 468）
   - `op_begin` / `op_finalize`（搜索定义）
   - `cmd_update_resolve`（行 3771）作为参数化 pin 写入的参考实现
   - `main` 分发块（行 4464）
4. 实现 `cmd_pin_add` → `cmd_pin_list` → `cmd_pin_remove`，按此顺序。
5. 补充测试到 `tests/test_gov_cli.sh`。
6. 同步 §9 中列出的文档。
