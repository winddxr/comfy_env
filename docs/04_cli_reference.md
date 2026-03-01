# 04 CLI 命令参考

> 以下为 `comfy_env/bin/gov` 当前实现语义（Batch-4）。

## 1. 首装与核心依赖

### `gov init --comfyui-dir <abs-path> --python <python-spec>`

用途：幂等初始化本地治理配置，写入 `paths.comfyui_dir` 和 `runtime.python`，并创建/同步最小 prod 环境。

关键行为：

1. 首次初始化时，两个参数都必填。
2. 已有 `config.toml` 时，未传参数沿用现值，传入参数覆盖现值。
3. 若 `pyproject.toml` 缺失，则从 `pyproject.toml.template` 生成。

### `gov install torch --index-url <url>`

用途：先安装受治理的 `torch/torchvision/torchaudio`，写入 `dependency-groups.torch`。

关键行为：

1. 内部使用 `uv add --group torch ... --index <derived-name>=<url>`。
2. 成功后会同步 prod，并执行 `import torch, torchvision, torchaudio` smoke test。
3. 会生成可撤销的 operation。

### `gov install [--requirements-file <path>]`

用途：把 ComfyUI 核心依赖导入到 `dependency-groups.core` 并同步 prod。

关键行为：

1. 默认读取 `${paths.comfyui_dir}/requirements.txt`。
2. 若 torch 还未通过 `gov install torch` 安装，会直接阻断。
3. 会过滤 `torch`、`torchvision`、`torchaudio`，避免双重权威。
4. 会生成可撤销的 operation。

## 2. 核心依赖升级事务

### `gov update run [--requirements-file <path>] [--timeout <seconds>]`

用途：为核心依赖升级创建独立事务，写入 `kind=core_update` 记录。

### `gov update inspect <txid>`

用途：查看核心依赖升级事务摘要，包括 requirements 来源、staged workdir、diff、冲突和日志。

### `gov update resolve <txid> [--pin <pkg==version>]... [--pins-file <path>]`

用途：用参数化 pin 修复 `update run` 或 `update promote` 的 lock 冲突。

### `gov update promote <txid> [--approve-core --reason "..."] [--allow-failed-run]`

用途：把核心依赖升级事务的 staged snapshot 晋升到 prod。

### `gov update abort <txid>`

用途：删除该核心升级事务的 candidate env 与 staged workdir，并标记为 `aborted`。

## 3. 插件事务

### `gov node add <git_url> [--ref <sha/tag>] [--id <node_id>]`

用途：克隆插件并写入 `state/plugins.json`。

### `gov node remove <node_id> [--purge-code]`

用途：移除插件注册与依赖组，并执行依赖 GC。

### `gov tx run <node_id> [--timeout <seconds>]`

用途：在 candidate 环境运行 ComfyUI 并记录插件事务。

### `gov tx inspect <txid>`

用途：查看插件事务摘要。

### `gov tx abort <txid>`

用途：删除插件事务 candidate env 并标记为 `aborted`。

### `gov tx promote <txid> [--approve-core --reason "..."] [--allow-failed-run]`

用途：把插件事务晋升到 prod。

### `gov resolve <txid>`

用途：交互输入 `pkg==version`，修复插件事务冲突并重试 lock。

## 4. 操作与运行时

### `gov op list`

用途：列出操作记录。

### `gov op inspect <op_id>`

用途：查看 `state/ops/<op_id>/meta.json`。

### `gov undo <op_id>`

用途：撤销指定成功 operation。

### `gov status`

用途：查看当前配置、prod 环境、事务总量和待处理升级事务。

### `gov run [--sync] [-- <args...>]`

用途：在 `.venv-prod` 中启动 ComfyUI。

### `gov stop`

用途：停止由 `gov run` 启动的 ComfyUI。
