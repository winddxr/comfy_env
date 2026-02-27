# 03 运行与操作手册

## 1. 初始化

```bash
./bin/gov init
./bin/gov status
```

说明：

1. `gov init` 会检查 `pyproject.toml`；若缺失则自动从 `pyproject.toml.template` 复制生成。
2. `config.toml` 为本地用户态配置文件，不进入 Git；默认参考 `config.toml.template`。

## 2. 新插件接入

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

## 3. 插件卸载与 GC

```bash
./bin/gov node remove <node_id>
./bin/gov node remove <node_id> --purge-code
```

## 4. 操作审计与撤销

```bash
./bin/gov op list
./bin/gov op inspect <op_id>
./bin/gov undo <op_id>
```

## 5. 生产启动与停止

```bash
./bin/gov run [--sync] [-- <comfyui_args...>]
./bin/gov stop
```

说明：
1. `gov run` 使用 `.venv-prod` 环境启动 ComfyUI。
2. `--sync` 可选，确保启动前强制做一次依赖同步。
3. `--` 后的所有参数均透传给 ComfyUI 本身，例如：`./bin/gov run -- --listen 0.0.0.0 --port 8188`。

## 6. 运行约束

1. `undo` 需要哈希一致性，不满足会阻断。
2. `plugins.json` 为本地状态，不应作为跨环境真相。
3. `managed_deps` 仅缓存，卸载权威是 `pyproject` 的 group。
