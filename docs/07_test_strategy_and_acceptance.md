# 07 测试策略与验收矩阵

## 1. 测试分层

1. 静态检查：命令路由、函数存在、文件结构。
2. 集成测试：真实 `uv` 下命令行为。
3. 端到端测试：`tx run -> (resolve) -> promote -> remove -> rollback`。

## 2. 测试准备

最小 ComfyUI 模拟：

```bash
mkdir -p /tmp/comfyui/custom_nodes
cat >/tmp/comfyui/main.py <<'PY'
print("fake comfyui start")
PY
```

配置 `comfy_env/config.toml` 的 `paths.comfyui_dir` 指向 `/tmp/comfyui`。

## 3. 核心验收矩阵

## 3.1 Batch-1 回归

1. `init` 可创建 prod 与 lock。
2. `tx run` 产出事务 JSON。
3. `tx inspect` 返回稳定摘要。
4. `tx abort` 正确清理 candidate。

## 3.2 Batch-2（resolve/promote）

1. Promote 成功路径：状态到 `promoted`，并产出 pre/post snapshot。
2. Promote 冲突路径：状态到 `needs_resolution`，冲突报告存在。
3. Resolve 路径：输入 pin 后可转 `resolved`。
4. Core gate：未 `--approve-core` 时阻断。

## 3.3 Batch-3（remove/rollback）

1. Remove 路径：生成 pre-remove snapshot，prod exact sync 成功。
2. Rollback 路径：恢复真相并重建 prod。
3. Snapshot retention：>50 时仅保留最新 50。

## 3.4 故障恢复验证

1. Promote 期间故意制造 sync 失败，验证自动恢复 pre-promote snapshot。
2. Rollback 期间故意制造 sync 失败，验证恢复 rollback_pre。

## 4. 结果判定

1. 命令退出码符合预期（成功=0，失败!=0）。
2. 状态文件字段与状态流转一致。
3. 真相文件与 snapshot 一致可重放。

## 5. 建议自动化

1. 增加 shell 测试脚本目录（后续）。
2. 增加 `gov status --json` 便于断言（后续）。
