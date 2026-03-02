# Comfy Environment Governance (`comfy_env`)

ComfyUI 依赖治理系统。
完全切换到切换到的高速UV并且具备更好缓存策略的依赖管理，同时并且不破坏项目本体pyptoject.toml避免每次git时候的合并操作。支持本体和插件的依赖事务管理，可以精确回滚，保证不炸环境。放心升级和折腾，
现在同时覆盖两类依赖面：

1. 插件依赖：继续使用 `tx run -> tx promote -> undo`
2. ComfyUI 本地核心依赖：使用 `init -> install torch -> install` 完成首装，后续使用 `update run -> update promote -> undo`

面向AI使用和coding优化。

## 0. 如果你是 AI

1. 开发或审计时，阅读 `dev-docs/architecture-haiku.md`，把它当作唯一入口导航。
2. 使用时，阅读：`docs/04_cli_reference.md`
## 1. 项目目标

1. 可复现：本地 `pyproject.toml + uv.lock` 可重建依赖环境。
2. 可审计：插件事务、核心依赖升级、操作备份都有状态与日志。
3. 可恢复：`install/promote/remove/update promote` 失败自动回退；成功操作可 `undo`。
4. 可治理：核心包变更需要显式批准。

## 2. 命令总览

1. `./bin/gov init --comfyui-dir <abs-path> --python <python-spec>`
2. `./bin/gov install torch --index-url <url>`
3. `./bin/gov install [--requirements-file <path>]`
4. `./bin/gov update run [--requirements-file <path>] [--timeout <seconds>]`
5. `./bin/gov update inspect <txid>`
6. `./bin/gov update resolve <txid> [--pin <pkg==version>]... [--pins-file <path>]`
7. `./bin/gov update promote <txid> [--approve-core --reason "..."] [--allow-failed-run]`
8. `./bin/gov update abort <txid>`
9. `./bin/gov status`
10. `./bin/gov node add <git_url> [--ref <sha/tag>] [--id <node_id>]`
11. `./bin/gov node remove <node_id> [--purge-code]`
12. `./bin/gov tx run <node_id> [--timeout <seconds>]`
13. `./bin/gov tx inspect <txid>`
14. `./bin/gov tx abort <txid>`
15. `./bin/gov tx promote <txid> [--approve-core --reason "..."] [--allow-failed-run]`
16. `./bin/gov resolve <txid>`
17. `./bin/gov op list`
18. `./bin/gov op inspect <op_id>`
19. `./bin/gov undo <op_id>`
20. `./bin/gov run [--sync] [-- <args...>]`
21. `./bin/gov stop`

## 3. 目录结构（侧车模式）

```text
TopDir/
├── ComfyUI/
│   ├── main.py
│   ├── requirements.txt
│   └── custom_nodes/
└── comfy_env/
    ├── bin/gov
    ├── config.toml.template
    ├── pyproject.toml.template
    ├── config.toml
    ├── pyproject.toml
    ├── uv.lock
    ├── .venv-prod/
    ├── .venv-candidate/
    ├── state/
    │   ├── plugins.json
    │   ├── ops/
    │   ├── transactions/
    │   ├── conflicts/
    │   ├── logs/
    │   └── work/
    ├── dev-docs/
    └── docs/
```

## 4. 首次安装（新顺序）

```bash
./bin/gov init --comfyui-dir /abs/path/to/ComfyUI --python 3.12
./bin/gov install torch --index-url https://download.pytorch.org/whl/cu130
./bin/gov install
```

说明：

1. `init` 负责写入 `config.toml` 的 `paths.comfyui_dir` 与 `runtime.python`，并初始化最小 prod 环境。
2. `install torch` 把 `torch/torchvision/torchaudio` 收敛到 `dependency-groups.torch`，并记录可撤销的 operation。
3. `install` 从 `${comfyui_dir}/requirements.txt` 导入基础依赖到 `dependency-groups.core`。
4. `install` 在 torch 未先安装时会阻断。

## 5. 后续升级（核心依赖事务）

```bash
./bin/gov update run
./bin/gov update inspect <txid>
./bin/gov update resolve <txid> --pin pkg==1.2.3
./bin/gov update promote <txid>
```

说明：

1. `update run` 读取 `requirements.txt`，在 staged workdir 构造候选真相，再同步到 candidate 环境观察。
2. `update promote` 只晋升该事务已经固定下来的 staged snapshot，不会重新读取当前 `requirements.txt`。
3. promote 失败会回滚，成功的 `update_promote` operation 可用 `undo` 撤销。

## 6. 插件事务（保持不变）

```bash
./bin/gov node add <git_url> [--id <node_id>]
./bin/gov tx run <node_id>
./bin/gov tx inspect <txid>
./bin/gov tx promote <txid>
```

冲突时：

```bash
./bin/gov resolve <txid>
./bin/gov tx promote <txid>
```

## 7. 撤销与状态

```bash
./bin/gov status
./bin/gov op list
./bin/gov undo <op_id>
```

`status` 现在会额外显示：

1. `config_ready`
2. `comfyui_dir`
3. `python`
4. `torch_ready`
5. `core_ready`
6. `update_transactions_pending`

## 8. 文档导航

1. `dev-docs/architecture-haiku.md`
2. `docs/04_cli_reference.md`
3. `docs/10_quick_start_for_llm.md`
