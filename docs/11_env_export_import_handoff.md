# 11 环境导出导入 Handoff

## 1. 目标

为 `comfy_env` 增加生产环境导出/导入能力，面向以下场景：

1. 在本地较低成本工作站完成 ComfyUI 生产依赖与自定义节点验证。
2. 将已验证的环境快速迁移到高成本 GPU 云端服务器。
3. 避免在迁移时重新走插件逐个接入、事务观察、人工补 pin 的长流程。

## 2. 修正后的结论

### 2.1 依赖迁移结论

1. 之前把 uv 锁文件跨环境迁移能力说得过于保守，需要修正。
2. `uv.lock` 是 uv 的 cross-platform / universal lockfile；`uv export --format pylock.toml` 是官方支持的标准导出路径。
3. 因此，Python 依赖层的整体迁移应以锁文件为中心，不需要导出 `.venv-prod` 本体。
4. 但导出时不应默认重新解析依赖；迁移交付物应尽量绑定到“已经验证过的那份锁定状态”。
5. 结论：导出时优先使用 `uv export --format pylock.toml --locked --all-groups`，而不是裸跑 `uv export`。

### 2.2 增量导入结论

1. v1 不应支持“按单节点做依赖增量导入”。
2. 原因不在 uv 锁文件，而在 `comfy_env` 当前本地真相模型：
   - 依赖权威仍是 `pyproject.toml` 与 `uv.lock`
   - `plugins.json` 只是注册元数据，不是跨环境依赖真相
   - `dependency-groups.overrides` 是共享组，没有“某条 override pin 属于哪个节点”的归属模型
3. 因此，当前设计下唯一可靠的导入方式是“整体导入全部依赖 + 全部节点源码快照”。
4. 未来若要做增量，只建议考虑“节点源码增量同步”，不建议在现有模型上做“节点依赖增量导入”。

## 3. v1 设计边界

### 3.1 In Scope

1. 导出当前已验证生产环境所需的最小完整交付物。
2. 在目标机器上整体恢复：
   - Python 依赖锁定状态
   - `custom_nodes` 源码快照
   - 插件注册元数据
3. 导入后重建 `.venv-prod` 并做基础校验。

### 3.2 Out Of Scope

1. 不导出 `.venv-prod` 目录本体。
2. 不支持按单节点依赖增量导入。
3. 不支持“把 bundle 合并进一个未知状态的既有环境”。
4. 不在 v1 自动处理跨 OS / 跨架构 / 跨 Python ABI 的兼容修复。

## 4. 当前 bundle 结构

当前实现只支持 `.tar` bundle，archive 内固定只有一个顶层 `bundle/` 目录，结构如下：

```text
bundle.tar
└── bundle/
    ├── manifest.json
    ├── pylock.toml
    ├── uv.lock
    ├── pyproject.toml
    ├── state/
    │   └── plugins.json
    ├── custom_nodes/
    │   ├── <node_id_1>/
    │   └── <node_id_2>/
    └── audit/
        ├── prod-freeze.txt
        └── export-summary.json
```

说明：

1. `manifest.json`：
   - 记录 bundle 完整性与兼容性元数据。
   - 当前至少包含：`created_at`、`created_by`、`python_spec`、`python_version`、`platform_system`、`platform_machine`、`bundle_format_version`、`node_ids`、`files.sha256`。
2. `pylock.toml`：
   - 用于标准化迁移与目标端安装。
   - 生成命令草案：`uv export --format pylock.toml --locked --all-groups --output-file pylock.toml`
3. `uv.lock`：
   - 保留 uv 原生锁文件，作为本地真相和额外保真信息。
   - 不用它替代 `pylock.toml`，而是和 `pylock.toml` 一起导出。
4. `pyproject.toml`：
   - 保留 dependency groups 真相。
5. `state/plugins.json`：
   - 用于恢复节点注册元数据。
   - 明确它不是依赖真相，只是导入后辅助恢复节点记录。
6. `custom_nodes/<node_id>/`：
   - 保存当前工作站上的节点运行时源码快照。
   - 保留工作树中的未提交修改与未跟踪文件。
   - 过滤 `.git/` 目录与 `.git` 指针文件，不导出 Git 管理元数据。
   - 不依赖远端 `git_url/ref` 在云端重新获取。
7. `audit/prod-freeze.txt`：
   - 用 `uv pip freeze` 导出，只用于审计和导入后比对，不作为导入真相。
8. `audit/export-summary.json`：
   - 记录导出摘要，如 `python_spec`、`node_ids`、`node_count` 及关键交付物是否存在。

## 5. 当前命令

### 5.1 已实现命令

1. `./bin/gov env export <output_tar>`
2. `./bin/gov env import <bundle_tar> --comfyui-dir <abs-path> --python <python-spec>`

### 5.2 可选后续命令

1. `./bin/gov env inspect <bundle_path>`
2. `./bin/gov env verify <bundle_path>`

v1 先不做 `nodes-only import` 或 `partial import`。

## 6. 行为定义

### 6.1 Export

1. `ensure_layout` 后执行。
2. 检查以下前置条件：
   - `pyproject.toml` 与 `uv.lock` 存在
   - `state/plugins.json` 存在
   - `config.toml` 存在且 `paths.comfyui_dir` 可解析
3. 仅接受不存在的 `.tar` 输出路径；父目录必须已存在。
4. 用 `--locked` 导出 `pylock.toml`；若 lock 已过期则直接失败，不自动刷新。
5. 从 `state/plugins.json` 枚举已注册节点，并把每个 `install_relpath` 对应目录导出为运行时快照；保留未提交修改与未跟踪文件，但排除 `.git` 元数据。
6. 若 registry 中任一节点的 `install_relpath` 缺失、越界或目标目录不存在，则 export 直接失败。
7. 导出 `prod-freeze.txt`，并写出 `audit/export-summary.json`。
8. 生成 `manifest.json`，至少包含：
   - `created_at`
   - `created_by`
   - `python_spec`
   - `python_version`
   - `platform_system`
   - `platform_machine`
   - `bundle_format_version`
   - `node_ids`
   - `files.sha256`
9. 输出 bundle 路径和摘要信息。
10. bundle 中导出的 `pyproject.toml` 应已收紧到单一 Python minor 与单一平台；导入侧以它作为兼容性判定依据。

### 6.2 Import

1. 导入默认按 bundle 对目标机做精确恢复，不区分“空白目标”和“允许覆盖”两种模式。
2. 只支持 `.tar` bundle；archive 中必须只有一个顶层 `bundle/` 目录。
3. 先安全解包 `bundle.tar` 到 staging 目录，再校验 `manifest.json` 与关键文件完整性；校验失败即退出，不提前改写 root truth。
4. `--python` 若传入纯 `major.minor` 则直接使用；若传入 patch 版本或解释器选择器，则先通过 `uv python find --no-python-downloads` 解析目标机解释器，再规范化为 minor 线。
5. 先校验目标机 Python minor 线是否与 bundle `requires-python` 兼容，并校验当前主机是否落在 bundle `[tool.uv].environments` 内。
6. 导入前创建 `env_import` operation，备份 `config.toml` 与受影响的 `custom_nodes` 目录。
7. 将 `pyproject.toml`、`uv.lock`、`state/plugins.json` 复制到 staging workdir，并执行 `uv lock --check` 等效校验。
8. staging 通过后，先用 bundle truth 重建 `.venv-prod`，再按 bundle 恢复 `custom_nodes` 运行态快照。
9. 只有 staging 与 prod sync 成功后，才把 `pyproject.toml`、`uv.lock`、`state/plugins.json` 切回 root，并用目标机 CLI 参数更新 `config.toml`。
10. 导入后做 smoke test，并输出导入摘要。
11. 任一步失败都会恢复 root truth，并恢复本次导入覆盖或清理过的 `custom_nodes` 目录。

## 7. 实现锚点

### 7.1 关键入口

当前实现主要入口：

1. `cmd_env_export`
2. `cmd_env_import`
3. `resolve_python_minor`
4. `bundle_check_runtime_compatibility`
5. `bundle_lock_check`
6. `sync_project_env_exact`
7. `run_smoke_test_in_env`

### 7.2 关键 helper

当前与 handoff 直接相关的 helper：

1. `bundle_manifest_write`
2. `bundle_manifest_verify`
3. `bundle_tar_create`
4. `bundle_tar_extract`
5. `bundle_copy_custom_nodes`
6. `bundle_stage_truth`
7. `bundle_restore_plugins_registry`
8. `bundle_export_pylock`
9. `bundle_backup_custom_nodes`
10. `bundle_apply_custom_nodes`
11. `env_import_restore_after_failure`

### 7.3 导入路径现状

1. 没有复用 `node add`，因为导入不是重新从 Git 安装节点。
2. 没有复用插件事务流，因为导入目标是快速恢复已验证环境，而不是重新观察每个节点影响。
3. 当前实现采用单独的“受保护整体恢复路径”，语义是：
   - verify
   - staging
   - exact sync
   - custom_nodes restore
   - smoke test
   - commit / rollback

## 8. 后续维护时需要同步的文档

后续若继续调整 handoff 行为，至少同步更新：

1. `docs/04_cli_reference.md`
2. `docs/05_data_contracts.md`
3. `docs/06_development_workflow.md`
4. `dev-docs/application-core/contracts.md`
5. `dev-docs/data/spec.md`
6. `dev-docs/subsystems/dependency-sync/spec.md`
7. `dev-docs/subsystems/source-integration/spec.md`

## 9. 测试计划

### 9.1 成功路径

1. 在 fixture 环境导出 bundle。
2. 在干净目标目录导入 bundle。
3. 校验：
   - `pyproject.toml` 已恢复
   - `uv.lock` 已恢复
   - `state/plugins.json` 已恢复
   - `custom_nodes` 节点目录存在
   - `.venv-prod` 可启动并通过 smoke test

### 9.2 失败路径

1. `uv.lock` 过期时 export 应失败，不自动 re-lock。
2. bundle 缺文件或 hash 不一致时 import 应失败。
3. 依赖同步失败时应回滚 root truth，不留下半完成状态。
4. 若 import 已清理 bundle 外节点目录，失败回滚时应恢复这些目录。

### 9.3 边界路径

1. `plugins.json` 中存在节点，但源码目录缺失时 export 应阻断。
2. bundle 含共享 `overrides` 时整体导入成功。
3. 试图只恢复单节点依赖时应明确返回“不支持”。

## 10. 风险与默认决策

### 10.1 已确认的风险

1. 当前 `dependency-groups.overrides` 无节点归属模型，所以不能可靠做节点依赖级增量导入。
2. `plugins.json` 只是注册缓存，不应被误当成依赖权威。
3. 导入到异构平台时，锁文件能否完全适配取决于目标环境是否落在其兼容集合中。

### 10.2 默认决策

1. v1 采用整体导入，不做依赖增量导入。
2. v1 导出节点源码快照，不依赖导入端联网拉取 Git。
3. v1 导出 `pylock.toml` 时使用 `--locked`，不允许导出时 re-lock。
4. v1 保留 `uv.lock` 和 `pylock.toml` 两份锁定表示。
5. v1 导入默认就是 exact restore，可视为显式覆盖目标环境。

## 11. 建议新会话的起手顺序

1. 先读 `dev-docs/architecture-haiku.md`。
2. 再读本文件，确认修正后的结论与边界。
3. 阅读当前依赖与状态相关实现：
   - [bin/gov](/home/windy/comfy-hub/comfy_env/bin/gov)
   - [dev-docs/data/spec.md](/home/windy/comfy-hub/comfy_env/dev-docs/data/spec.md)
   - [dev-docs/subsystems/dependency-sync/spec.md](/home/windy/comfy-hub/comfy_env/dev-docs/subsystems/dependency-sync/spec.md)
   - [dev-docs/subsystems/source-integration/spec.md](/home/windy/comfy-hub/comfy_env/dev-docs/subsystems/source-integration/spec.md)
4. 优先核对 `tests/test_gov_cli.sh` 中关于 runtime snapshot、`.git` 过滤和 exact restore 的覆盖场景。
5. 修改实现后，再同步 CLI 文档、契约文档和测试。
