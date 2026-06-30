# 群体 AI 开发功能需求与架构设计

最后更新：2026-06-30

本文把 `群体ai开发.md` 中关于 Agent 协作网络的讨论，整理成适合一龙项目落地的产品需求和技术架构。核心判断是：这项能力不应该做成普通 AI 群聊，而应该做成项目空间上的“多用户、多节点、多类型 AI 开发调度层”，复用一龙已有的 PC 节点、项目成员、项目频道、真实 Git 工作区、任务日志、审批、计费和验收链路。

## 1. 目标定位

一龙项目已经支持用户把自己的 PC 注册成节点，并让自己的 Codex、Copilot、Claude、Gemini 或 API key 参与项目开发。因此“群体 AI 开发”的正确形态不是“一个用户叫多个 AI”，而是“一个项目里的多个成员，把各自授权的 PC 节点和 AI 能力组织成一个临时开发团队”：

```text
项目成员提出目标
  -> 系统生成可验收事项 Matter
  -> 选择多个用户贡献的 Bot / Agent
  -> 分配到多个用户的本机 PC 节点或授权远程节点
  -> 每个 Agent 在隔离 worktree 中执行
  -> 不同类型 AI 之间审核、交接、汇总
  -> 项目负责人或事项负责人在关键节点批准和验收
  -> 上下文、偏好、技能、节点质量沉淀回项目
```

一句话：**多个用户贡献节点和 AI，多种 AI 类型共同执行，项目空间负责把它们组织成一个可追踪、可验收、可计费、可复盘的开发团队。**

## 1.1 必须成立的多主体模型

这份需求的主语必须同时包含三类“多”：

1. 多用户：项目 owner、协作者、节点提供者、审核者、验收者可以是不同用户。
2. 多节点：同一个 Matter 可以同时使用 owner 自己的 PC、协作者的 PC、公开授权的远程 PC 节点。
3. 多类型 AI：Codex、Copilot、Claude、Gemini、本机 API runtime、平台 AI 可以在同一个 Matter 中承担不同角色。

所以后续所有设计都要区分这些身份：

| 身份 | 含义 | 关键权限 |
|---|---|---|
| requester_user_id | 发起 Matter 的项目成员 | 创建计划、查看自己可见结果 |
| project_owner_id | 项目所有者 | 管理成员、节点策略、最终合并策略 |
| provider_user_id | 提供 PC 节点和 AI 能力的用户 | 授权节点、收取收益、保护本机隐私 |
| assignee_bot_id | 被派发任务的 Bot | 只能在授权范围内执行 |
| reviewer_user_id / reviewer_bot_id | 审核者，可以是人或 AI | 提交审查意见，不直接替决策人验收 |
| decision_user_id | 最终批准或打回的人 | 批准计划、合并、验收或取消 |

MVP 可以限制在“同一个项目内的成员和他们授权的节点”，但不能把产品定义成单用户功能。

## 2. 现有系统基础

这项功能应优先复用以下现有能力：

| 现有能力 | 当前入口 | 在群体 AI 开发中的作用 |
|---|---|---|
| PC 节点注册和发现 | `/api/me/nodes`、`/api/nodes`、`node_credentials`、`node_registry` | 找到项目成员自己的节点和可用远程节点 |
| 节点 AI 能力 | `allowed_clis`、`NodeDevRuntimeProfile`、`pc_agent_runtime_choice.rs` | 判断 Codex/Copilot/API runtime/平台 AI 是否可用 |
| 用户项目工作区 | `projects.node_id + workspace_path`、`project_workspace_*` | 确定项目实际在哪台 PC、哪个目录执行 |
| 本机执行路线 | Route A/B/C/C2/C3 | 支持本机 CLI、本机 API key、平台 AI、远程节点能力 |
| 项目频道 | `project_channels`、`project_channel_messages`、`reply_to_message_id` | 承载讨论、计划卡、进度卡、验收卡 |
| 任务系统 | `tasks`、`task_events`、`project_task_scheduler.rs` | 追踪用户可见任务状态 |
| 本机任务日志 | `.elon/agent-runs/*.jsonl`、`/api/project-agent-runs` | 回放 Agent 执行、恢复中断任务、展示工具调用摘要 |
| 上下文系统 | `context_compiler`、repo map、symbol index、task pack | 给不同 Agent 分发合适的项目上下文 |
| 项目成员和权限 | project members、频道权限、`can_start_ai` | 判断谁能发起、审批、查看和验收 |
| 节点计费和证明 | `node_compute_runs`、`node_transactions` | 记录跨用户节点执行证明、成本和收益 |

不建议另建一套脱离项目空间和 PC 节点的“AI 协作平台”。那会绕开已有鉴权、工作区、安全审批、任务恢复和计费闭环。

## 3. 概念映射

| 讨论概念 | 一龙项目落地概念 | 说明 |
|---|---|---|
| Bot | Agent Profile / Worker Bot | 可被调度的数字开发者，绑定提供者用户、节点、运行路线、能力和历史表现 |
| Channel | Project Channel | 项目里的协作现场，承载讨论、任务、进度和验收 |
| Thread | Work Thread | 围绕一个事项的消息串，MVP 可先复用 `reply_to_message_id` |
| Matter | Project AI Matter | 从讨论转成可执行事项，有发起人、负责人、参与 Bot、产物、验收标准和状态 |
| Context | Project Context Pack | 项目文档、repo map、符号索引、历史决策和任务相关文件 |
| Taste | Project / User Preference | 项目成员验收、打回理由、风格偏好、架构偏好 |
| Skill | AI-to-AI Skill Package | 面向总调度 AI 和 Worker Bot 的机器可读能力包，声明适用意图、输入输出、权限、成本、测试和兼容性 |
| Demo Oracle | 预言家 AI / Demo Oracle | 在正式 Matter 和 Skill 执行前，用低成本把模糊需求生成可讨论 demo、草图和用户流程 |
| Skill Router | Skill Router | 根据需求、项目上下文、成功率、成本、风险和兼容性选择 Skill 组合 |
| Orchestration | Group AI Coordinator | 负责任务拆解、派发、审核、合并、验收汇报 |

## 4. 功能需求

### R1. Agent 身份与能力卡

每个参与项目开发的 AI 都应该有身份，而不是只是一个模型名称。这个身份必须同时说明“谁提供的、在哪台节点上跑、用哪种 AI、能做什么”。

需要记录：

- `bot_id`：项目内稳定 ID。
- `display_name`：例如“Codex 实现者”“Copilot 审核员”“Claude 文档员”。
- `provider_user_id`：提供这个 Bot/节点能力的用户。
- `node_id`：默认运行节点。
- `runtime_route`：Route A / Route B / Route C / Route C2 / Route C3。
- `cli_name`：codex / copilot / claude / gemini / api-runtime / server-runtime。
- `capabilities`：实现、审查、测试、文档、调研、构建、发布说明等。
- `risk_level`：只读、可写、可运行命令、可发起合并。
- `history_stats`：成功率、耗时、最近失败原因、用户评分。

MVP 可以先根据项目成员已授权节点的 `allowed_clis` 自动生成临时 Bot，不急着做完整 Bot 市场。

### R2. Matter 事项

项目成员在项目频道提出需求后，系统先生成 Matter 草案，让有权限的人确认后再执行。

Matter 至少包含：

- 用户原始需求。
- 发起人、所属项目、可见频道。
- 可参与用户、可用节点、可用 AI 类型。
- 系统整理后的目标和边界。
- 建议协作模式。
- 子任务列表。
- 推荐 Bot、节点、节点提供者和执行角色。
- 每个子任务的验收标准。
- 预计要修改或检查的模块。
- 需要运行的验证命令。
- 风险提示和需要用户确认的点。

状态流：

```text
draft
  -> plan_ready
  -> approved
  -> running
  -> review
  -> changes_requested
  -> accepted
  -> closed
```

失败和取消状态：

```text
running -> failed
running -> canceled
review  -> canceled
```

### R3. 六种协作模式

| 模式 | 项目场景 | MVP 优先级 |
|---|---|---|
| Solo | 一个 Bot 完成边界清晰的小任务 | P0 |
| Critic | 一个 Bot 实现，另一个 Bot 独立审查 | P0 |
| Split | 大任务按模块拆给多个用户/节点/Bot 并行完成 | P0 |
| Pipeline | 需求分析、实现、测试、文档、验收稿按顺序交接 | P1 |
| Roundtable | 多 Bot 先讨论技术方案，再由协调器收束 | P1 |
| Swarm | 多 Bot 独立给方案或实现，最后择优 | P2 |

MVP 先做 Solo、Critic、Split，因为它们最贴合真实代码开发，也最容易用 Git worktree 做隔离。其中 Split 必须支持不同用户贡献的不同节点共同执行，而不是只在一台电脑上开多个 Bot。

### R4. 隔离 worktree

群体 AI 开发必须强制隔离执行。隔离单位不是只有 Bot，还包括用户、节点、Matter 和子任务：

```text
ELON_NODE_WORKSPACE_ROOT/
  usr_xxx/
    prj_xxx/
      repo/                 # 项目主仓库，只做基线和最终合并入口
      worktrees/
        matter_xxx_user_a_node_a_bot_a/   # 用户 A 的节点/Bot 执行目录
        matter_xxx_user_b_node_b_bot_b/   # 用户 B 的节点/Bot 执行目录
      artifacts/
      logs/
```

硬规则：

1. Bot 不直接修改项目主工作区 `repo/`。
2. 每个 Bot/节点/子任务有独立 worktree、分支、日志和产物目录。
3. Split 模式需要先规划文件或模块所有权，避免两个 Bot 同改同一块。
4. Critic 默认只读实现者的 diff、测试结果和执行日志。
5. 合并必须由协调器控制，并在频道里展示 diff、冲突和验证结果。

### R5. 人类决策节点

系统应该坚持“Agents do, Humans decide”：

- Matter 计划需要具备权限的项目成员批准。
- 写文件、patch、运行命令、高风险 Git 操作继续走现有工具审批。
- 合并前展示 diff、测试结果、审查意见和剩余风险。
- 事项负责人可以接受、打回、换 Bot、换节点、缩小范围或取消。
- 不同用户的打回理由、验收偏好和节点质量反馈要沉淀为项目记忆。

### R6. 上下文分发

不同 Bot 不应该拿同一份巨大上下文。

上下文来源：

- 项目文档频道和固定 AI 文档。
- `context_compiler` 生成的 repo map、符号索引、task pack。
- Matter 的 Brief、验收标准和文件边界。
- 相关频道消息、项目成员意见和用户偏好。
- 其他 Bot 的输出摘要、diff、测试结果。

分发策略：

- 实现 Bot：拿目标文件、相关符号、测试建议、编码规范。
- 审查 Bot：拿计划、diff、验收标准、测试输出。
- 汇总 Bot：拿子任务摘要、冲突点、产物清单。
- Roundtable Bot：拿问题和约束，不拿其他 Bot 的未提交草稿。

### R7. 审核与验收

每个 Matter 都要留下机器可读的验收记录：

- `acceptance_criteria`：验收标准。
- `verification_commands`：实际运行的命令。
- `review_findings`：审查发现、严重程度、关联文件。
- `artifacts`：patch、报告、APK、截图、日志、构建产物。
- `final_decision`：accepted / rejected / accepted_with_risks。
- `decision_by`：最终决策用户。

### R8. 经验沉淀

每次协作沉淀三类资产：

- Context：项目背景、历史决策、架构约束、失败复盘。
- Taste：不同项目成员喜欢或拒绝的实现风格、UI 风格、命名习惯、审核偏好。
- Skill：可复用任务模板、测试清单、发布清单、项目专属 agent 指令。

MVP 可以先写入项目作用域 `user_memories` 或项目文档；后续再独立做 `project_ai_preferences` 和 `project_ai_skills`。项目内沉淀的 Skill 经过脱敏、测试、权限审计和作者确认后，才允许升级为平台 Skill；不能自动把用户私有项目经验公开到市场。

### R9. 预言家 AI 与 demo 预演

当用户需求仍处于讨论阶段、存在多个方向、正式开发成本较高或用户明确要求先看效果时，总调度 AI 应先调用预言家 AI，而不是直接创建重型开发任务。

预言家 AI 的输入：

- 总调度 AI 整理后的产品目标、目标用户、核心场景和不确定点。
- 当前项目已有页面、设计规范和可复用组件摘要。
- 可用官方 Skill 的能力摘要，仅用于提出后续建议，不直接执行正式 Skill。
- 本轮 demo 的预算、时间、允许产物类型和禁止事项。

预言家 AI 的输出：

- 一句话产品定位和核心用户流程。
- 3 至 5 个关键页面或状态。
- 静态 HTML/前端 mock、截图式草图、流程图或可点击轻原型之一。
- 假数据和未实现能力的明确标记。
- 需要用户确认的问题。
- 建议的 Skill 候选、选择理由和预计成本区间。

硬边界：

1. demo 写入独立临时目录或 artifact，不进入项目正式分支。
2. 默认不连接真实数据库、支付、生产密钥和外部用户数据。
3. demo 通过不等于正式功能完成，用户确认后必须重新生成 Matter 和验收标准。
4. 小范围、目标明确的修改可以跳过预言家 AI，避免增加等待和 token 成本。
5. 预言家 AI 只提供可讨论证据，最终产品方向仍由用户决定。

### R10. AI-to-AI Skill 路由

Skill 由机器可读 manifest 和执行说明组成，至少声明：

- Skill ID、版本、作者和兼容范围。
- `intents`、`capabilities`、输入 schema、输出 artifact 类型。
- 所需权限、允许工具、风险等级和数据边界。
- 预计 token、节点、构建和第三方 API 成本。
- 前置条件、冲突领域和可组合 Skill。
- 验收标准、测试入口、历史成功率和失败原因。

Skill Router 不能只按关键词匹配。推荐评分维度为：需求匹配、项目兼容、历史成功率、与其他 Skill 的冲突、预算、权限、节点可用性和风险。MVP 只路由官方 Skill，路由结果写入 Matter 并向用户解释“选择了什么、为什么、预计会产生什么结果”。

## 5. 架构设计

### 5.1 总体架构

```text
APK / PC 项目频道
  -> Rust API Server
  -> Group AI Coordinator
       -> Product Discussion / Requirement Maturity
       -> Demo Oracle（按需）
       -> Matter Planner
       -> Skill Registry / Skill Router
       -> Member / Permission Resolver
       -> Node / Bot Selector
       -> Context Pack Builder
       -> Worktree Allocator
       -> Dispatch Adapter
       -> Review / Merge Controller
  -> PC Relay
  -> User A PC Node / User B PC Node / Remote Authorized Node
       -> isolated worktree
       -> Route A/B/C/C2/C3 runtime
       -> tool approval
       -> task journal / agent-runs
  -> Project channel / task / artifact / billing records
```

### 5.2 后端模块边界

新增功能不要塞进 `project_space.rs`、`project_api.rs`、`node_api.rs`。建议新建领域目录：

```text
server/src/group_ai/
  mod.rs              # re-export 和路由组装
  types.rs            # Matter、Bot、Run、Review DTO
  store.rs            # 表读写
  permissions.rs      # 项目成员、节点提供者、可见性和审批权限
  planner.rs          # 从频道消息生成 Matter 计划
  bot_selector.rs     # 按成员授权、节点状态、AI 类型、成本选择 Bot
  context.rs          # 调用 context_compiler 生成上下文
  worktree.rs         # 跨节点 worktree 分配与合并预览协议
  dispatcher.rs       # 调用现有 node/agent dispatch
  reviewer.rs         # Critic、验收摘要、风险整理
  api.rs              # HTTP handlers
```

PC 前端后续进入 `pc-frontend/`：

```text
pc-frontend/src/features/group-ai/
  MatterList.tsx
  MatterDetail.tsx
  BotPicker.tsx
  CollaborationModeControl.tsx
  ReviewPanel.tsx
```

旧 `server/src/assets/pc_*.js` 只做兼容桥接，不继续堆复杂交互。

### 5.3 数据模型草案

建议新增表：

```sql
project_ai_bots(
  id TEXT PRIMARY KEY,
  project_id TEXT NOT NULL,
  provider_user_id TEXT NOT NULL,
  node_id TEXT,
  display_name TEXT NOT NULL,
  runtime_route TEXT NOT NULL,
  cli_name TEXT,
  capabilities_json TEXT NOT NULL,
  risk_level TEXT NOT NULL,
  enabled INTEGER NOT NULL DEFAULT 1,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);

project_ai_matters(
  id TEXT PRIMARY KEY,
  project_id TEXT NOT NULL,
  channel_id TEXT NOT NULL,
  requester_user_id TEXT NOT NULL,
  decision_user_id TEXT,
  source_message_id TEXT,
  title TEXT NOT NULL,
  brief TEXT NOT NULL,
  collaboration_mode TEXT NOT NULL,
  status TEXT NOT NULL,
  participant_user_ids_json TEXT NOT NULL,
  node_policy_json TEXT NOT NULL,
  acceptance_criteria_json TEXT NOT NULL,
  plan_json TEXT NOT NULL,
  final_summary TEXT,
  final_decision TEXT,
  decided_by_user_id TEXT,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);

project_ai_matter_assignments(
  id TEXT PRIMARY KEY,
  matter_id TEXT NOT NULL,
  bot_id TEXT NOT NULL,
  assignee_user_id TEXT,
  provider_user_id TEXT,
  node_id TEXT,
  role TEXT NOT NULL,
  runtime_route TEXT NOT NULL,
  cli_name TEXT,
  worktree_path TEXT,
  branch_name TEXT,
  status TEXT NOT NULL,
  result_summary TEXT,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);

project_ai_reviews(
  id TEXT PRIMARY KEY,
  matter_id TEXT NOT NULL,
  reviewer_bot_id TEXT,
  reviewer_user_id TEXT,
  target_assignment_id TEXT,
  severity TEXT NOT NULL,
  finding_json TEXT NOT NULL,
  status TEXT NOT NULL,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);
```

复用现有表：

- `tasks`：用户可见任务状态。
- `project_channel_messages`：计划卡、进度卡、审核卡、验收卡。
- `artifacts`：patch、报告、构建产物。
- `node_compute_runs`：跨用户节点执行证明、成本和收益。
- `node_transactions`：节点提供者结算。
- `user_memories`：项目作用域和成员偏好。

### 5.4 API 草案

```http
GET  /api/projects/{project_id}/ai/bots
POST /api/projects/{project_id}/ai/bots
GET  /api/projects/{project_id}/ai/available-nodes
POST /api/projects/{project_id}/ai/node-authorizations

POST /api/projects/{project_id}/ai/matters/plan
GET  /api/projects/{project_id}/ai/matters
GET  /api/projects/{project_id}/ai/matters/{matter_id}
POST /api/projects/{project_id}/ai/matters/{matter_id}/approve
POST /api/projects/{project_id}/ai/matters/{matter_id}/start
POST /api/projects/{project_id}/ai/matters/{matter_id}/request-changes
POST /api/projects/{project_id}/ai/matters/{matter_id}/accept
POST /api/projects/{project_id}/ai/matters/{matter_id}/cancel
```

PC 节点本地优先复用：

- `/api/project-agent-runs`
- 工具审批接口
- task journal 和恢复接口
- 项目工作区 provision / inspect / health 接口

后续需要显式 worktree 分配时，再增加本机受保护接口：

```http
POST /api/project-worktrees/allocate
POST /api/project-worktrees/merge-preview
POST /api/project-worktrees/dispose
```

## 6. MVP 路线

### 阶段 0：产品边界

- 明确群体 AI 开发只进入项目空间，不进入普通聊天。
- 明确 Matter、Bot、协作模式、验收卡的产品语言。
- 明确 PC 节点执行和 worktree 隔离是硬规则。
- 明确一龙会话主 AI 是长期对话 owner；预言家 AI 和 Skill Agent 都是受限旁路角色。

### 阶段 0.5：预言家 AI 与官方 Skill 试验

- 定义 demo artifact 契约、临时存储位置、预算和安全边界。
- 先支持静态 HTML 或现有前端组件组成的可点击 demo，不做真实后端。
- 建立 3 至 5 个官方 Skill manifest，覆盖页面原型、项目初始化、UI 实现、测试和发布检查。
- 让总调度 AI 输出需求成熟度，并决定跳过 demo、生成 demo 或继续讨论。
- demo 经用户确认后再创建 Matter；记录确认、打回和放弃原因。
- Skill Router 只给出官方 Skill 建议，不开放第三方交易。

### 阶段 1：单项目、多成员授权节点、多 Bot

范围：

- 限制在一个项目内，但允许项目成员把自己的在线 PC 节点授权给该项目。
- Bot 从所有已授权节点的 `allowed_clis` 自动生成。
- Matter 记录 requester、provider、assignee、decision maker。
- 支持 Solo、Critic、Split。
- Matter 计划必须人工批准。
- 执行日志继续走 `.elon/agent-runs`。
- 结果展示在项目频道。

最小闭环：

```text
频道消息
  -> 生成 Matter 计划
  -> 有权限项目成员批准
  -> 分配 Bot 和 worktree
  -> 执行
  -> 审查
  -> 汇总 diff/测试/风险
  -> 决策人验收
```

### 阶段 2：跨节点与远程节点

- 从“项目成员授权节点”扩展到“公开或半公开远程节点”。
- Route C2 / Route C3 进入 Bot 选择。
- 引入节点 owner 授权、价格、容量、可见性和隐私边界。
- 远程节点收益进入 `node_compute_runs` 和结算流水。

### 阶段 3：复杂协作模式

- Roundtable：多 Bot 给方案，协调器收束计划。
- Pipeline：需求、实现、测试、文档、发布说明分阶段交接。
- Swarm：多 Bot 独立方案竞选，用户或评审 Bot 选择。

### 阶段 4：偏好与技能沉淀

- 项目成员验收和打回理由结构化进入项目偏好。
- 成功流程沉淀为项目 Skill。
- 新 Bot 加入项目时自动读取项目 Taste / Skill。

### 阶段 5：Skill 仓库与分发

- 把验证通过的官方 Skill 升级为版本化 Skill Registry。
- 增加安装、依赖、兼容性、安全审核、调用记录和质量评分。
- 邀请制开放创作者提交，先审核后上架。
- 支持按次、订阅或项目授权计费，模型/节点成本与 Skill 价值分开记录。
- 长期将 Skill 生成的应用、模板和插件接入版本分发、更新和二次创作体系。

## 7. 权限、安全和成本

1. 项目成员必须有 `can_start_ai` 才能创建 Matter 或启动 Bot。
2. 节点提供者必须显式授权节点可被该项目、该协作模式或该任务使用。
3. 只有 owner、manager、Matter 创建者或被授予 decision 权限的人可以批准计划、验收结果、取消任务。
4. Bot 不拥有用户身份，只拥有受限代理身份；每次操作都必须能追溯到 Matter、项目、节点、provider 和 requester。
5. PC 节点只执行绑定项目的受控路径，不接受任意客户端传来的本地路径。
6. Route B/C 的写文件、patch、命令执行继续走现有工具审批；工具审批由节点所在用户或其授权策略决定。
7. 跨用户远程节点默认只暴露工作区任务，不暴露 provider 的本地绝对路径、密钥、prompt 全文或完整命令输出。
8. 每次节点执行都写入 `node_compute_runs`，包含 requester、provider、feature、usage_mode、状态、耗时、token、成本和失败原因。
9. 节点断线、任务超时、审批 waiter 丢失时，只能基于 journal/快照继续新任务，不能伪装成原任务仍可直接审批。

## 8. 风险与应对

| 风险 | 应对 |
|---|---|
| 多 Bot 同改文件导致冲突 | 强制 worktree 隔离、文件所有权计划、合并前冲突检查 |
| 多用户权限混乱 | Matter 记录 requester/provider/decision_user，节点授权独立于项目成员权限 |
| Agent 互相复制错误结论 | Critic 独立读 diff 和证据，不直接相信实现者总结 |
| 上下文泄露到远程节点 | 按角色裁剪 context pack，敏感文件和密钥路径 fail-closed |
| 成本失控 | 每个 Matter 设预算，Roundtable/Swarm 默认需要确认 |
| 用户被 AI 消息刷屏 | 频道默认展示计划卡、进度卡、验收卡，详细日志折叠 |
| 自动合并破坏项目 | MVP 不自动发布，合并前必须展示 diff 和验证结果 |
| 节点离线导致任务卡死 | 使用 task journal、恢复入口、stale task 处理 |

## 9. 验收标准

MVP 完成时应满足：

1. 项目成员能在项目频道把一条需求转换成 Matter 计划。
2. 系统能展示建议 Bot、节点、节点提供者、AI 类型、协作模式、子任务和验收标准。
3. 有权限项目成员批准后，至少两个来自不同 AI 类型或不同节点的 Bot 能在隔离 worktree 中完成 Split 或 Critic 流程。
4. 频道里能看到计划、执行进度、审查意见、最终结果和验收卡。
5. PC 节点断线或任务中断后，用户能看到恢复建议。
6. 后端能追踪 Matter -> participant user -> assignment -> node run -> artifact -> final decision。
7. 所有写文件和运行命令仍走现有工具审批边界。
8. 新增代码遵守模块化规则，不把群体 AI 逻辑塞进已有大型入口文件。

## 10. 推荐第一批开发任务

1. 新建 `server/src/group_ai/` 模块和只读 DTO。
2. 增加 Matter 草案表和 API：从频道消息生成计划草案。
3. 增加项目成员节点授权查询：列出该项目可用的用户、节点、AI 类型。
4. 在项目频道插入 `matter_plan` 类型消息卡。
5. 从项目授权节点的 `allowed_clis` 生成临时 Bot 列表。
6. 支持有权限项目成员批准 Matter，创建 assignments。
7. 接入现有 PC 节点 dispatch，先跑 Solo。
8. 增加 Critic：实现完成后派发只读审查任务。
9. 增加 Split：按计划给不同用户/节点/Bot 创建 assignment 和 worktree。
10. 汇总 diff、测试、日志和审查意见，生成验收卡。
11. 把用户最终接受或打回写入 Matter、项目偏好和节点质量反馈。

## 11. 最终形态

最终的一龙项目空间应该像一个“多个用户共同贡献节点和 AI 的开发组织”：

- 多个用户的 PC 是工作区和执行资源。
- 多种 AI/CLI 是数字开发者。
- 项目频道是协作现场。
- Matter 是可追踪的交付单。
- Worktree 是跨用户、跨节点、跨 Bot 的隔离施工区。
- 审查和验收是质量闸门。
- Context、Taste、Skill 是越用越强的项目资产。

这样，“群体 AI 开发”才会真正贴合一龙项目：一个项目不只是让一个 AI 回答问题，而是让多个用户授权的多台 PC 节点、多种 AI 能力共同把项目往前推进。
