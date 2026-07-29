---
title: "S4.0 共享节点任务经济回执与影子账本实施计划"
owner: project
reviewed_at: 2026-07-30
review_interval_days: 30
role: implementation_plan
lifecycle: draft
authority: proposal
default_retrieval: true
source_refs:
  - "docs/drafts/open-commerce-network-sui-agent-economy.md"
  - "codex-thread-current://2026-07-30-s4-shadow-settlement"
implementation_refs:
  - "feature:cap-project-space"
  - "feature:cap-multi-agent"
  - "feature:cap-windows-node"
  - "feature:cap-provider-routing"
  - "file:server/src/group_ai/types.rs"
  - "file:server/src/group_ai/executor.rs"
  - "file:server/src/group_ai/review_gate.rs"
  - "file:server/src/group_ai/actions/assignment_actions.rs"
  - "file:server/src/store/node_compute_runs.rs"
  - "file:server/src/store/node_ledger.rs"
  - "file:server/src/ai_cli/pc_billing.rs"
  - "file:server/src/node_api/usage.rs"
---

# S4.0 共享节点任务经济回执与影子账本实施计划

## 文档状态

本文是 `docs/drafts/open-commerce-network-sui-agent-economy.md` 中 S4.0 的首个可执行计划，目标是验证“一项多人、多 AI、共享节点任务能否形成可追溯、不可重复、可核对的经济回执”。

本文只授权后续 AI 代理设计和实现链外影子账本，不授权：

- 发行代币、部署 Move 合约或连接 Sui 主网；
- 接入真实稳定币、钱包、交易所或二级市场；
- 修改现有人民币余额、节点余额、实际扣款和提现逻辑；
- 承诺投资收益、合同分红、股权或代币升值；
- 把 API Key、Codex 凭据、私钥、客户原始数据或代码上传链上；
- 以本文证明区块链、开放商业网络阶段二至四已经实现。

现有生产计费继续以 `token_usage`、`billing_events`、`node_transactions` 和 `node_compute_runs` 等真实记录为准。S4.0 新账本只能读取和映射这些事实，不能再次扣款或再次增加提供者余额。

## 一句话目标

> 把现有的“谁发起 Matter、谁的节点执行、消耗了多少、谁审查、是否验收”连接成一张任务级经济回执，为以后测试网托管、自动分账、信誉和争议处理建立可验证基础。

## 为什么先做这一项

项目现有能力已经覆盖试点所需的大部分事实：

1. 项目成员可以创建 Matter，并生成多个 Assignment。
2. Assignment 可以派发到授权 PC 节点的真实 Git 工作区执行。
3. `compute_call_id` 可以把 Assignment 结果与 `node_compute_runs` 关联。
4. 共享节点用量可以形成真实消费者扣费和提供者收益流水。
5. 非 solo Matter 需要结构化 Review，Matter 还需要人工验收。

当前缺少的不是新的计算网络，而是任务级的统一经济语义：

- 一个 Matter 实际使用了哪些计算记录；
- 现有算力成本和未来结果奖励如何区分；
- 任务通过、失败、取消、重试或争议时如何处理；
- 同一完成事件重复到达时如何保证不重复分配；
- 消费者、提供者、审查者和平台能否看到同一张回执；
- 未来链上对象如何映射，而不侵入当前生产系统。

因此，S4.0 应先完成链外状态机、双重记账不变量和对账，再决定是否进入 Sui 测试网。

## 现有实现事实

下表是实施前必须复核的当前事实。它们是复用入口，不代表要在原文件内继续堆积经济逻辑。

| 现有能力 | 当前实现 | S4.0 的使用方式 |
|---|---|---|
| Matter 状态 | `server/src/group_ai/types.rs` | 读取 `plan_ready`、`running`、`review_ready`、`done`、`canceled`、`failed` |
| Assignment 执行 | `server/src/group_ai/executor.rs` | 读取节点、提供者、CLI、产物和 `compute_call_id` |
| Assignment 结算证据 | `server/src/group_ai/actions/assignment_actions.rs` | 复用执行证据，不把事件字段当作独立资金真源 |
| 审查与验收门禁 | `server/src/group_ai/review_gate.rs`、`server/src/group_ai/actions.rs` | `matter_accepted` 是任务结果奖励可以记账的唯一默认触发点 |
| 节点执行证明 | `server/src/store/node_compute_runs.rs` | 读取时长、Token、真实成本、提供者收益和结算状态 |
| 节点收益流水 | `server/src/store/node_ledger.rs` | 作为已发生算力分账的生产真源，只读映射 |
| PC CLI 资源归属 | `server/src/ai_cli/pc_billing.rs` | 区分自有 Codex、共享 Codex 和平台资源 |
| 用户节点账本查询 | `server/src/node_api/usage.rs` | 为后续统一的任务经济视图提供现有查询基础 |

必须保留两个不同概念：

### 算力补偿

算力补偿按实际模型调用和计费事件产生。当前系统会在执行完成后写入节点流水，它补偿资源消耗，不等待 Matter 最终验收。

### 任务结果奖励

任务结果奖励补偿可验收的软件产物、审查或其他贡献。S4.0 只模拟该奖励；默认仅在 Matter 通过验收后记入影子账本，不影响真实余额。

算力补偿和任务结果奖励不能混为一笔。任务失败时，已经真实发生的算力用量仍应保留；失败任务默认不产生额外结果奖励。

## 首期试点范围

首期只选择满足以下条件的 Matter：

- 属于一个真实项目空间；
- 至少有一个非发起人拥有的授权节点参与；
- Assignment 由现有群体 AI 自动派发；
- 执行产生 `compute_call_id` 和可读取的 `node_compute_runs`；
- 有明确验收标准、产物和人工验收结果；
- 不涉及支付、删除生产数据、部署生产环境等高风险动作；
- 通过项目级功能开关明确加入试点。

推荐首个试点任务：

> 由项目成员创建一个低风险文档或小型代码改动 Matter，派发到另一位提供者的 PC 节点，完成结构化审查并由项目决策者验收；系统生成一张包含算力事实、结果奖励模拟分配和对账状态的任务经济回执。

## 参与角色

| 角色 | 当前来源 | S4.0 权利与责任 |
|---|---|---|
| 任务发起人 | `matter.requester_user_id` | 提交目标和验收标准，不得自行伪造执行事实 |
| 项目付款方 | 首期默认项目所有者或明确指定成员 | 只承担影子预算，不发生新增真实扣款 |
| 执行提供者 | `assignment.provider_user_id` | 提供节点或 AI 资源，获得算力事实和模拟结果奖励 |
| Assignment 执行者 | `assignment.assignee_user_id` 或 Bot | 完成具体工作，可与节点提供者不同 |
| 审查者 | `ProjectAiReview` 中的用户或 Bot | 给出结构化审查结论，不能替代项目最终验收 |
| 项目决策者 | 现有 Matter 决策权限 | 通过现有门禁后验收或打回 |
| 平台 | 一龙服务端 | 记录、核对和展示，不拥有用户原始业务数据 |

首期不得默认“发起人、执行者、审查者、验收者”由同一人同时承担。solo Matter 可以用于技术测试，但不能作为多方经济闭环成立的证明。

## 领域对象

建议新增独立的 `task_settlement` 领域，不扩充 `node_ledger.rs` 的职责。

### `TaskSettlementIntent`

表示一项 Assignment 进入经济核对流程。

建议字段：

| 字段 | 说明 |
|---|---|
| `id` | 内部唯一标识 |
| `project_id` | 所属项目 |
| `matter_id` | 所属 Matter |
| `assignment_id` | 所属 Assignment |
| `payer_user_id` | 影子付款方 |
| `provider_user_id` | 节点或资源提供者 |
| `assignee_user_id` | 实际执行者，可为空 |
| `reviewer_user_ids_json` | 参与审查的用户集合 |
| `policy_version` | 计价和分配规则版本 |
| `status` | 当前结算状态 |
| `idempotency_key` | 不可重复创建的业务键 |
| `created_at`、`updated_at` | 审计时间 |

首期幂等键建议为：

```text
task-settlement:v1:{project_id}:{matter_id}:{assignment_id}
```

### `TaskSettlementSource`

保存回执所依据的事实引用，不复制大段原始数据。

建议字段：

- `settlement_intent_id`
- `compute_call_id`
- `node_compute_run_id`
- `node_transaction_id`
- `token_usage_event_id`
- `billing_event_id`
- `source_kind`
- `source_digest`

一个 Assignment 可以因重试关联多个 compute run。所有 run 必须保留，不能只取最后一次成功记录而隐藏失败成本。

### `TaskSettlementQuote`

在事实完整后冻结一份影子计价快照：

- `currency`：首期固定为 `RMB_FEN_SHADOW`；
- `compute_cost_fen`：从现有真实执行记录映射；
- `compute_provider_earned_fen`：从现有节点流水映射；
- `outcome_reward_fen`：首期由项目预算策略计算，默认可为 0；
- `review_reward_fen`：首期默认 0，但保留字段；
- `platform_service_fee_fen`：首期仅模拟；
- `reserve_fen`：争议或退款储备，首期仅模拟；
- `policy_snapshot_json`：保存当时规则，后续改规则不追溯修改旧回执；
- `source_digest`：对来源事实和策略快照计算摘要。

所有金额使用整数“分”，禁止用浮点数参与账本运算。

### `TaskSettlementReceipt`

表示一次任务经济判断已经形成不可变结果：

- `posted`：Matter 已验收，结果奖励影子分配生效；
- `voided`：Matter 取消或最终失败，不产生结果奖励；
- `adjusted`：已发布回执发生纠错，使用新调整记录冲正；
- `disputed`：回执被质疑，等待人工处理；
- `reconciled`：回执与全部来源事实核对一致。

已发布回执不得原地改金额。任何纠错必须创建 `TaskSettlementAdjustment` 和反向/补充账本分录。

### `TaskLedgerTransaction` 与 `TaskLedgerEntry`

每次发布、作废或调整形成一笔交易和多条分录。

建议账户：

- `project_budget:{project_id}`
- `consumer_expense:{payer_user_id}`
- `provider_receivable:{provider_user_id}`
- `assignee_receivable:{assignee_user_id}`
- `reviewer_receivable:{reviewer_user_id}`
- `platform_service_revenue`
- `dispute_reserve`

每笔交易必须满足：

```text
借方合计 = 贷方合计
```

首期账本是影子账本，`receivable` 只表示模拟权利，不代表平台欠款或可提现余额。

## 状态机

### Matter 和 Assignment 映射

现有状态不修改：

```text
Matter:
plan_ready -> running -> review_ready -> done
                    |          |
                    v          v
                  failed     plan_ready
                    |
                    v
                 retry

Assignment:
planned -> running -> completed/settled/settled_no_provider
              |
              v
            failed -> planned
```

### 影子结算状态

```text
draft
  -> collecting_sources
  -> priced
  -> awaiting_acceptance
  -> posted
  -> reconciled
```

失败分支：

```text
draft/collecting_sources/priced/awaiting_acceptance
  -> voided

posted/reconciled
  -> disputed
  -> adjusted
  -> reconciled
```

状态规则：

1. Assignment 创建后可以建立 `draft`，但不能产生分录。
2. 执行完成且可找到 `compute_call_id` 后进入 `collecting_sources`。
3. 来源事实和策略版本冻结后进入 `priced`。
4. 全部非审查 Assignment 完成后进入 `awaiting_acceptance`。
5. 只有现有 `accept_matter` 成功并写入 `matter_accepted` 后才可 `posted`。
6. Matter 在发布前进入 `canceled` 或最终 `failed` 时转为 `voided`。
7. 已发布回执不能因 Matter 字段被直接修改而消失。
8. 同一幂等键和同一发布版本最多产生一笔 `posted` 交易。

## 默认计价与分配规则

S4.0 不创造新的真实价格体系，默认规则如下：

1. `compute_cost_fen` 直接读取现有 `node_compute_runs.billed_cost_rmb_fen`。
2. `compute_provider_earned_fen` 直接读取对应 `node_transactions.provider_earned_fen`。
3. 同一 `compute_call_id` 重复事件只能关联同一来源，不得重复累加。
4. 自有节点、自有 Codex 或未实际扣费的记录可以是 0，但仍保留执行证明。
5. 任务失败或取消不删除算力事实，也不生成额外结果奖励。
6. `outcome_reward_fen` 首期默认 0；只有项目显式配置影子预算后才模拟。
7. `review_reward_fen` 首期默认 0，先验证审查身份和独立性。
8. 平台费率和提供者分账比例读取当前版本化配置，不在代码中复制固定比例。
9. 任何报价必须保存完整策略快照，不能使用“当前配置”重新计算历史回执。

后续若启用非零结果奖励，首期建议使用固定任务奖励，不使用代币价格、市场竞价或复杂评分：

```text
任务影子总额
  = 执行者结果奖励
  + 审查奖励
  + 平台技术服务费
  + 争议储备
```

算力补偿是对现有真实记录的映射，不再次加入上述可支付总额。

## 事件与触发点

建议新增领域事件：

- `task_settlement_intent_created`
- `task_settlement_source_attached`
- `task_settlement_priced`
- `task_settlement_awaiting_acceptance`
- `task_settlement_posted`
- `task_settlement_voided`
- `task_settlement_disputed`
- `task_settlement_adjusted`
- `task_settlement_reconciled`
- `task_settlement_reconciliation_failed`

建议触发关系：

| 现有事件或动作 | 新领域动作 |
|---|---|
| Assignment 创建 | 幂等创建 Intent |
| `assignment_execution_completed` | 关联 compute run 和节点流水 |
| `assignment_execution_failed` | 保留失败用量，等待重试或最终作废 |
| `matter_review_ready` | 冻结 Quote，进入待验收 |
| `matter_accepted` | 发布结果奖励影子分录和 Receipt |
| `matter_canceled` | 未发布 Intent 作废 |
| 人工争议操作 | 标记 disputed，不修改原分录 |

触发逻辑应通过独立服务调用或可靠事件投影完成。不得把数据库写入散落到 `executor.rs`、`actions.rs` 和 `node_router.rs` 多个位置。

## 建议代码边界

模块名可在实施审计后调整，但责任必须保持独立。

```text
server/src/task_settlement/
  mod.rs
  model.rs          # 对象、状态和错误
  policy.rs         # 版本化计价与分配规则
  service.rs        # 用例编排和权限无关领域逻辑
  projection.rs     # 从 Matter/compute/ledger 投影事实
  ledger.rs         # 双重记账和不变量
  reconcile.rs      # 来源核对和异常报告
  api.rs            # 只读查询与受控争议入口

server/src/store/
  task_settlements.rs
  task_settlement_tests.rs
```

边界要求：

- `group_ai` 只发送领域事实或调用一个窄接口，不负责分账公式；
- `node_ledger` 继续负责现有节点收益，不负责 Matter 验收；
- `task_settlement` 不直接调用节点进程、CLI 或 Git；
- 未来 `sui_adapter` 只消费已发布回执，不决定业务金额；
- PC/APK 只展示服务端回执，不在客户端计算金额；
- 每个文件遵守项目模块化约束，不创建巨型经济管理文件。

## 存储和迁移

建议首期使用独立追加式表：

- `task_settlement_intents`
- `task_settlement_sources`
- `task_settlement_quotes`
- `task_settlement_receipts`
- `task_ledger_transactions`
- `task_ledger_entries`
- `task_settlement_adjustments`
- `task_settlement_reconciliation_issues`

约束要求：

- `idempotency_key` 唯一；
- `compute_call_id + settlement_intent_id` 唯一；
- 已发布交易使用唯一 `posting_key`；
- 金额必须大于等于 0，方向由借贷字段表达；
- Receipt 引用的 Quote 和策略版本不可为空；
- 调整必须引用原 Receipt；
- 删除 Matter、用户或节点时不得级联删除历史回执；
- 不回填或改写 `node_transactions` 和 `node_compute_runs`；
- 历史 Matter 回填作为独立任务，首期默认只处理功能开启后的新 Matter。

## 候选查询接口

接口名称在编码前需要按现有路由风格确认，首期建议只读：

```text
GET /api/projects/{project_id}/economy/settlements
GET /api/projects/{project_id}/economy/settlements/{settlement_id}
GET /api/projects/{project_id}/economy/reconciliation
GET /api/me/economy/contributions
```

返回内容至少包含：

- Matter 和 Assignment 摘要；
- 参与角色；
- 来源 compute run 和节点交易引用；
- 算力事实；
- 结果奖励影子分配；
- 状态、策略版本和摘要；
- 是否对账一致；
- 明确的 `shadow_only: true` 标记。

接口不得返回节点密钥、API Token、Codex 凭据、完整 Prompt、原始客户数据和未授权项目内容。

## 产品展示

首期不建立“钱包”页面，建议在项目 Matter 详情增加“经济回执”只读区域：

- 任务总状态；
- 真实算力用量与成本事实；
- 当前节点提供者的既有收益事实；
- 模拟结果奖励和分配；
- 审查、验收和对账状态；
- “影子记录，不代表可提现资产”的固定提示。

提供者个人页可以聚合“我参与的任务”，但不能把影子应收与现有可提现节点余额相加。

## 权限与安全

1. 项目成员只能读取自己有权访问的项目回执。
2. 节点提供者只能读取与自己相关的贡献明细，不能读取项目其他敏感数据。
3. 普通执行者不能验收自己的任务并触发结果奖励。
4. 非 solo 经济试点至少需要一个独立 passed Review。
5. 争议、调整和策略修改必须记录操作者、原因和前后摘要。
6. 所有写操作使用服务端身份和现有项目权限，不接受客户端自报金额。
7. 功能开关默认关闭，并支持按项目白名单开启。
8. 任何链 SDK、钱包依赖和私钥配置不得在 S4.0 引入。

建议功能开关：

```text
ELON_TASK_SHADOW_SETTLEMENT_ENABLED=false
```

同时应有项目级启用标记，避免一个全局开关让全部历史任务自动进入试点。

## 对账规则

每张回执至少核对：

1. Intent 所属项目、Matter、Assignment 是否一致；
2. Assignment 节点是否与 compute run 节点一致；
3. `compute_call_id` 是否唯一且属于该执行；
4. 消费者、提供者和资源所有者是否与冻结上下文一致；
5. Token 和成本是否与 `node_compute_runs` 一致；
6. 提供者收益是否与 `node_transactions` 一致；
7. Matter 是否通过现有 Review Gate 并最终 `done`；
8. Quote 的来源摘要和策略摘要是否仍可重算；
9. Ledger 借贷合计是否相等；
10. 是否存在重复发布、来源缺失或发布后被原地修改。

对账失败只生成问题记录和告警，不自动改现有生产余额。

## 可观测性

至少记录以下指标：

- 创建的 Intent 数；
- 找不到 compute run 的 Assignment 数；
- 找不到节点流水的已计费 run 数；
- 待验收、已发布、已作废和争议数量；
- 幂等去重次数；
- 对账失败数量及原因；
- 从 Assignment 完成到回执可读取的延迟；
- 影子总成本、影子结果奖励和既有真实节点收益，三者分开展示。

日志中使用内部 ID，不写 Prompt、凭据、客户数据和合同原文。

## 验收标准

首期必须通过以下用例：

1. 一个共享节点 Assignment 完成后只创建一个 Intent。
2. 重复收到相同完成事件不会产生重复来源或重复分录。
3. Intent 能通过 `compute_call_id` 关联正确的 compute run。
4. 已计费的共享节点执行能关联正确节点交易和提供者收益。
5. 自有节点或未计费执行仍能生成 0 成本事实回执。
6. Matter 未验收时不得发布结果奖励分录。
7. Matter 通过现有 Review Gate 并验收后，只发布一次影子交易。
8. Matter 取消或最终失败时不产生结果奖励，失败算力事实仍保留。
9. 重试产生多个 compute run 时全部可追溯，不重复计算同一 run。
10. 已发布回执只能通过调整记录纠错，不能原地改金额。
11. 每笔账本交易借贷相等。
12. 关闭功能开关后，现有 Matter、节点计费和节点余额行为完全不变。
13. 项目外用户无法读取回执，节点提供者看不到无关项目数据。
14. 用户视图、提供者视图和管理员对账视图引用同一 Receipt。
15. API 明确返回 `shadow_only: true`，UI 不把影子金额显示为可提现资产。

## AI 代理任务拆分

后续代理应按顺序执行，不并行修改同一模块。

### T0：实现审计与决策记录

- 复核 Matter、Assignment、Review、compute run 和节点流水的真实状态转换；
- 输出事件触发图和数据库唯一约束；
- 确认项目级功能开关存放位置；
- 不改业务代码。

完成条件：本文中的实现引用与当前主分支一致，所有待定项有明确选择。

### T1：纯领域模型与不变量

- 新建 `task_settlement/model.rs`、`policy.rs` 和 `ledger.rs`；
- 实现状态转换、整数金额、幂等键和借贷平衡测试；
- 不访问数据库、网络或 Sui。

完成条件：状态机非法转换、重复发布、金额溢出和不平衡交易测试全部通过。

### T2：追加式存储

- 新增迁移和 `store/task_settlements.rs`；
- 实现 Intent、来源、Quote、Receipt、Entry、Adjustment 和 Issue 的原子写入；
- 添加并发幂等和重启恢复测试。

完成条件：重复请求只返回同一结果，已发布数据不可被普通更新覆盖。

### T3：事实投影与对账

- 从现有 Matter、Assignment、compute run 和节点交易建立只读投影；
- 支持重试、多 run、0 成本和缺失来源；
- 生成对账问题，不改变现有账本。

完成条件：节点现有测试继续通过，新增对账矩阵通过。

### T4：Matter 生命周期集成

- 通过窄服务边界响应 Assignment 完成、Matter 验收和取消；
- 接入功能开关和项目白名单；
- 验证重复事件、失败恢复和进程重启。

完成条件：关闭开关零行为变化；开启后完整产生影子回执。

### T5：查询 API 与只读 UI

- 增加项目回执、个人贡献和对账查询；
- 在现有 Matter 详情或节点视图中增加只读展示；
- 所有页面明确区分真实余额和影子金额。

完成条件：消费者、提供者和项目决策者能核对同一任务，不泄露无关项目内容。

### T6：Sui 映射规范

- 仅把稳定的链外对象映射到候选 `MatterObject`、`ResultReceipt`、Escrow 和事件；
- 输出 Move 属性测试和测试网失败场景；
- 不部署主网，不接真实资产。

进入条件：至少完成一轮真实多方 Matter 试点，且影子账本连续对账无重复和金额差异。

## 首期默认决策

为避免后续代理自行猜测，S4.0 默认采用以下选择：

| 问题 | 首期默认 |
|---|---|
| 谁是影子付款方 | 项目所有者或现有权限系统明确指定的项目成员 |
| 何时发布结果奖励 | Matter 通过 Review Gate 并执行 `accept_matter` 后 |
| 失败任务是否有结果奖励 | 否 |
| 失败任务算力是否保留 | 是，作为真实资源事实 |
| 审查奖励 | 字段保留，金额默认 0 |
| 结果奖励 | 金额默认 0，项目显式配置后才模拟 |
| 节点真实收益是否重算 | 否，只读取现有节点流水 |
| 历史 Matter 是否回填 | 否，另立任务 |
| 是否接 Sui | 否 |
| 是否允许提现 | 否 |

## 下一轮需要讨论的产品问题

完成 T0 前，应由产品负责人逐项确认：

1. 非零结果奖励来自项目固定预算、任务报价还是成交收入；
2. 执行者与节点提供者不是同一人时，结果奖励归谁；
3. 审查者何时获得奖励，以及如何避免互相刷审查；
4. 多次重试的算力成本由谁承担，是否设置任务总预算上限；
5. 项目成员能否在派发前看到影子报价并确认；
6. 争议由项目所有者、独立审查者还是平台仲裁；
7. 试点成功的业务指标是节省成本、提高交付率，还是形成真实跨用户交易。

在这些问题未确认前，代理可以实现 0 金额影子闭环和技术不变量，不得自行开启真实结果奖励。

## S4.0 完成定义

只有同时满足以下条件，S4.0 才能标记完成：

- 至少一个真实项目完成跨用户、跨节点的低风险 Matter；
- 执行、用量、节点流水、审查和验收全部进入同一回执；
- 重试、失败、取消、重复事件和调整都有通过的自动化测试；
- 连续试点期间不存在重复发布和账本不平衡；
- 影子账本与生产计费对账一致，且未改变任何真实余额；
- 用户能够理解“资源成本、节点收益、结果奖励”三者区别；
- 项目负责人书面批准是否进入 S4.1 测试网适配器研究。

S4.0 的价值不是“已经上链”，而是证明项目现有多人 AI 开发、共享节点、审查和经济记录可以组成一个可信任务闭环。只有这个闭环在链外稳定，Sui 才可能成为所有权和结算的放大器，而不是替代尚未成立的业务。
