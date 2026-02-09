# 08 故障排查与已知限制

## 1. 常见报错与处理

## 1.1 `python3/python is required`

1. 安装 Python 3.10+。
2. 确认 PATH 包含 `python3` 或 `python`。

## 1.2 `command not found: uv`

1. 安装 uv。
2. 确认 PATH。

## 1.3 `transaction not found`

1. 检查 `state/transactions/` 是否存在对应 txid。
2. 用 `gov status` 查看最近事务。

## 1.4 Promote 总是 `needs_resolution`

1. 查看 `state/conflicts/<txid>.json`。
2. 用 `gov resolve <txid>` 输入精确 pin。
3. 再次 `gov tx promote`。

## 1.5 Rollback 后 sync 失败

1. 检查网络/索引可达性。
2. 检查 snapshot 内 lock 文件是否有效。
3. 恢复网络后重试 rollback。

## 2. 诊断顺序

1. 先看退出码和 stderr。
2. 再看 `state/logs` 和 `state/conflicts`。
3. 再看 tx/snapshot JSON 是否结构完整。
4. 最后再人工干预真相文件。

## 3. 已知限制

1. `gov run` 仍为占位。
2. promotion plan 基于启发式（requirements + diff），不是完备语义分析。
3. conflict report 是摘要，不保证精确根因定位。
4. 验收平台以 Linux 为主，Windows 仅开发辅助。

## 4. 紧急恢复建议

1. `gov snapshot list`
2. 选择最近稳定快照执行 `gov rollback <snapshot_id>`
3. 若回滚失败，优先恢复网络/索引，再重试。
