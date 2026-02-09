# 04 CLI 命令参考

> 以下为 `comfy_env/bin/gov` 当前实现对应的命令语义。

## 1. 基础命令

## 1.1 `gov init`

用途：初始化布局、锁定依赖并创建 prod 环境。  
失败码：非 0 表示 lock 或 sync 失败。

## 1.2 `gov status`

用途：查看 prod、lock、事务总量与待处理事务。

## 2. 节点命令

## 2.1 `gov node add <git_url> [--ref <sha/tag>] [--id <node_id>]`

用途：克隆插件并写入 `plugins.json`。

## 2.2 `gov node remove <node_id> [--purge-code]`

用途：移除节点依赖组并触发 GC；可选删除插件目录。

## 3. 事务命令

## 3.1 `gov tx run <node_id> [--timeout <seconds>]`

用途：在 candidate 执行 ComfyUI 并记录事务。

## 3.2 `gov tx inspect <txid>`

用途：输出事务摘要、差异数量、核心影响、日志路径。

## 3.3 `gov tx abort <txid>`

用途：删除 candidate 并把事务标记为 `aborted`。

## 3.4 `gov tx promote <txid> [--approve-core --reason "..."] [--allow-failed-run]`

用途：尝试晋升事务到 Prod。  
关键参数：

1. `--approve-core`：允许核心包影响事务继续晋升。
2. `--reason`：记录审批原因（建议必填）。
3. `--allow-failed-run`：允许 `failed` 状态事务继续尝试晋升。

## 4. 冲突命令

## 4.1 `gov resolve <txid>`

用途：交互输入版本 pin，重试 lock。  
输入格式：`pkg==version`，空行结束。

## 5. 快照与回滚命令

## 5.1 `gov snapshot list`

用途：列出快照索引。

## 5.2 `gov snapshot inspect <snapshot_id>`

用途：查看快照 `meta.json`。

## 5.3 `gov rollback <snapshot_id>`

用途：恢复真相文件并重建 prod。

## 6. 占位命令

## 6.1 `gov run`

当前为占位：返回非 0 并提示未完成启动编排。

## 7. 错误处理约定

1. 参数错误返回非 0，并输出 `Usage`。
2. 找不到事务/快照返回非 0。
3. destructive 操作失败时优先尝试恢复前置快照。
