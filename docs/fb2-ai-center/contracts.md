# fb2 AI Center 契约

## 主项目对 fb2 输出

### 1. 外部应用信息

```http
GET /api/external/apps/fb2
```

用途：

- 返回 fb2 中文名、logo、默认群、功能开关。
- fb2 注册过的账号在主项目注册时，应提示使用 fb2 账号登录，并显示 fb2 品牌信息。

### 2. fb2 用户创建主项目会话

```http
POST /api/external/apps/fb2/accounts/session
X-Elon-External-App-Token: <shared-secret>
```

fb2 后端调用。主项目返回：

- 主项目 bearer token
- 主项目用户信息
- fb2 账号绑定信息
- 默认加入的群聊
- 首次试用额度信息

认证 header 以线上实现为准：`X-Elon-External-App-Token: <shared-secret>`，也可使用 `Authorization: Bearer <shared-secret>`。旧文档中的 `X-External-App-Token` 不再作为接入示例。

### 3. 主项目用户授权登录 fb2

```http
POST /api/external/apps/fb2/authorize
Authorization: Bearer <main-project-token>

POST /api/external/apps/fb2/authorize/exchange
X-Elon-External-App-Token: <shared-secret>
```

用途：

- 主项目用户可以授权登录 fb2。
- fb2 只拿到授权交换结果，不读取主项目内部密码或会话实现。

### 4. 聊天和语音启动协议

```http
GET /api/external/apps/fb2/chat-bootstrap
Authorization: Bearer <main-project-token>
```

用途：

- 告诉 fb2 默认群、消息接口、AI 回复接口、ASR/TTS 接口、WebSocket 语音协议和推荐交互。
- fb2 客户端应优先读这个接口，而不是硬编码主项目路径。
- `voice.composer` 会声明完整微信式输入栏要求：`VoiceComposerView`、录音浮层、状态、区域、回调和系统 ASR 到云端 ASR 的兜底配置。
- `aiReply` 会声明 `@EL`、长按 `AI回复`、群聊总结等入口如何触发主项目模型回复，以及 `topic_hint`、Context Pack、回答规则和失败降级策略。
- `billing` 会声明余额接口、试用额度来源和检查点：ASR/TTS/context fetch 不检查 AI 余额，AI 回复生成前才检查。

### 5. 业务上下文契约

```http
GET /api/external/apps/fb2/context-contract
```

用途：

- 给 fb2 代理读取主项目认可的 Context Pack 示例、质量告警、工具契约、观测指标和计费策略。
- 给 fb2 代理读取 `answer_policy_contract`（`fb2.answer_policy.v1`），用于固定 AI 回答边界、引用规则和评测问题。
- `answer_policy_contract.eval_scenarios` 是机器可读评测矩阵，覆盖 `today_matches_analysis`、`my_ticket_analysis`、`platform_order_risk`、`group_opinion_summary`、`selected_message_review`、`source_reference_audit`，声明每个场景的入口、优先上下文、必需来源、引用和禁止输出。
- 给 fb2 代理读取 `context_readiness_contract`，用于自动判断本次 Context Pack 是 `blocked`、`degraded` 还是 `ready`。
- 这个接口不返回密钥，不读取 fb2 业务数据。

主项目实际拉取 fb2 上下文后，会把 `answer_policy_contract.prompt_answer_rules` 投影成 prompt 里的 `<answer_rules>`，也会给归一化结果补 `answer_policy` 并放进 prompt metadata。fb2 不返回 `answer_policy` 时，主项目使用默认策略，但 fb2 的 Context Pack 和工具结果必须能支撑这些回答边界。

运行时回答 prompt 还会注入 `<fb2_domain_scenario_guidance schema="fb2.domain_scenario_prompt.v1">`。该块由主项目根据 `topic_hint`、工具计划和 Context Pack 数据识别六类场景，并从 `domain_context_projection_contract.domain_scenario_matrix` 的同一源头读取 `required_citations` 和 `forbidden_outputs`。fb2 后续新增工具或字段时，应先更新矩阵，再让主项目 prompt/planner 复用同一源头，避免 contract 与模型实际提示漂移。

工具规划和执行审计 metadata 同步包含 `domain_scenario_selection schema=fb2.domain_scenario_selection.v1`。它不是给模型看的正文，而是机器可读计划字段：每个选中场景都带 `permission_scope`、`primary_tools`、`required_citations` 和 `forbidden_outputs`。后续排查“AI 为什么查了这些 fb2 工具 / 为什么不能回答平台订单明细 / 为什么必须引用 order_id 或 message_id”时，应优先看 executed tools 里的 plan metadata，而不是只看最终自然语言回复。

## AI 数据接入格式原则 v1

fb2 给主项目 AI 的事实输入必须先投影成任务相关的 Context Pack，而不是把数据库、原始网页或索引结果直接交给模型：

- `context_pack` 是唯一主正文，格式为 XML-wrapped Markdown，例如 `<fb2_context_pack>...</fb2_context_pack>`。
- JSON 只承载紧凑机器元数据：`context_pack_version`、`generated_at`、`context_audit_id`、`metrics`、`citation_sources`、`tool_contract`、`usage_policy`、`answer_policy`、`preflight_readiness`。
- `citation_sources[]` 是回答引用和 feedback 回写的来源索引；比赛、赔率、订单、群观点、平台摘要都必须尽量提供可引用 ID。
- 禁止把原始 HTML、巨大 JSON、全量数据库记录、全量 embedding、未裁剪订单明细或其它用户私密数据直接放进 prompt。
- MCP/RAG 不是当前完成条件；后续 MCP 只能包装现有 REST Context Pack、tool manifest 和 `POST /tools/execute`，不能绕过 fb2 权限和审计另建事实源。

`GET /api/external/apps/fb2/context-contract` 还会返回 `domain_context_projection_contract`，这是 fb2 域数据版 RCP（Refactor/Task Context Pack）规范，用来把 repo map/符号索引讨论中的原则落到比赛和订单业务：

- `format.wrapper=fb2_context_pack`，正文必须是 XML-wrapped Markdown。
- `required_sections` 固定 `usage_boundary`、`match_facts`、`user_order_slice`、`platform_order_summary`、`group_opinion_slice`、`retrieval_evidence`、`quality_feedback`。
- `source_registry.required_kinds` 固定业务事实来源：`context_audit`、`match`、`odds`、`user_order`、`ticket`、`group_message`、`opinion_memory`、`platform_order_summary`。
- `source_registry.quality_history_kinds` 固定质量历史来源：`feedback`、`opinion_adoption`，默认 `default_chat_fact=false`；只有显式标注 `scope=quality_history` 时，才可以作为“历史质量反馈/观点采纳记录”引用，不能冒充比赛、赔率、订单或群友观点事实。
- `domain_scenario_matrix` 把六类真实用户问题映射到必须召回的 Context Pack 小节、主项目可自动工具、权限请求、source kinds、feedback 路由和可验收信号：今日比赛、我的票、平台订单风险、群观点、长按消息复核、来源审计。
- `retrieval_projection` 要求 fb2 返回召回理由、命中词、新鲜度、权限范围和是否截断，而不只是返回一堆数据。
- `permission_projection` 固定用户订单、平台匿名摘要和群观点的权限头与禁止泄漏项。
- `quality_closure` 固定 feedback、feedback-summary、opinion-adoption-summary、quality-summary 的闭环口径。
- `anti_patterns` 明确禁止 `raw_html_prompt`、`giant_json_prompt`、`full_database_dump`、`raw_embedding_dump`、`uncited_odds`、`uncited_order`、`platform_order_detail_leak` 等输入形态。

主项目 smoke 会检查这些字段，防止后续把 fb2 AI 数据输入退化成无来源的大 JSON 或临时摘要。

`GET /api/external/apps/fb2/context-contract` 还会返回 `tool_result_envelope_contract`，这是主项目执行 fb2 工具后注入 prompt 的标准结果信封：

- `schema=fb2.tool_result_envelope.v1`，`normalized_result_schema=external_app.normalized_tool_result.v1`。
- 必需字段包括 `schema`、`tool_name`、`request_id`、`status`、`success`、`data`、`error`、`generated_at`、`source_ids`、`visibility`、`metrics`、`grounding`、`reason`。
- `grounding.schema=external_app.tool_result_grounding.v1`，状态固定为 `grounded`、`weak`、`unsafe`、`unavailable`。
- 只有 `success=true` 且 `grounding.status=grounded/weak` 的工具结果可以作为事实；`weak` 必须说明证据缺口，`unsafe/unavailable` 只能说明不可用。
- 工具结果 source registry 同样区分 `business_source_kinds` 和 `quality_history_kinds`。回答和 feedback 回写只能引用 AI 回复正文显式提到的 `source_ids`，不得把未提到、失败或权限不匹配的工具结果写成事实来源。

工具发现、质量端点和反馈写回的边界见 `tool-manifest-boundary.md`。简要口径是：`chat_auto_executable_tool_ids` 才代表主项目聊天 AI 可自动调用的工具；`context_quality_summary`、`context_permission_summary` 等质量/权限能力可以是 integration-only 受保护 HTTP 端点，不要求作为聊天自动 tool id；`feedback`、`opinion_adoption` 默认是质量闭环路线，不要求每次 Context Pack 都作为业务事实 source kind 输出。

主项目返回的 tool execution contract 中，`main_project_execution_result.plan.domain_scenario_selection` 固定为上述机器可读场景选择结果。fb2 不需要复制主项目 planner 逻辑，但新增业务工具时要确保 manifest、Context Pack section 和 `domain_scenario_matrix` 能让主项目 planner 选出正确场景。

## fb2 对主项目输出

### 1. Context Pack

```http
GET /api/main-project/context/pack
X-FB2-AI-CENTER-TOKEN: <shared-secret>
X-FB2-AI-CONTEXT-USER-ID: <same fb2-user-id-if-external_user_id-present>
X-FB2-AI-CONTEXT-SCOPE: platform_order_summary  # only when include_platform_orders=true
```

推荐 query：

```text
group_id=official
external_user_id=<fb2-user-id-if-linked>
topic_hint=<user-question-or-summary-topic>
limit=30
discussion_limit=80
order_limit=20
lottery_type=JingCai|BeiDan
include_platform_orders=false
```

权限头规则：

- 当主项目传 `external_user_id` 时，必须同步传同值 `X-FB2-AI-CONTEXT-USER-ID`，否则 fb2 会按 `missing_context_user_id` 或 `context_user_mismatch` 拒绝，避免普通用户读取他人订单。
- 主项目默认不请求平台订单摘要；只有 `ELON_EXTERNAL_APP_FB2_PLATFORM_ORDER_CONTEXT=true` 且请求带 `X-FB2-AI-CONTEXT-SCOPE: platform_order_summary` 时，才允许请求 `include_platform_orders=true`。fb2 服务端仍可用自己的开关拒绝该范围。

主项目群聊 AI 会从最后一次有效 @EL 用户问题中提取 `topic_hint`。例如用户说 `@EL 帮我分析今天比赛和我的票`，主项目会传：

```text
topic_hint=帮我分析今天比赛和我的票
```

fb2 应该用 `topic_hint` 缩小比赛、订单、群观点召回范围；如果用户只是单独发送 `@EL`，主项目会回退使用前一条真实用户问题。

其他入口也会传 `topic_hint`：

- 长按群消息点击 `AI回复`：使用被选中消息正文。

## Live Tool Manifest

主项目 `GET /api/external/apps/fb2/context-contract` 会主动读取 fb2：

```text
GET /api/main-project/context/tool-manifest
```

返回字段 `live_tool_manifest` 只保留脱敏摘要：

- `status`: `ready | degraded | unavailable | not_configured`
- `tool_count`
- `tool_ids`
- `context_pack_version`
- `has_usage_policy`
- `has_tool_selection_policy`
- `main_project_tool_execution_policy.chat_auto_executable_tool_ids`: 主项目群聊 AI 允许自动规划并执行的 fb2 工具。
- `main_project_tool_execution_policy.manifest_only_tool_ids`: fb2 manifest 已暴露、但主项目聊天 AI 仍只当发现信息/回调/直接 Context endpoint 的工具或接口。
- `main_project_tool_execution_policy.main_project_allowed_missing_tool_ids`: 主项目静态 allowlist 里有、但 fb2 实时 manifest 当前没暴露的工具；出现时按契约漂移处理。
- `secret_values_exposed=false`

这个字段用于确认 fb2 侧当前真实工具契约是否可用，并区分“已发现”和“聊天 AI 可自动执行”。读取失败或 allowlist 漂移时不能让 AI 假装工具存在；主项目仍保留静态契约作为降级说明。
- 创建群聊总结帖：优先使用用户填写的 `topic`，并补充 `title`、`instructions`。
- 自动拆分群聊总结帖：使用主项目拆出的议题 topic。
- `/context/pack` 不可用回退 `/context/today-matches` 时，主项目仍会传 `group_id` 和 `topic_hint`。

最低返回字段：

```json
{
  "success": true,
  "data": {
    "context_pack_version": "fb2-chat-pack-v1",
    "generated_at": "2026-06-20T12:00:00+08:00",
    "context_audit_id": "audit-id",
    "context_pack": "<fb2_context_pack>...</fb2_context_pack>",
    "matches": [],
    "user_orders": [],
    "group_messages": [],
    "platform_order_summary": {},
    "citation_sources": [],
    "metrics": {},
    "tool_contract": {},
    "usage_policy": {},
    "answer_policy": {}
  }
}
```

`context_readiness_contract` 的核心检查：

- `context_pack` 非空。
- `generated_at` 存在。
- 比赛、订单、群观点相关结论有 `match_id`、`order_id` 或 `message_id`。
- `metrics.budget_status` 不能是 `empty`。
- 用户问订单剖析时，必须有当前用户可见订单来源：优先使用 Context Pack `user_orders`，也允许使用已按 `external_user_id` + `X-FB2-AI-CONTEXT-USER-ID` 裁剪的 `match_analysis_brief.data.user_orders`；`search_user_orders` 只是补充展开工具。
- 回答规则由主项目 `answer_policy_contract.prompt_answer_rules` 提供，fb2 的数据必须能支撑这些规则。
- `answer_policy` 可由 fb2 返回，也可由主项目默认补齐。

### 2. Context Pack 内容边界

`context_pack` 使用 Markdown 正文，外层使用 XML 风格标签：

```md
<fb2_context_pack version="1.0" project="fb2">

## 使用边界

- 只作为比赛讨论和订单剖析参考。
- 不承诺命中，不诱导投注。
- 必须区分数据事实、群友观点和 AI 推断。

## 今日/近期比赛与赔率

## 当前用户订单/票据

## 群讨论观点

## 平台/店铺订单摘要

## 数据缺口和更新时间

</fb2_context_pack>
```

每条事实尽量带 source id：

- 比赛：`match_id`
- 赔率：`odds_id` 或 `odds_updated_at`
- 订单：`order_id` / `ticket_id`
- 群观点：`message_id`
- 审计：`context_audit_id`
- 回填引用候选：`citation_sources[]`，每项至少包含 `kind`、`id`、`label`，主项目会用 AI 回复中出现的来源 ID/标签回写 `/context/feedback`

## 权限规则

- 当前用户订单只能返回当前登录用户自己的票据。
- 平台订单默认只返回匿名聚合，除非主项目显式传 `include_platform_orders=true` 且 fb2 服务端确认权限。
- 群友观点必须可审计，至少包含 `message_id`、时间和摘要。
- fb2 不把数据库连接、内部表结构或其他用户明细交给主项目。

## 主项目处理规则

- 主项目优先拉 `/context/pack`，失败后回退 `/context/today-matches`。
- 主项目拉 `/context/pack` 时会按 fb2 契约附加用户身份头和平台 scope 头；这些头只用于权限裁剪，不改变数据归属。
- 主项目会做 token budget 裁剪，不把无限大 JSON 塞进 prompt。
- 主项目会在 prompt metadata 增加 `context_fact_summary`，保留比赛、本人订单、群消息数量、少量来源 ID 和简短本人订单样例；这用于防止模型漏看 Context Pack 已有 `user_orders`。
- 主项目会在 `context_fact_summary.preflight_readiness` 中提前投影 fb2 readiness 的 `status` 和少量 `warnings`；如果出现 `fb2_readiness_blocked/degraded/unavailable/not_configured`，AI 必须把它当成数据链路缺口，而不是业务事实。
- 主项目会在 prompt metadata 增加 `context_gap_summary`，把 Context Pack 的状态、readiness、budget、质量告警、业务数据是否可用和 token budget 裁剪字段压缩成模型先读的缺口摘要；当 `fact_answer_allowed=false` 时，AI 必须向用户说明数据缺口，不能编造 fb2 没有提供的比赛、赔率、订单、平台摘要或群友观点。
- 主项目会在 executed tool JSON 前增加 `tool_fact_summary`，把 `match_analysis_brief.data.user_orders` 等当前用户订单样例提前投影，避免大赔率 JSON 被截断时丢失“我的票”结构信息。
- 主项目会在 executed tool JSON 前增加 `tool_gap_summary`，把 `skipped/failed/unavailable` 工具结果提前投影；这些只代表数据缺口，不能编造成比赛、赔率、订单或群友观点事实。
- 主项目会在 AI 回复生成后做来源 ID 校验：回复正文里出现的比赛、赔率、订单、票据、群消息、观点记忆、平台摘要和 audit 类 source id，必须能匹配本轮 `citation_sources`、选中消息额外来源、当前 `context_audit_id`，或 `success=true` 且 `grounding.status=grounded/weak` 的工具结果。未匹配来源会在 `/context/feedback` payload 中标记 `wrong_context=true`，并写入 `note` 的 `source_validation=...`；主项目不新增 fb2 未声明的顶层字段。
- 主项目会生成 `context_quality.warnings`，例如 `missing_context_pack`、`empty_matches`、`missing_tool_contract`。
- 主项目日志只记录 `topic_hint_present`、`fallback_used`、`answer_policy_schema`、`context_quality_warning_count`、`tool_readiness_status` 等观测字段，不记录 shared secret、完整用户票据或题目原文。
- AI 回答必须区分事实、群友观点和推断；上下文不足时要明确说明。
