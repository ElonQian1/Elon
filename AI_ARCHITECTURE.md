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

## 后台多端 UI 设计数据面

微调画布的数据面由 Windows 节点的 `yilong-ui-live` MCP 提供，PC 画布只是可选客户端：

```text
AI / PC Canvas
  -> list targets / list recent sessions
  -> resume or open project-scoped design session
  -> bind AI taskId + bounded exclusive lease
  -> cursor-based compact design event stream
  -> Web/PWA/Tauri frontend: controlled Chromium
  -> Tauri native host: project runtime + descendant window capture
  -> Android: Live Runtime
  -> semantic UI tree + PNG path/hash
  -> DraftOperation v2 + per-platform capability tier
  -> verified UI tree / selector -> reviewable source binding candidates
  -> source change + evidence-gated platform writeback receipt
```

代理先用 `ui_get_design_capabilities` 验证当前节点实际安装的 schema，再读取 Web、PWA、Tauri、Android 小型目标索引。无需打开 PC 页面即可恢复/打开项目 `designSessionId`；单次捕获之外，Web/PWA/Tauri 可用持久浏览器在同一 page 中导航和执行有界 selector 交互，非秘密表单值只引用项目 fixture。Android 不使用浏览器替代。Tauri 原生链路按项目发现命令启动，仅访问登记 Runtime 后代进程，窗口像素、菜单/候选对话框和项目 command trace 分层，且不开放任意菜单点击或 command 执行。

设计意图通过项目级、乐观 revision 的 Design Draft v2 表达。`SET_STYLE` 可在持久浏览器中白名单预览并恢复；文字、资源、variant、结构和响应式操作按平台能力分级进入源码 handoff，页面内联预览永远不是源码证据。源码绑定器只在目标声明的项目源码根内扫描已校验 UI tree/selector/route，返回带 SHA-256、行号和字节范围的多个 `CANDIDATE`，必须先采用再显式确认才成为 `BOUND`。AI taskId 通过有界独占 lease 绑定 designSession/draft，后台或 PC 以 taskId/cursor 增量读取紧凑事件；PC 因此跟随指定任务切换 session，而不是轮询项目最近会话。写回回执继续固定 Git/source 基线并要求分平台证据。具体契约、安装节点仍待升级和未执行真实编译/平台验收的边界见 `docs/headless-ui-design-mcp.md`。

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

## 任务级分布式算力联邦

一龙把用户节点、受管 GPU 集群和外部算力池统一为 `ComputeProvider`，聚合可独立执行、分片、重试和验证的 AI 任务。普通公网节点不被伪装成一张低延迟虚拟 GPU；需要张量或流水线并行的工作负载由受管集群内部完成，集群对一龙表现为一个逻辑 Provider。

```text
需求 / 本地 AI
  -> Broker + Offer Registry + Price Snapshot
  -> Reservation + Attempt Lease (fencing generation)
  -> User Node / Managed Cluster / External Pool
  -> declared / observed / verified usage
  -> Settlement Receipt + pending/disputed/available balance
```

架构分为五个平面：

- 控制面：Provider、Offer、报价、预留、租约、重试和取消；
- 数据面：输入工件、执行事件、检查点和结果工件；
- 工件面：Plugin、Runtime、Model 分开版本化和内容寻址；
- 验证与计量面：节点声明、平台观测、挑战/复算和最终验证事实；
- 市场与结算面：标准 Compute SKU、期货交付窗口、不可变价格快照和双价格腿回执。

当前 `server/src/compute_federation/` 提供未编译、未完整接线的云端领域合同；`server/src/store/compute_capacity_*.rs` 与 Provider、Offer、Price Snapshot、Job、Reservation Registry 已形成隔离的 CapacityPool、供给、Claim、Store-canonical 请求摘要、可组合事务内 Claim kernel、账本审计、到期恢复、生命周期、不可变历史与锁价快照。本人 Provider HTTP/MCP 控制面只允许创建和查询服务端固定为 `registering/self_declared` 的 `user_node` 或 `managed_cluster`；它不接受外部矿池、路由、凭据、适配器或 verified 证据，并返回脱敏视图。本人 CapacityPool 控制面只在自有 Provider 下创建和查询 `registering` Pool，可从不可变账本重算当前 epoch 的余额一致性，并按稳定序号分页读取省略消费者和业务因果字段的事务与双分录；审计健康和历史存在都不代表硬件 verified。Bucket 控制面只在当前 Pool 版本下创建 open、零发行余额交付窗口并读取账本投影；Supply 控制面在显式确认后向同一窗口原子追加 self-declared `issuance -> available` 双分录，或仅把 available 撤入 retired，且 registering 阶段也可纠正尚未交易的声明。服务端固定版本和首次时间并生成摘要，响应不返回原始资源范围密钥或档案；这些入口不激活、验证或发布 Offer。Hold V2 固定完整 causal binding，Reservation Claim 强制绑定 Offer/Job/Reservation，Finish 继承并审计原始 held 绑定；Job、Reservation 登记和余额预授权、退款均已有不提交的事务内入口。v175 `compute_broker_reservation` 已将这些构件组合为单一事务的第一版 Reserve，并强制预算结果为 `reserved` 且含余额结果；不可变回执绑定原 quoted Job、reserved Job、Capacity Claim、active Reservation 和平台人民币余额预授权，可在合同到期后继续重放且不依赖余额表的可变到期字段。v176 在 Attempt 尚未激活时，以另一单一事务完成严格退款、held Claim Release/Expire、Job 与 Reservation 终态，并保存可审计的不可变终态回执。通用余额释放与到期器排除 Broker 管理的预算，v176 通过精确预授权 ID 的严格入口终结，避免单腿退款。登录用户 HTTP 与项目范围 MCP 已提供本人 Job/Reservation 列表、详情和 Reserve、Release、Expire；MCP 的财务冻结与取消要求显式确认。整条路径仍是 `implementation_uncompiled`，只支持 `platform_balance_cny`，尚未执行迁移、HTTP/MCP 运行验证、激活 Attempt 或实现运行中任务与实际用量结算。`server/src/node_agent_compute_plugin_host/` 已增加未编译的 Manifest、InstallPlan、双槽生命周期、ReadyCapability、Attempt 命令和 typed event 合同，但尚无下载器、Sidecar 或通用协议接线。现有节点模型白名单、Token 预留和流租约继续作为 `user_node + llm_chat` 兼容路径。权威架构与阶段见 `docs/distributed-compute/README.md`。

v177 另行保存 Provider/Pool 激活证据申请。本人 HTTP/MCP 可显式提交、查询和取消，平台 `admin/owner` 只能通过 HTTP 审核；双方均可只读预检申请状态、路由、verified 硬件、信任层、精确版本和账本审计并获得稳定阻断码。`approved` 与 `ready_for_activation` 都固定返回 `activation_effect=none`，不触发激活、verified 硬件写入、路由配置或 Offer 发布。该控制面为 `implementation_uncompiled`，详细边界见 `docs/distributed-compute/activation-evidence-api.md`。
项目级 HTTP/MCP 还可创建归属当前用户的 submitted Job，按完整合同发现当前有效的既有 Offer/Price Snapshot，并把当前版本绑定到所选候选；服务端固定用户、项目、状态和时间，并校验商户归属。候选只返回价格合同与最小 Provider 摘要，不返回节点路由、凭据或适配器配置。该能力仍为 `implementation_uncompiled`，不包含新报价生成或自动撮合。

Provider 本人接口与信任边界见 `docs/distributed-compute/provider-api.md`，共享资源、窗口和供给边界见 `docs/distributed-compute/capacity-pool-api.md`、`docs/distributed-compute/capacity-bucket-api.md`、`docs/distributed-compute/capacity-supply-api.md`，证据申请和审核边界见 `docs/distributed-compute/activation-evidence-api.md`；Job、报价和预留接口见 `docs/distributed-compute/broker-api.md`。

## 关键模块边界

| 模块 | 职责 |
|---|---|
| `server/src/context_compiler/` | repo map、上下文包、符号索引、RAG/task pack |
| `server/src/context_compiler/agent_rag_context.rs` | 暴露给 API agent 的 RAG 工具定义和调用 |
| `server/src/context_compiler/symbol_index_task_pack.rs` | 自然语言任务到符号/影响/context pack 的主流程 |
| `server/src/context_compiler/symbol_index_embedding_provider.rs` | embedding provider 抽象与本地 hash provider |
| `server/src/agent_config.rs` | 用户模型/API key 配置和加密持久化 |
| `server/src/agent_llm_call.rs` | OpenAI-compatible chat 调用和用量记录 |
| `server/src/compute_federation/` | 分布式算力 Provider、Offer、Job、Lease、价格与回执领域合同；当前未接运行路径 |
| `server/src/store/compute_capacity_*.rs`、`compute_{provider,offer,job,reservation}_registry*`、`compute_quote_candidates.rs`、`compute_broker_reservation/`、`compute_federation_{provider,capacity_pool,capacity_bucket,capacity_supply,broker}_{service,api,mcp}.rs`、`compute_federation_mcp.rs` | 共享容量池、版本化合同、本人 self-declared Provider、registering Pool、当前 epoch 账本审计与脱敏历史、open Bucket 与原子 Supply Add/Withdraw 控制面、项目级 Job 创建/既有候选发现与锁价、本人 Job/Reservation 查询、本地原子 Reserve、未执行任务原子 Release/Expire；当前未编译、未迁移、未运行验证，未接 Provider 路由/激活、Offer 发布、新报价生成、自动撮合、Attempt 或运行中结算 |
| `server/src/store/compute_activation_requests.rs`、`compute_federation_activation_{service,api,mcp}.rs` | v177 激活证据申请、本人控制面和管理员 HTTP 审核；批准不激活、不写 verified 事实、不发布 Offer，当前未编译、未迁移、未运行验证 |
| `server/src/node_agent_compute_plugin_host/` | Windows 节点 legacy Host seam 与按需插件、Attempt、Runner event 合同；当前只有旧 LLM 接入，正式合同未接运行路径 |
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
