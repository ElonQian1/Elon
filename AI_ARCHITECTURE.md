# Elon AI Architecture Guide

本文件给 AI 代理快速理解 elon 的架构分层。完整系统说明见 `docs/system-architecture.md`；本文件保持高信噪比，帮助 AI 在修改前判断应该查哪一层。

## 总体分层

```text
Android APK / Web UI
  -> Rust API server
  -> Intent Router / Conversation Owner
  -> Demo Oracle（低成本产品预演，按需触发）
  -> Group AI Coordinator / Matter Planner
  -> AI-to-AI Skill Router / Skill Registry
  -> AI CLI / API agent routing
  -> Project workspace / Git worktree
  -> Context compiler / repo map / symbol index
  -> Build, publish, verification scripts
```

## 运行时主链路

1. 用户在 APK 或 Web 里通过多轮讨论表达自然语言需求。
2. 后端识别项目、用户、会话、模型配置以及需求成熟度。
3. 对目标仍不清晰、改动成本高或存在多个产品方向的需求，调用预言家 AI 生成低成本 demo、页面草图、用户流程和待确认问题；明确的小改动跳过该阶段。
4. 用户确认方向后，Group AI Coordinator 生成 Matter，Skill Router 从官方或已审核 Skill 中选择能力组合，并说明选择理由、成本和风险。
5. AI CLI、API agent 或 Worker Bot 在真实 Git 隔离工作区执行任务。
6. context compiler 为不同角色生成裁剪后的 repo map、symbol index、task pack 和验证线索。
7. AI 修改代码后由 Reviewer / Verifier 执行最小有效验证和独立审查。
8. 业务提交进入目标项目 Git 历史，再由后端或 APK 发布脚本按需构建、上传、部署和验证。
9. 运行结果、用户验收和失败原因沉淀为 Context、Taste、Skill 质量数据。

预言家 AI 不是正式开发者，也不拥有发布权限。它默认使用低成本模型和受限工具，只允许生成临时 demo 产物；不得直接修改正式项目主线、接入真实支付、执行生产部署或把假数据包装成已完成能力。

## PC 节点 AI 运行路线

项目会话里的 AI 能力按三层理解，避免把“路由选择”“CLI 传输方式”和“前端展示”混成一个开关：

| 层级 | 决定什么 | 当前状态 |
|---|---|---|
| 1. 运行路线 | AI / 模型从哪里来，项目在哪台 PC 执行 | 已有 `route_a` / `route_b` / `route_c` / `route_c2` / `route_c3` |
| 2. CLI 会话 / 传输模式 | node-agent 如何启动、连接和恢复 CLI | Codex 默认 `direct_json_pipe`；`pipe_sidecar` 是建议目标形态，当前未独立实现；`pty_sidecar` 保留给终端接管和 TUI |
| 3. 前端展示 / 恢复模式 | PC UI 如何展示过程、折叠最终回复、恢复或接管任务 | 结构化过程卡片读取 JSON 事件和 task journal；终端 attach 读取 PTY sidecar |

因此，Route A 本机 CLI 是否使用 PTY 是第二层传输模式选择，不是新的运行路线。Route A / Route C3 都可以在自己的节点内选择 `direct_json_pipe`、未来 `pipe_sidecar` 或 `pty_sidecar`。

第一层：运行路线：

| 路线 | 模型/AI 来源 | 项目文件与命令在哪里执行 | 适用场景 |
|---|---|---|---|
| `route_a` 本机AI | 项目绑定 PC 上已登录的 Codex / Copilot / Claude / Gemini CLI | 项目绑定 PC 节点 | 项目 owner 自己电脑已准备好 CLI；项目会话默认优先走这条 |
| `route_b` 我的Key | 项目绑定 PC 上配置的 OpenAI-compatible API key | 项目绑定 PC 节点的一龙工具 runtime | 用户想用自己的模型 key，仍让本机执行文件/命令 |
| `route_c` 平台AI | 一龙平台提供模型 | 项目绑定 PC 节点的一龙工具 runtime | 用户没有 CLI 或自己的 key；项目操作仍不搬到服务器 |
| `route_c2` 远程AI | 其他用户 PC 节点的 API runtime | 被授权的远程 PC 节点 | 自己电脑不方便运行，借用远程节点和它的 key |
| `route_c3` 远程Codex | 其他用户 PC 节点已登录的 Codex / Claude / Copilot 等 CLI | 被授权的远程 PC 节点 | 借用远程节点上的专业 CLI |

当前 PC 项目会话默认值是 `route_a`。前端“强制 Codex / 直连”开关会把本轮请求强制成 `route_a`，并传入本机 `localNodeId` 与项目 `workspacePath`。

第二层：CLI 会话 / 传输模式：

| 模式 | 当前是否具备 | 定位 |
|---|---|---|
| `direct_json_pipe` | 已具备，Codex 默认 | node-agent 直接启动 `codex exec --json`，读取干净 stdout JSONL / stderr，并把事件写入任务过程 |
| `pipe_sidecar` | 当前未独立实现，建议作为下一阶段目标 | sidecar 负责进程生命周期、取消、journal、session id 和恢复入口；CLI stdout/stderr 仍保持程序 pipe，不进入 PTY |
| `pty_sidecar` | 已具备，辅助路 | 用 portable_pty / ConPTY 管真实终端，适合 TUI、人工接管、resize、交互输入和终端型 CLI |

长期看，`pipe_sidecar` 比把 Codex JSON 放进 PTY 更适合后台结构化过程展示：它保留 sidecar 的生命周期管理和恢复能力，同时避免 PTY 污染 JSONL。但这不是当前已经完成的独立实现，当前 Codex 主路仍是 `direct_json_pipe`。

### 现状、未来蓝图和差距

`pipe_sidecar` 不是因为 Codex CLI 不够好才需要。Codex CLI 负责“聪明地干活”：读项目、跑命令、改文件、总结结果；一龙平台负责“可靠地管理这次干活”：排队、并行、取消、重连、恢复、journal、前端过程展示和最终回复折叠。

| 维度 | 当前 `direct_json_pipe` | 未来 `pipe_sidecar + pipe + JSON` | 差距 / 判断 |
|---|---|---|---|
| 最小可用 | 已足够让 Codex 直接处理项目会话 | 不是 MVP 必需 | 只要“发一句话等回复”，当前方案不需要 sidecar |
| JSON 干净度 | 好，直接读 `codex exec --json` stdout | 同样好，仍然走 pipe，不走 PTY | 未来不能退回 `PTY + JSON` |
| 任务管理 | 主要由 node-agent 当前 runner 和 task journal 管 | sidecar 独立管理进程生命周期、取消、session id、journal 和恢复入口 | 差距在平台级生命周期统一性，不在 Codex 智能能力 |
| 重连 / 恢复 | 依赖现有 journal、Codex session/thread id 和云端快照组合 | sidecar 提供更稳定的运行句柄和恢复契约 | 多会话并行、节点重启、长任务恢复越多，sidecar 价值越高 |
| 前端过程感 | 能展示 JSON 事件里已有的公开过程 | 更容易稳定沉淀完整过程、等待状态、取消和恢复状态 | 当前优先把 JSON 事件解析和 UI 展示补完整 |
| 多 CLI 扩展 | Codex 最顺；其他 CLI 视输出能力而定 | sidecar 可统一管理 Codex / Claude / Copilot / Gemini 等 CLI | 多 CLI 成为核心能力后再实现更划算 |

因此当前策略是：先把 `direct_json_pipe` 跑稳，把 Codex JSON 公开过程完整展示出来；当取消、重连、恢复、多会话并行和多 CLI 管理成为主要瓶颈时，再升级为 `pipe_sidecar + pipe + JSON`。`pipe_sidecar` 是平台级会话管理蓝图，不是对 Codex CLI 能力的替代。

当前 Codex CLI 的主路不是 PTY，而是：

```text
PC 网页端 -> Rust server -> node-agent -> codex exec --json
  -> 直接读取 stdout JSONL
  -> 解析 assistant_message / tool_call / tool_result / usage / final_reply
  -> 网页端任务过程卡片
```

`codex exec --json` 是给程序消费的结构化事件流，默认不能再放进 PTY/ConPTY 里抠 JSON，否则终端折行、ANSI 控制序列、光标帧和提示文本会污染事件流。当前节点默认 `ELON_CODEX_JSON_DIRECT_STDOUT=1`，因此 Codex 跳过 PTY sidecar；只有显式设置 `ELON_CODEX_JSON_DIRECT_STDOUT=0` 才回到旧 sidecar 路径。

PTY/ConPTY sidecar 仍然保留，但定位是辅助路：

- 交互式 `codex` TUI 或其他终端型 CLI 需要用户接管时使用。
- 需要真实终端输入、resize、取消、调试、审批恢复时使用。
- Copilot / Claude / Gemini 等没有稳定 JSONL 事件流的 CLI 可以继续走 sidecar/PTY。

## 项目理解 / RAG 架构

当前项目理解系统不是纯向量库，而是混合检索：

```text
AI_PROJECT / AI_INDEX / AGENTS
  -> context compiler
  -> repo map + chunks
  -> rust-analyzer / semantic facts
  -> symbol_index.sqlite
  -> symbol search + graph + impact pack
  -> local-hash-v1 vector retrieval
  -> task pack / patch plan / verification hints
```

各层职责：

- 文档层：说明项目是什么、规则是什么、入口在哪里。
- repo map 层：压缩仓库结构、重要文件、目录摘要。
- 符号层：记录函数、类型、trait、模块、调用/引用/包含关系。
- chunk 层：为全文搜索和向量召回提供代码片段。
- 向量层：补充自然语言语义召回；当前默认 `local-hash-v1`。
- 验证层：生成 patch plan、dry run、review、verification、repair context。

## 关键模块边界

| 模块 | 职责 |
|---|---|
| `server/src/context_compiler/` | repo map、上下文包、符号索引、RAG/task pack |
| `server/src/context_compiler/agent_rag_context.rs` | 暴露给 API agent 的 RAG 工具定义和调用 |
| `server/src/context_compiler/symbol_index_task_pack.rs` | 自然语言任务到符号/影响/context pack 的主流程 |
| `server/src/context_compiler/symbol_index_embedding_provider.rs` | embedding provider 抽象与本地 hash provider |
| `server/src/agent_config.rs` | 用户模型/API key 配置和加密持久化 |
| `server/src/agent_llm_call.rs` | OpenAI-compatible chat 调用和用量记录 |
| `scripts/publish-server.ps1` | 后端构建、版本 claim、上传、部署、验证 |
| `scripts/publish-apk.ps1` | APK 构建、签名、上传和版本发布 |

## 当前缺口

架构上已经完成了“文档 + repo map + 符号索引 + 混合检索 + 验证闭环”的主干。剩余增强应按这个顺序做：

1. 远程 embedding provider：把用户 API key/模型配置接入 `SymbolEmbeddingProvider`，支持 OpenAI-compatible `/embeddings`。
2. 成本和权限边界：远程 embedding 必须记录模型、维度、调用来源和失败原因，避免默认高成本全量回填。
3. 增量索引：文件 hash 未变化时跳过 chunk、symbol、embedding 重算。
4. 检索回归集：把真实任务保存为 eval case，比较 symbol/chunk/vector 召回质量。
5. 面向 UI 的状态解释：让用户能看到“项目已索引/缺少 embedding/需要重新索引”的原因。

## 修改建议

- 需要改项目理解/RAG：优先进入 `server/src/context_compiler/`，不要另建平行管线。
- 需要接用户 API key：先查 `server/src/agent_config.rs` 和 `server/src/user_agent_secrets.rs`。
- 需要新增检索能力：先让 `repo_context_task_pack` 能解释和暴露，不要只做后台数据。
- 需要上线后端：按 `AGENTS.md` 和 `.github/copilot-instructions.md` 的 commit/push/deploy 流程执行。
