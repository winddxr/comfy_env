# 13 事务逻辑总览与 Rust 分支文档审计

## 1. 文档目标

这份文档只总结 **与具体实现语言无关** 的事务逻辑契约，目的是把近期几次修正里真正稳定的行为抽出来，作为 Bash 版与 Rust 版都应共同遵守的业务规则。

本文不讨论：

1. Bash / Python / Rust 的具体实现技巧。
2. 某个 helper 的内部写法。
3. 某个测试脚本如何 mock。

本文讨论：

1. `tx run` / `update run` 的观察语义。
2. `tx promote` / `update promote` 的晋升与清理语义。
3. candidate env、staged workdir、日志、状态机这些审计对象的生命周期。
4. 这些规则在 `origin/feat/rust-rewrite` 文档中是否已经完整表达。

## 2. 统一术语

1. `candidate env`
   事务运行时使用的候选虚拟环境，只用于观察，不直接代表 prod 真相。
2. `staged workdir`
   staged snapshot 所在目录，保存待 lock / 待 promote 的真相副本。
3. `观察运行`
   在 candidate env 中启动 ComfyUI，用有限时间窗口观察是否能正常启动、加载与运行。
4. `artifact`
   事务过程中产生的可审计目录或文件，本文重点指 `candidate_env`、`staged_workdir`、stdout/stderr log、freeze 文件、冲突报告。
5. `事务成功`
   这里只指事务记录层面的 `status=completed` 或 `status=promoted`，不等同于“进程自然退出 0”。

## 3. 当前应视为跨语言契约的逻辑

### 3.1 torch-family 属于独立权威

1. `torch`、`torchvision`、`torchaudio` 不属于普通 core requirements 的直接治理范围。
2. `update run` 或 `install` 读取 ComfyUI requirements 时，遇到 torch-family 必须过滤并继续，而不是把它当成事务失败原因。
3. 也就是说，`requirements.txt` 里包含 torch-family 是正常输入；系统应跳过它们，而不是因此阻断升级事务。

这个规则的本质不是“某种工具要不要报错”，而是：

1. core requirements 与 torch-family 的权威来源不同；
2. 同一个包族不能在两个治理入口里同时成为 source of truth。

### 3.2 观察运行是“有界观察”，不是“必须自然退出”

`tx run` 和 `update run` 的 candidate 运行，本质上是一次 **有时间上限的观察窗口**。

正确语义是：

1. candidate 运行开始后，只要进入观察阶段，就允许在到达超时上限时被系统收束。
2. 因超时被收束，不应自动解释为“事务失败”。
3. 事务应记录真实 `run_exit_code`，但事务状态应记为 `completed`，而不是 `failed`。

因此，下面两件事要同时成立：

1. `run_exit_code=124` 之类的“观察超时退出码”必须被保留下来，供审计和 inspect 使用。
2. 事务 `status` 仍应是 `completed`，因为这代表“观察窗口完成”，不是“观察失败”。

这个区分非常重要，因为：

1. ComfyUI 是常驻服务，很多成功场景本来就不会自行退出；
2. 如果把“观察超时”记为 `failed`，就会把“成功拉起并稳定运行到观察结束”的场景错误地标成失败；
3. 这会进一步污染 promote 资格判断，让本不需要 `--allow-failed-run` 的事务被误判。

### 3.3 candidate 输出必须同时满足“可见性”与“可审计性”

candidate 运行期间的 `stdout` / `stderr`，正确契约不是二选一，而是同时满足两件事：

1. 运行中的操作员应能在终端实时看到输出；
2. 同一份输出也必须落入事务日志文件，作为后续 inspect / 审计依据。

因此，`tx run` / `update run` 的输出契约应是：

1. 终端实时回显 candidate 输出；
2. 同步写入 `state/logs/<txid>.stdout.log` 与 `state/logs/<txid>.stderr.log`；
3. `logs.run_exit_code` 记录最终退出码。

如果只写日志、不回显终端，用户会误以为命令挂住。
如果只回显终端、不写日志，则 inspect 与审计链断裂。

### 3.4 `update run` 的 staged core 更新应是“一次性改写，再 lock”

core update 的本质，是把新的 requirements 集合替换进 staged snapshot，再验证该 snapshot。

因此，语言无关的正确行为应是：

1. 先根据 requirements 生成过滤后的 core 依赖集合；
2. 直接把 `dependency-groups.core` 改写成目标集合；
3. 然后做一次明确的 lock；
4. lock 成功后再进入 candidate sync 与观察运行。

不应把它设计成：

1. 逐条 remove；
2. 再逐条 add；
3. 每一步都触发完整求解。

原因不是性能优化本身，而是事务语义：

1. staged snapshot 应代表一个明确的目标集合；
2. 事务应该围绕“目标集合是否能 lock / sync / run”来判定；
3. 不能让中间若干个临时集合主导用户感知，导致长时间无输出、candidate env 长时间不出现、用户误判为卡死。

### 3.5 promote 成功后，临时 artifacts 默认应被清理

一旦 `tx promote` 或 `update promote` 已经成功把 staged truth 晋升到 prod，那么：

1. `candidate env` 不再参与系统正确性；
2. `staged workdir` 也不再是运行所需真相；
3. 它们应默认被视为临时 artifacts，而不是长期活跃资产。

因此默认契约应是：

1. `tx promote` 成功后，默认清理该事务的 `candidate_env`；
2. `update promote` 成功后，默认清理该事务的 `candidate_env` 与 `staged_workdir`；
3. 若操作者明确要求保留，则通过 `--keep-artifacts` 显式退出默认清理策略。

这个规则的目标是：

1. 降低磁盘占用；
2. 降低“事务已经 promoted，但磁盘上仍残留完整 candidate env”带来的运维歧义；
3. 保留审计信息，但不长期保留整套候选运行环境。

### 3.6 清理必须晚于 promote 成功落账

artifact 清理的顺序也属于业务契约，不只是实现细节。

正确顺序是：

1. 先完成 prod sync / smoke；
2. 再把事务状态记为 `promoted`；
3. 再把 operation 记为成功；
4. 最后再清理 artifacts。

如果清理失败，正确语义是：

1. promote 依然是成功的；
2. 只发出 warning；
3. 不允许因为“删目录失败”反向污染已经成功的 promote 结果。

### 3.7 事务 JSON 必须保留 artifact 的原始路径

成功 promote 后，即使目录已经删除，事务 JSON 里也不应把：

1. `candidate_env`
2. `staged_workdir`

清空或抹除。

正确契约是：

1. JSON 持续保存它们当时的原始路径；
2. inspect 根据“路径是否还存在”补充展示状态；
3. 审计信息与真实目录生命周期分离。

这能保证：

1. 历史事务仍能回答“它当时的 candidate env 在哪里”；
2. 路径缺失不会被误解成“字段丢了”或“事务不完整”；
3. 自动清理不会破坏历史记录的可读性。

### 3.8 inspect 需要区分“已清理”与“异常缺失”

当事务 JSON 中记录了 artifact 路径，但目录已经不存在时，inspect 不能只显示裸路径。

正确行为应是：

1. 若事务已 `promoted` 或 `aborted`，且路径不存在，显示为 `(cleaned)`；
2. 若事务仍在其他状态，且路径不存在，显示为 `(missing)` 或等价语义；
3. 不改变 JSON，只改变 inspect 呈现。

这条规则能把“按策略清理”与“异常丢失”区分开。

## 4. 推荐状态语义补充

为了避免未来再次出现“进程成功但事务记失败”的问题，建议把下面几条视为明确的跨语言状态规则：

1. `running -> completed`
   适用于 candidate 观察完成，包括：
   - 进程自然退出 0；
   - 进程在观察窗口到期后被系统正常收束。
2. `running -> failed`
   适用于真正的运行失败，例如：
   - candidate env sync 失败；
   - entrypoint 缺失；
   - 运行期非观察性异常退出；
   - 观察前置步骤本身失败。
3. `completed -> promoted`
   适用于 promote 成功，无论 artifacts 是否随后清理成功。
4. `completed|resolved|failed -> needs_resolution`
   仅适用于 lock / dependency conflict 一类需要人工 pin 介入的场景，不应被运行超时滥用。

## 5. Rust 分支文档审计

下面的审计对象是 `origin/feat/rust-rewrite` 分支中的文档，而不是当前 Bash 实现。

### 5.1 已对齐项

下列逻辑在 Rust 分支文档中已经基本表达正确：

1. `update run` 会过滤 torch-family
   - 见 `dev-docs/commands/update.md`
2. core update 采用 staged snapshot，再 promote 到 root truth
   - 见 `dev-docs/commands/update.md`
3. candidate 运行超时不依赖 Unix `timeout` 命令
   - 见 `dev-docs/commands/tx.md`
   - 见 `dev-docs/modules/runtime-executor.md`
   - 见 `dev-docs/adr/003-rust-rewrite-plan.md`
4. promote 成功后会清理 artifacts
   - 见 `dev-docs/commands/tx.md`
   - 见 `dev-docs/commands/update.md`

也就是说，Rust 分支文档没有重复“torch-family 必然导致 update run 失败”这种错误。

### 5.2 明确存在的错误或遗漏

#### A. 把观察超时写成 `failed`

这是最重要的不一致项。

Rust 分支文档目前仍把 candidate 运行超时描述为失败：

1. `dev-docs/commands/tx.md`
   - 写明 timeout 时 `status -> "failed"`
2. `dev-docs/commands/update.md`
   - 写明 timeout/error -> `status "failed"`
3. `dev-docs/modules/runtime-executor.md`
   - `RunOutcome::TimedOut` 被单列，但没有说明它在事务层应映射到 `completed`

这会把“有界观察完成”误写成“运行失败”，与当前正确事务语义不一致。

#### B. 缺少“终端实时输出 + 日志落盘”双通道契约

Rust 分支文档提到了日志文件，但没有把“终端实时可见”列为契约：

1. `dev-docs/modules/runtime-executor.md`
   - 只写了 `stdout/stderr -> log files`
2. `dev-docs/commands/tx.md`
   - 只写 capture 到 log files
3. `dev-docs/commands/update.md`
   - 只写 log files

这会遗漏一个重要的用户可感知行为：命令不应在观察阶段看起来“无输出地挂住”。

#### C. `--keep-artifacts` 与“默认清理”没有文档化

Rust 分支文档虽然描述了成功 promote 后会清理 artifacts，但没有写出新的默认策略接口：

1. `dev-docs/commands/tx.md`
   - `tx promote` synopsis 缺少 `--keep-artifacts`
2. `dev-docs/commands/update.md`
   - `update promote` synopsis 缺少 `--keep-artifacts`
3. `docs/04_cli_reference.md`
   - `gov tx promote` / `gov update promote` 都缺少 `--keep-artifacts`

也就是说，Rust 分支文档没有覆盖“默认清理，但允许显式保留”的完整产品语义。

#### D. 缺少“JSON 保留路径，inspect 标记 cleaned”的契约

Rust 分支文档虽然写了 transaction schema 有 `candidate_env` / `staged_workdir`，但没有把 promote 后的表现说完整：

1. `dev-docs/modules/state-ledger.md`
   - 没写路径字段在清理后仍保留
2. `docs/05_data_contracts.md`
   - 没写清理后仍保留原始路径
3. `dev-docs/commands/tx.md`
   - `tx inspect` 没写 `(cleaned)` / `(missing)` 呈现
4. `dev-docs/commands/update.md`
   - `update inspect` 也没写对应呈现

这会导致实现者不清楚：

1. promote 后究竟该不该清空 JSON 字段；
2. inspect 应如何区分“正常清理”与“异常缺失”。

#### E. 缺少“清理发生在成功落账之后”的顺序约束

Rust 分支文档现在写了“clean up candidate env / workdir”，但没有把顺序说死：

1. `dev-docs/commands/tx.md`
2. `dev-docs/commands/update.md`

缺少的关键信息是：

1. 先 `tx_set_promoted` / finalize success；
2. 再 cleanup；
3. cleanup 失败只警告，不反写 promote 为失败。

这会在实现阶段留下歧义，容易把“清理失败”错误上升为“promote 失败”。

### 5.3 中等优先级遗漏

#### F. torch-family 的“跳过而非失败”没有被写成显式规则

Rust 分支的 `dev-docs/commands/update.md` 已写“Filter out torch family packages”，这比旧问题前进了一步。

但它还没有把下面这层语义写明：

1. torch-family 出现在 requirements 中是允许的；
2. 过滤它们是正常路径；
3. 它们本身不构成 `update run` 的失败原因。

这不是当前最严重的问题，但如果想避免未来再次回退到错误语义，建议在命令文档中写得更明确。

## 6. 对 Rust 分支文档的修订建议

建议后续在 Rust 分支至少同步修订下面这些文档：

1. `dev-docs/commands/tx.md`
2. `dev-docs/commands/update.md`
3. `dev-docs/modules/runtime-executor.md`
4. `dev-docs/modules/state-ledger.md`
5. `docs/04_cli_reference.md`
6. `docs/05_data_contracts.md`

建议优先顺序：

1. 先修 timeout 的状态映射；
2. 再修 candidate 输出双通道契约；
3. 再补 `--keep-artifacts`、artifact path 保留、inspect 的 `(cleaned)` 语义；
4. 最后补清理顺序与 torch-family 的显式说明。

## 7. 建议作为后续统一验收标准的条目

无论 Bash 版还是 Rust 版，后续都建议把下面这些行为纳入统一验收：

1. `requirements.txt` 含 torch-family 时，`update run` 继续执行而不是因此失败。
2. candidate 运行因观察超时结束时，事务状态为 `completed`，同时保留 `run_exit_code`。
3. `tx run` / `update run` 的 candidate 输出同时进入终端和日志。
4. `tx promote` / `update promote` 默认清理临时 artifacts。
5. 传 `--keep-artifacts` 时，promote 后保留这些目录。
6. 清理后事务 JSON 仍保留原始 artifact 路径。
7. inspect 能把 `(cleaned)` 与 `(missing)` 区分开。
8. cleanup 失败不会把已经成功的 promote 反写成失败。
