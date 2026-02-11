# 08 故障排查与限制

## 1. 常见故障

### 1.1 Promote lock 冲突

1. 查看 `state/conflicts/<txid>.json`。
2. 执行 `gov resolve <txid>` 输入 pin。
3. 再次 `gov tx promote <txid>`。

### 1.2 Promote/Remove sync 失败

1. 命令会自动恢复 pre-op backup。
2. 先检查网络、索引、镜像源。
3. 修复后重试原命令。

### 1.3 Undo 被阻断

1. 错误通常是哈希不一致（当前状态已偏离目标 op 的 post 状态）。
2. 用 `gov op inspect <op_id>` 对照 `files.*.post_sha256`。
3. 确认后选择新的撤销路径，避免强制覆盖。

## 2. 排查顺序

1. 看命令退出码与 stderr。
2. 看 `state/logs/` 与 `state/conflicts/`。
3. 看 `state/transactions/<txid>.json`。
4. 看 `state/ops/<op_id>/meta.json`。
