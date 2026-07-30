# Elon AI Architecture Guide

本文件给 AI 代理快速理解 elon 的架构分层。完整系统说明见 `docs/system-architecture.md`；本文件保持高信噪比，帮助 AI 在修改前判断应该查哪一层。

## 总体分层

```text
Android APK / Web UI
  -> Rust API server
  -> Intent Router / Conversation Owner
  -> Group AI Coordinator / Matter Planner
  -> AI CLI / API agent routing
  -> Project workspace / Git worktree
  -> Context compiler / repo map / symbol index
  -> Build, publish, verification scripts
```

## 运行时主链路

1. 用户在 APK 或 Web 里通过多轮讨论表达自然语言需求。
2. 后端识别项目、用户、会话和模型配置。
3. 用户确认方向和验收标准后，Group AI Coordinator 生成 Matter 和执行计划。
4. AI CLI、API agent 或 Worker Bot 在真实 Git 隔离工作区执行任务。
5. context compiler 为不同角色生成裁剪后的 repo map、symbol index、task pack 和验证线索。
6. AI 修改代码后由 Reviewer / Verifier 执行最小有效验证和独立审查。
7. 业务提交进入目标项目 Git 历史，再由后端或 APK 发布脚本按需构建、上传、部署和验证。
8. 运行结果、用户验收和失败原因沉淀为 Context 和 Taste 数据。

## PC 节点 AI 运行路线

项目会话里的 AI 能力按三层理解，避免把“路由选择”“CLI 传输方式”和“前端展示”混成一个开关：

| 层级 | 决定什么 | 当前状态 |
|---|---|---|
| 1. 运行路线 | AI / 模型从哪里来，项目在哪台 PC 执行 | 已有 `route_a` / `route_b` / `route_c` / `route_c2` / `route_c3` |
| 2. CLI 会话 / 传输模式 | node-agent 如何启动、连接和恢复 CLI | Codex JSON 默认 `pipe_sidecar + pipe + JSON`；`direct_json_pipe` 保留为回退；`pty_sidecar` 保留给终端接管和 TUI |
| 3. 前端展示 / 恢复模式 | PC UI 如何展示过程、折叠最终回复、恢复或接管任务 | 结构化过程卡片读取 JSON 事件和 task journal；终端 attach 读取 PTY sidecar |

因此，Route A 本机 CLI 是否使用 PTY 是第二层传输模式选择，不是新的运行路线。Route A / Route C3 都可以在自己的节点内选择 `pipe_sidecar`、`direct_json_pipe` 或 `pty_sidecar`。

第一层：运行路线：

| 路线 | 模型/AI 来源 | 项目文件与命令在哪里执行 | 适用场景 |
|---|---|---|---|
| `route_a` 本机AI | 项目绑定 PC 上已登录的 Codex / Copilot / Claude / Gemini CLI | 项目绑定 PC 节点 | 项目 owner 自己电脑已准备好 CLI；项目会话默认优先走这条 |
| `route_b` 我的Key | 项目绑定 PC 上配置的 OpenAI-compatible API key | 项目绑定 PC 节点的一龙工具 runtime | 用户想用自己的模型 key，仍让本机执行文件/命令 |
| `route_c` 平台AI | 一龙平台提供模型 | 项目绑定 PC 节点的一龙工具 runtime | 用户没有 CLI 或自己的 key；项目操作仍不搬到服务器 |
| `route_c2` 远程AI | 其他用户 PC 节点的 API runtime | 被授权的远程 PC 节点 | 自己电脑不方便运行，借用远程节点和它的 key |
| `route_c3` 远程Codex | 其他用户 PC 节点已登录的 Codex / Claude / Copilot 等 CLI | 被授权的远程 PC 节点 | 借用远程节点上的专业 CLI |

当前 PC 项目会话默认值是 `route_a`。前端“直连 CLI”开关会把本轮请求强制成 `route_a`，并传入本机 `localNodeId` 与项目 `workspacePath`；当前主实现是 Codex CLI，后续可在同一 Route A 下接入 Copilot / Claude / Gemini 等本机 CLI。

### Codex 桌面监督与 PC 执行闭环

> 当前状态：自 2026-07-26 起暂停使用。暂停期间不得新建或自动续跑监督任务；实现与下述架构说明保留为历史和恢复参考。当前项目任务由当前代理直接按照目标仓库规则完成。

Codex 桌面端讨论本项目并需要实际改动时，可以通过仓库级 `codex-pc-supervisor` Skill 把需求交给本机 `/api/local-tasks`。PC 节点启动 Codex CLI，在项目隔离 worktree 内修改、验证、提交、发布和收尾；桌面端保留监督者角色，独立检查 journal、diff、测试、产物与 `FINALIZABLE`，再写入验收结论。协议和操作手册见 `docs/codex-desktop-pc-supervision.md`。

```text
用户 <-> Codex Desktop（拆解 / 监督 / 验收）
                    |
                    v  elon.desktop_pc_supervision.v1
         一龙 PC 本机节点 -> Codex CLI -> 项目 worktree
                    |                    |
                    +--- journal / diff / tests / publish ---+
                    ^                                      |
                    +--------- 桌面端独立复核 <-------------+
```

闭环有两条但共享同一审计链：任务闭环负责完成需求；能力闭环只在平台能力阻断原任务时先派发 `capability_repair`，修复验收后再以 `resume_original` 续跑。非阻塞改进使用 `post_task_improvement`，必须排在用户任务之后。执行提示带 `<elon-pc-executor>` 防递归；看到该标记的 CLI 必须直接工作，不能把任务再次派回节点。

这是可版本化、可回滚、可审计的系统级迭代，不是模型自行训练。桌面端监督不扩大用户授权，也不能用“执行者声称成功”代替独立证据；本机节点不可用时不得静默改为桌面端写代码。

并发边界要分两层看：不同 PC 节点、不同项目工作区可以并行；同一 PC 节点上的外部 CLI 也可以并行，但必须受“PC 节点 + CLI”的容量槽位限制，不能无限并发。`route_a` / `route_c3` 这类路线依赖本机 Codex、Claude、Copilot 登录态和 sidecar；同一台电脑上的 CLI 进程还会共享缓存、系统资源、项目路径、Git/Cargo 产物和模型服务限流。多个会话无上限压上去时容易出现“只有等待状态、没有公开输出、最后超时失败”。因此后端必须按节点硬件、用户配置和运行状态限制同节点 CLI 并发槽位，前端必须把“节点槽位已满 / 排队等待时长 / 已获得节点执行权 / 已派发到 CLI / CLI 输出中”展示出来。

Android 真机调试在此之外还有提交级集成边界：会话的源码编辑和普通编译可在隔离 worktree 并行，但共享真机不直接消费任何会话的脏目录，也不让多个会话写同一个 APK 路径。节点只接收明确 ready、来源 task/session 可追踪、HEAD 与提交列表一致的干净候选，在节点数据根下为每一代创建独立 detached 集成 worktree，按登记顺序 cherry-pick。每个“仓库 + 项目 + 物理设备 + 节点”固定槽保存基础 SHA、贡献提交、冲突、期望/已安装代次、预览所有者和 LKG 策略状态；同文件冲突不自动解决，旧代次在安装前再次检查 fencing token。LKG 默认关闭，只有任务显式启用后才记录、推进和校验最近成功 APK；默认路径不要求 LKG，也不阻塞构建、ADB 覆盖安装或收尾。构建后的当前 APK 始终经包名、应用标签、签名、版本和 SHA-256 校验后进入内容寻址目录，再由物理设备身份级互斥锁串行覆盖安装。

真机包身份同样是节点级不变量：所有 `.uituner`、`.uitest`、`.uitest_anim` 兼容调用都归一成 `com.elon.app.uituner_<稳定节点指纹>`。模拟器只有在显式请求时才允许隔离测试后缀；正式 `com.elon.app` 不经过这条归一化。节点更新若发现持久调试指纹与 `install_id` 漂移，必须拒绝调试部署并说明如何恢复，不能静默生成第二套包；签名不一致、冲突或设备离线也不得自动卸载手机现有应用。

当前容量策略是保守自动估算：未知硬件默认 1 个槽；较强工作站可自动提升到 2-4 个槽；`ELON_PC_NODE_CLI_MAX_PARALLEL` 可显式指定本机 CLI 并发槽位，`ELON_PC_NODE_CLI_HARD_MAX_PARALLEL` 控制服务端硬上限。后续可以继续把 CPU/内存/显存、当前活跃 CLI 数、失败率和平均首包时间纳入动态降档。

第二层：CLI 会话 / 传输模式：

| 模式 | 当前是否具备 | 定位 |
|---|---|---|
| `pipe_sidecar` | 已具备，Codex JSON 默认 | sidecar 负责进程生命周期、取消、journal、session id 和恢复入口；CLI stdout/stderr 仍保持程序 pipe，不进入 PTY |
| `direct_json_pipe` | 已具备，回退路径 | node-agent 直接启动 `codex exec --json`，读取干净 stdout JSONL / stderr；设置 `ELON_CODEX_PIPE_SIDECAR=0` 且保持 `ELON_CODEX_JSON_DIRECT_STDOUT=1` 时使用 |
| `pty_sidecar` | 已具备，辅助路 | 用 portable_pty / ConPTY 管真实终端，适合 TUI、人工接管、resize、交互输入和终端型 CLI |

当前 `pipe_sidecar` 已用于 Codex JSON 主路：它保留 sidecar 的生命周期管理和恢复能力，同时避免 PTY 污染 JSONL。显式设置 `ELON_CODEX_PIPE_SIDECAR=0` 可回退到旧的直接子进程 pipe；显式设置 `ELON_CODEX_JSON_DIRECT_STDOUT=0` 会回到旧 PTY sidecar 路径，只应作为兼容或调试用途。

### 当前能力和后续增强

`pipe_sidecar` 不是因为 Codex CLI 不够好才需要。Codex CLI 负责“聪明地干活”：读项目、跑命令、改文件、总结结果；一龙平台负责“可靠地管理这次干活”：排队、并行、取消、重连、恢复、journal、前端过程展示和最终回复折叠。

| 维度 | 当前默认 `pipe_sidecar + pipe + JSON` | 回退 `direct_json_pipe` | 差距 / 判断 |
|---|---|---|---|
| 最小可用 | 已让 Codex 直接处理项目会话 | 仍可工作 | direct 适合排障；默认用 sidecar 管生命周期 |
| JSON 干净度 | 好，仍然读 `codex exec --json` stdout pipe | 好，直接读 stdout pipe | 两者都不能退回 `PTY + JSON` |
| 任务管理 | sidecar 独立管理进程生命周期、取消、session id、journal 和恢复入口 | 主要由 node-agent 当前 runner 和 task journal 管 | 默认路径已经补齐平台级生命周期 |
| 重连 / 恢复 | sidecar registry 暴露 `managed_pipe_json_sidecar`；云端可通过节点 WS `InspectCliTaskJournal` 读取本机 journal、attach、resume 和审批状态；前端会把恢复快照合成为公开过程 | 依赖 journal、Codex session/thread id 和云端快照组合 | 当前已闭环“恢复可见 + 继续入口”；更强恢复还要把 Codex 原生 `resume <SESSION_ID>` 自动接进续接执行 |
| 前端过程感 | JSON 事件和 sidecar output/journal 一起支撑公开过程卡片 | 只靠直接 stdout 和 journal | 当前还要继续把 UI 过程卡片做细 |
| 多 CLI 扩展 | Codex 已走 pipe sidecar；PTY sidecar 继续服务终端型 CLI | Codex 专用回退 | 后续可把更多稳定 JSON CLI 接入 pipe sidecar |

因此当前策略是：默认用 `pipe_sidecar + pipe + JSON` 管 Codex；继续完善前端公开过程卡片、任务恢复入口和多 CLI 管理。`pipe_sidecar` 是平台级会话管理层，不是对 Codex CLI 能力的替代。

任务恢复目前已经不是只停留在“保留现场 + 允许继续”：服务器任务快照会带着 `pc_req_id` / `agent_id` 去问在线 Win 端，本机返回 task journal、sidecar attach、Codex session/thread、审批状态和 `resume` 合同；网页端把这些信息显示成公开过程，并在自动恢复失败时显示 `resume_required`。它仍不是强制热迁移：如果 Win 端离线、旧节点不支持协议、本机 journal 丢失或 Codex 原生 session 不可用，就只能从云端快照和当前工作区继续，而不能声称原进程已经无缝续上。

当前 Codex CLI 的主路不是 PTY，而是：

```text
PC 网页端 -> Rust server -> node-agent -> pipe sidecar -> codex exec --json
  -> 直接读取 stdout JSONL
  -> 解析 assistant_message / tool_call / tool_result / usage / final_reply
  -> 网页端任务过程卡片
```

`codex exec --json` 是给程序消费的结构化事件流，默认不能再放进 PTY/ConPTY 里抠 JSON，否则终端折行、ANSI 控制序列、光标帧和提示文本会污染事件流。当前节点默认 `ELON_CODEX_JSON_DIRECT_STDOUT=1` 且 `ELON_CODEX_PIPE_SIDECAR` 默认开启，因此 Codex 进入 `managed_pipe_json_sidecar`；只有显式设置 `ELON_CODEX_JSON_DIRECT_STDOUT=0` 才回到旧 PTY sidecar 路径。

PTY/ConPTY sidecar 仍然保留，但定位是辅助路：

- 交互式 `codex` TUI 或其他终端型 CLI 需要用户接管时使用。
- 需要真实终端输入、resize、取消、调试、审批恢复时使用。
- Copilot / Claude / Gemini 等没有稳定 JSONL 事件流的 CLI 可以继续走 sidecar/PTY。

## 项目理解 / RAG 架构

当前项目理解系统不是纯向量库，而是混合检索：

```text
AI_PROJECT / AI_INDEX / AGENTS
  -> project_docs_get_map / project_docs_get_node / project_docs_plan_context
  -> 按需读取少量权威 Markdown
  -> context compiler
  -> repo map + chunks
  -> rust-analyzer / semantic facts
  -> symbol_index.sqlite
  -> symbol search + graph + impact pack
  -> local-hash-v1 vector retrieval
  -> task pack / patch plan / verification hints
```

各层职责：

- 文档治理层：路径和元数据说明规则、权威性、生命周期与推荐入口。
- 项目图谱层：同一 Rust 模型分别描述产品功能、技术架构和文档主题；节点连接 Markdown 与 `file:/test:/route:/symbol:` 实现证据，AI 先取局部图再决定读什么。
- repo map 层：压缩仓库结构、重要文件、目录摘要。
- 符号层：记录函数、类型、trait、模块、调用/引用/包含关系。
- chunk 层：为全文搜索和向量召回提供代码片段。
- 向量层：补充自然语言语义召回；当前默认 `local-hash-v1`。
- 验证层：生成 patch plan、dry run、review、verification、repair context。

项目图谱不替代符号索引，也不把“存在文档”当成“实现完成”。图谱负责给人和 AI 一个稳定的项目导航与声明关系；context compiler、符号索引和测试负责核验代码事实。PC 网页端与所有供应商的 Streamable HTTP MCP 消费同一后端图谱，图级或节点级评审把结构变更写入 `proposed_knowledge_graph`，经 revision、权限模式和 Git 文档事务后才成为共享事实。

长聊天另行编译为 `.elon/discussion-graph.json`，不混入产品功能、技术架构或当前权威文档。讨论图以稳定节点 ID 和 Git 快照保存演化：MCP 可读取语义版本、旧版图、版本差异和单节点生命周期，PC 端显示同一时间轴。确定性审查先发现来源、权威性、失效关联、未解决异议和演化链问题；Win 端登录账号的任意受支持 AI CLI 再通过 proposal/apply 形成修正版。每次应用产生新版本，不改写旧图，也不需要为单次脑图变化发布一龙程序版本。

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
