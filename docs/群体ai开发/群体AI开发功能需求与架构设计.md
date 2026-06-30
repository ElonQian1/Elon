# 群体 AI 开发功能需求与架构设计

最后更新：2026-06-30

本文把 `群体ai开发.md` 中关于 Agent 协作网络的讨论，整理成适合一龙项目落地的产品需求和技术架构。核心判断是：这项能力不应该做成普通 AI 群聊，而应该做成项目空间上的“多 Agent 开发调度层”，复用一龙已有的 PC 节点、项目频道、真实 Git 工作区、任务日志、审批和验收链路。

## 1. 目标定位

一龙项目已经支持用户把自己的 PC 注册成节点，并让自己的 Codex、Copilot、Claude、Gemini 或 API key 参与项目开发。因此“群体 AI 开发”的正确形态是：

```text
用户提出项目目标
  -> 系统生成可验收事项 Matter
  -> 选择多个 Bot / Agent
  -> 分配到用户自己的 PC 节点或授权远程节点
  -> 每个 Agent 在隔离 worktree 中执行
  -> Agent 之间审核、交接、汇总
  -> 用户在关键节点批准和验收
  -> 上下文、偏好、技能沉淀回项目
```

一句话：**用户拥有 PC 节点和 AI，项目空间负责把这些 AI 组织成一个可追踪、可验收、可复盘的开发团队。**

## 2. 现有系统基础

这项功能应优先复用以下现有能力：

| 现有能力 | 当前入口 | 在群体 AI 开发中的作用 |
|---|---|---|
| PC 节点注册和发现 | `/api/me/nodes`、`/api/nodes`、`node_credentials`、`node_registry` | 找到用户自己的节点和可用远程节点 |
| 节点 AI 能力 | `allowed_clis`、`NodeDevRuntimeProfile`、`pc_agent_runtime_choice.rs` | 判断 Codex/Copilot/API runtime/平台 AI 是否可用 |
| 用户项目工作区 | `projects.node_id + workspace_path`、`project_workspace_*` | 确定项目实际在哪台 PC、哪个目录执行 |
| 本机执行路线 | Route A/B/C/C2/C3 | 支持本机 CLI、本机 API key、平台 AI、远程节点能力 |
| 项目频道 | `project_channels`、`project_channel_messages`、`reply_to_message_id` | 承载讨论、计划卡、进度卡、验收卡 |
| 任务系统 | `tasks`、`task_events`、`project_task_scheduler.rs` | 追踪用户可见任务状态 |
| 本机任务日志 | `.elon/agent-runs/*.jsonl`、`/api/project-agent-runs` | 回放 Agent 执行、恢复中断任务、展示工具调用摘要 |
| 上下文系统 | `context_compiler`、repo map、symbol index、task pack | 给不同 Agent 分发合适的项目上下文 |
| 节点计费和证明 | `node_compute_runs`、`node_transactions` | 记录远程节点执行证明、成本和收益 |

不建议另建一套脱离项目空间和 PC 节点的“AI 协作平台”。那会绕开已有鉴权、工作区、安全审批、任务恢复和计费闭环。

## 3. 概念映射

| 讨论概念 | 一龙项目落地概念 | 说明 |
|---|---|---|
| Bot | Agent Profile / Worker Bot | 可被调度的数字开发者，绑定节点、运行路线、能力和历史表现 |
| Channel | Project Channel | 项目里的协作现场，承载讨论、任务、进度和验收 |
| Thread | Work Thread | 围绕一个事项的消息串，MVP 可先复用 `reply_to_message_id` |
| Matter | Project AI Matter | 从讨论转成可执行事项，有负责人、产物、验收标准和状态 |
| Context | Project Context Pack | 项目文档、repo map、符号索引、历史决策和任务相关文件 |
| Taste | Project / User Preference | 用户验收、打回理由、风格偏好、架构偏好 |
| Skill | Reusable Task Skill | 可复用的任务模板、测试清单、发布清单、项目规范 |
| Orchestration | Group AI Coordinator | 负责任务拆解、派发、审核、合并、验收汇报 |

## 4. 功能需求

### R1. Agent 身份与能力卡

每个参与项目开发的 AI 都应该有身份，而不是只是一个模型名称。

需要记录：

- `bot_id`：项目内稳定 ID。
- `display_name`：例如“Codex 实现者”“Copilot 审核员”“Claude 文档员”。
- `owner_user_id`：Bot 所属用户。
- `node_id`：默认运行节点。
- `runtime_route`：Route A / Route B / Route C / Route C2 / Route C3。
- `cli_name`：codex / copilot / claude / gemini / api-runtime / server-runtime。
- `capabilities`：实现、审查、测试、文档、调研、构建、发布说明等。
- `risk_level`：只读、可写、可运行命令、可发起合并。
- `history_stats`：成功率、耗时、最近失败原因、用户评分。

MVP 可以先根据当前项目绑定 PC 节点的 `allowed_clis` 自动生成临时 Bot，不急着做完整 Bot 市场。

### R2. Matter 事项

用户在项目频道提出需求后，系统先生成 Matter 草案，让用户确认后再执行。

Matter 至少包含：

- 用户原始需求。
- 系统整理后的目标和边界。
- 建议协作模式。
- 子任务列表。
- 推荐 Bot 和节点。
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
| Split | 大任务按模块拆给多个 Bot 并行完成 | P0 |
| Pipeline | 需求分析、实现、测试、文档、验收稿按顺序交接 | P1 |
| Roundtable | 多 Bot 先讨论技术方案，再由协调器收束 | P1 |
| Swarm | 多 Bot 独立给方案或实现，最后择优 | P2 |

MVP 先做 Solo、Critic、Split，因为它们最贴合真实代码开发，也最容易用 Git worktree 做隔离。

### R4. 隔离 worktree

群体 AI 开发必须强制隔离执行：

```text
ELON_NODE_WORKSPACE_ROOT/
  usr_xxx/
    prj_xxx/
      repo/                 # 项目主仓库，只做基线和最终合并入口
      worktrees/
        matter_xxx_bot_a/   # Bot A 的执行目录
        matter_xxx_bot_b/   # Bot B 的执行目录
      artifacts/
      logs/
```

硬规则：

1. Bot 不直接修改项目主工作区 `repo/`。
2. 每个 Bot/子任务有独立 worktree、分支、日志和产物目录。
3. Split 模式需要先规划文件或模块所有权，避免两个 Bot 同改同一块。
4. Critic 默认只读实现者的 diff、测试结果和执行日志。
5. 合并必须由协调器控制，并在频道里展示 diff、冲突和验证结果。

### R5. 人类决策节点

系统应该坚持“Agents do, Humans decide”：

- Matter 计划需要用户批准。
- 写文件、patch、运行命令、高风险 Git 操作继续走现有工具审批。
- 合并前展示 diff、测试结果、审查意见和剩余风险。
- 用户可以接受、打回、换 Bot、缩小范围或取消。
- 用户的打回理由和验收偏好要沉淀为项目记忆。

### R6. 上下文分发

不同 Bot 不应该拿同一份巨大上下文。

上下文来源：

- 项目文档频道和固定 AI 文档。
- `context_compiler` 生成的 repo map、符号索引、task pack。
- Matter 的 Brief、验收标准和文件边界。
- 相关频道消息和用户偏好。
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
- `decision_by`：验收人。

### R8. 经验沉淀

每次协作沉淀三类资产：

- Context：项目背景、历史决策、架构约束、失败复盘。
- Taste：用户喜欢或拒绝的实现风格、UI 风格、命名习惯。
- Skill：可复用任务模板、测试清单、发布清单、项目专属 agent 指令。

MVP 可以先写入项目作用域 `user_memories` 或项目文档；后续再独立做 `project_ai_preferences` 和 `project_ai_skills`。

## 5. 架构设计

### 5.1 总体架构

```text
APK / PC 项目频道
  -> Rust API Server
  -> Group AI Coordinator
       -> Matter Planner
       -> Bot Selector
       -> Context Pack Builder
       -> Worktree Allocator
       -> Dispatch Adapter
       -> Review / Merge Controller
  -> PC Relay
  -> User PC Node
       -> isolated worktree
       -> Route A/B/C runtime
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
  planner.rs          # 从频道消息生成 Matter 计划
  bot_selector.rs     # 按能力、节点状态、成本选择 Bot
  context.rs          # 调用 context_compiler 生成上下文
  worktree.rs         # worktree 分配与合并预览协议
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
  owner_user_id TEXT NOT NULL,
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
  created_by_user_id TEXT NOT NULL,
  source_message_id TEXT,
  title TEXT NOT NULL,
  brief TEXT NOT NULL,
  collaboration_mode TEXT NOT NULL,
  status TEXT NOT NULL,
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
  node_id TEXT,
  role TEXT NOT NULL,
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
- `node_compute_runs`：节点执行证明、成本和收益。
- `user_memories`：项目作用域偏好。

### 5.4 API 草案

```http
GET  /api/projects/{project_id}/ai/bots
POST /api/projects/{project_id}/ai/bots

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

### 阶段 1：单项目、单用户、多 Bot

范围：

- 只允许项目 owner 使用自己的在线 PC 节点。
- Bot 从该节点的 `allowed_clis` 自动生成。
- 支持 Solo、Critic、Split。
- Matter 计划必须人工批准。
- 执行日志继续走 `.elon/agent-runs`。
- 结果展示在项目频道。

最小闭环：

```text
频道消息
  -> 生成 Matter 计划
  -> 用户批准
  -> 分配 Bot 和 worktree
  -> 执行
  -> 审查
  -> 汇总 diff/测试/风险
  -> 用户验收
```

### 阶段 2：跨节点与远程节点

- 允许用户选择其他在线 PC 节点作为执行者。
- Route C2 / Route C3 进入 Bot 选择。
- 引入节点 owner 授权、价格、容量、可见性和隐私边界。
- 远程节点收益进入 `node_compute_runs` 和结算流水。

### 阶段 3：复杂协作模式

- Roundtable：多 Bot 给方案，协调器收束计划。
- Pipeline：需求、实现、测试、文档、发布说明分阶段交接。
- Swarm：多 Bot 独立方案竞选，用户或评审 Bot 选择。

### 阶段 4：偏好与技能沉淀

- 用户验收和打回理由结构化进入项目偏好。
- 成功流程沉淀为项目 Skill。
- 新 Bot 加入项目时自动读取项目 Taste / Skill。

## 7. 权限、安全和成本

1. 项目成员必须有 `can_start_ai` 才能创建 Matter 或启动 Bot。
2. 只有 owner、manager 或 Matter 创建者可以批准计划、验收结果、取消任务。
3. Bot 不拥有用户身份，只拥有受限代理身份；每次操作都必须能追溯到 Matter、项目、节点和触发用户。
4. PC 节点只执行绑定项目的受控路径，不接受任意客户端传来的本地路径。
5. Route B/C 的写文件、patch、命令执行继续走现有工具审批。
6. 跨用户远程节点默认只暴露工作区任务，不暴露 provider 的本地绝对路径、密钥、prompt 全文或完整命令输出。
7. 每次节点执行都写入 `node_compute_runs`，包含 feature、usage_mode、状态、耗时、token、成本和失败原因。
8. 节点断线、任务超时、审批 waiter 丢失时，只能基于 journal/快照继续新任务，不能伪装成原任务仍可直接审批。

## 8. 风险与应对

| 风险 | 应对 |
|---|---|
| 多 Bot 同改文件导致冲突 | 强制 worktree 隔离、文件所有权计划、合并前冲突检查 |
| Agent 互相复制错误结论 | Critic 独立读 diff 和证据，不直接相信实现者总结 |
| 上下文泄露到远程节点 | 按角色裁剪 context pack，敏感文件和密钥路径 fail-closed |
| 成本失控 | 每个 Matter 设预算，Roundtable/Swarm 默认需要确认 |
| 用户被 AI 消息刷屏 | 频道默认展示计划卡、进度卡、验收卡，详细日志折叠 |
| 自动合并破坏项目 | MVP 不自动发布，合并前必须展示 diff 和验证结果 |
| 节点离线导致任务卡死 | 使用 task journal、恢复入口、stale task 处理 |

## 9. 验收标准

MVP 完成时应满足：

1. 用户能在项目频道把一条需求转换成 Matter 计划。
2. 系统能展示建议 Bot、节点、协作模式、子任务和验收标准。
3. 用户批准后，至少两个 Bot 能在隔离 worktree 中完成 Split 或 Critic 流程。
4. 频道里能看到计划、执行进度、审查意见、最终结果和验收卡。
5. PC 节点断线或任务中断后，用户能看到恢复建议。
6. 后端能追踪 Matter -> assignment -> node run -> artifact -> final decision。
7. 所有写文件和运行命令仍走现有工具审批边界。
8. 新增代码遵守模块化规则，不把群体 AI 逻辑塞进已有大型入口文件。

## 10. 推荐第一批开发任务

1. 新建 `server/src/group_ai/` 模块和只读 DTO。
2. 增加 Matter 草案表和 API：从频道消息生成计划草案。
3. 在项目频道插入 `matter_plan` 类型消息卡。
4. 从当前项目绑定 PC 节点的 `allowed_clis` 生成临时 Bot 列表。
5. 支持用户批准 Matter，创建 assignments。
6. 接入现有 PC 节点 dispatch，先跑 Solo。
7. 增加 Critic：实现完成后派发只读审查任务。
8. 增加 Split：按计划创建多个 assignment 和 worktree。
9. 汇总 diff、测试、日志和审查意见，生成验收卡。
10. 把用户最终接受或打回写入 Matter 和项目偏好。

## 11. 最终形态

最终的一龙项目空间应该像一个“用户拥有节点和 AI 的开发组织”：

- 用户的 PC 是工作区和执行资源。
- 用户的 AI/CLI 是数字开发者。
- 项目频道是协作现场。
- Matter 是可追踪的交付单。
- Worktree 是隔离施工区。
- 审查和验收是质量闸门。
- Context、Taste、Skill 是越用越强的项目资产。

这样，“群体 AI 开发”才会真正贴合一龙项目：用户不只是让一个 AI 回答问题，而是让一组运行在自己电脑和可信节点上的 AI 共同把项目往前推进。
