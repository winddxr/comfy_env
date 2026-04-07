# 10 给接手 LLM 的快速上手

## 1. 先建立心智模型

1. 依赖真相是本地文件：`pyproject.toml` 和 `uv.lock`。
2. `config.toml` 现在还承载运行前置条件：`paths.comfyui_dir` 与 `runtime.python`。
3. `dependency-groups.torch` 和 `dependency-groups.core` 分别治理 torch 与 ComfyUI 基础依赖。
4. 插件事务和核心依赖升级事务共用 `state/transactions/`，但通过 `kind` 区分。
5. 破坏性变更仍然依赖 `operation backup + undo`。
6. 环境交付 bundle 只是运输载体；导入后的本地依赖真相仍然是 `pyproject.toml + uv.lock`。

## 2. 首次落地（新的最小顺序）

```bash
./bin/gov init --comfyui-dir /abs/path/to/ComfyUI --python 3.12
./bin/gov install torch --index-url https://download.pytorch.org/whl/cu130
./bin/gov install
```

说明：

1. `init` 首次执行时必须显式传 `--comfyui-dir` 和 `--python`。
2. 推荐把 `--python` 直接写成 minor 线，如 `3.12`；若传入 `3.12.8` 或解释器选择器，CLI 会先用 `uv python find --no-python-downloads` 解析到本机解释器，再回写为 `3.12`。
3. `install torch` 必须先于 `install`。
4. `install` 默认读取 `${comfyui_dir}/requirements.txt`。

## 3. 核心目录与用途

1. `bin/gov`：主 CLI。
2. `state/transactions/`：插件事务和 `kind=core_update` 核心升级事务。
3. `state/ops/`：`install/install torch/promote/remove/update promote/undo` 的操作元数据与备份。
4. `state/conflicts/`：lock 冲突报告。
5. `state/work/`：staged workdir。

## 4. 命令速查

### 4.1 环境与首装

1. `./bin/gov init --comfyui-dir <abs-path> --python <python-spec>`
2. `./bin/gov install torch --index-url <url>`
3. `./bin/gov install [--requirements-file <path>]`

### 4.2 核心依赖升级事务

1. `./bin/gov update run [--requirements-file <path>] [--timeout <seconds>]`
2. `./bin/gov update inspect <txid>`
3. `./bin/gov update resolve <txid> [--pin <pkg==version>]... [--pins-file <path>]`
4. `./bin/gov update promote <txid> [--approve-core --reason "..."] [--allow-failed-run]`
5. `./bin/gov update abort <txid>`

### 4.3 插件治理（保持原有路径）

1. `./bin/gov node add <git_url> [--id <node_id>]`
2. `./bin/gov tx run <node_id>`
3. `./bin/gov tx inspect <txid>`
4. `./bin/gov tx promote <txid>`
5. `./bin/gov resolve <txid>`

### 4.4 环境交付

1. `./bin/gov env export <output_dir>`
2. `./bin/gov env import <bundle_dir> --comfyui-dir <abs-path> --python <python-spec>`

### 4.5 审计与运行

1. `./bin/gov status`
2. `./bin/gov op list`
3. `./bin/gov op inspect <op_id>`
4. `./bin/gov undo <op_id>`
5. `./bin/gov run [--sync] [-- <args...>]`
6. `./bin/gov stop`

## 5. 推荐工作流

### 5.1 首次安装

```bash
./bin/gov init --comfyui-dir /abs/path/to/ComfyUI --python 3.12
./bin/gov install torch --index-url https://download.pytorch.org/whl/cu130
./bin/gov install
```

### 5.2 核心依赖升级

```bash
./bin/gov update run
./bin/gov update inspect <txid>
./bin/gov update resolve <txid> --pin pkg==1.2.3
./bin/gov update promote <txid>
```

### 5.3 插件接入

```bash
./bin/gov node add <git_url> [--id <node_id>]
./bin/gov tx run <node_id>
./bin/gov tx promote <txid>
```

### 5.4 环境交付

```bash
./bin/gov env export /abs/path/to/bundle-dir
./bin/gov env import /abs/path/to/bundle-dir --comfyui-dir /abs/path/to/ComfyUI --python 3.12
```

说明：

1. `env export` 导出的是 `custom_nodes` 运行态源码快照，不依赖导入端重新拉 Git。
2. 快照会保留工作树中的已修改文件和未跟踪文件，但不会把 `.git` 元数据带进 bundle。
3. `env import` 默认是 exact restore，会清理 bundle 外的 `custom_nodes/*` 目录。

## 6. 接手时最小核查清单

1. `./bin/gov help` 包含 `install` 和 `update` 命令族。
2. `./bin/gov status` 能输出 `config_ready/python/torch_ready/core_ready`。
3. `pyproject.toml.template` 包含 `core`、`torch`、`overrides` 三个固定组。
4. 若要做环境迁移，优先核对 `docs/04_cli_reference.md` 与 `docs/11_env_export_import_handoff.md` 中的 bundle/exact restore 语义。
