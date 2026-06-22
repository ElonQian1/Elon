# fb2 业务数据工具路线

fb2 用户真正需要的是“AI 能读懂 fb2 平台数据并帮助分析”，不是简单联网搜索。主项目应该把 fb2 提供的数据变成可检索、可引用、可评测的上下文。

机器可读口径由 `scripts\fb2-domain-data-blueprint-status.ps1` 和主项目 `GET /api/external/apps/fb2/context-contract` 固定，状态快照字段为 `latest_domain_data_blueprint schema=fb2.main_project.domain_data_blueprint.v1`，接口字段为 `domain_data_blueprint_contract`。它把本文件路线压缩成 6 条数据 lane：比赛赔率、当前用户票据、平台匿名摘要、群观点、观点学习闭环、质量反馈审计。若后续问“fb2 应该给主项目 AI 什么格式，是 Markdown 还是 MCP”，以该字段为准：第一阶段是 XML-wrapped Markdown Context Pack + JSON metadata + tool manifest/tools/execute，MCP 以后只能作为包装层。

## 阶段 1：Context Pack

先做一个稳定的 `GET /api/main-project/context/pack`。

必须覆盖：

- 今日/近期比赛
- 赔率和更新时间
- 当前用户订单/票据
- 群讨论观点
- 平台订单匿名聚合
- 数据来源、更新时间和缺口
- `tool_contract`
- `metrics`

Context Pack 不是普通 Markdown 摘要，而是 fb2 域数据投影。主项目 `/api/external/apps/fb2/context-contract` 的 `domain_context_projection_contract` 会固定这些必需小节：

更具体的可执行模板由同一接口的 `context_pack_template_contract schema=fb2.context_pack_template.v1` 输出。fb2 后端和子会话应优先按这个字段生成 `<fb2_context_pack>`，而不是从文档中复制临时 Markdown；主项目 smoke 和公开契约巡检会检查该字段。

| 小节 | 作用 | 必需来源 |
|---|---|---|
| `usage_boundary` | 告诉 AI 只能做比赛讨论/订单剖析参考，不承诺命中 | 使用边界 |
| `match_facts` | 今日/近期比赛、赔率、更新时间 | `match_id`、`odds_updated_at` |
| `user_order_slice` | 当前用户自己的票据和组合风险 | `order_id`、`ticket_id`、current-user 权限 |
| `platform_order_summary` | 平台/店铺匿名聚合，不泄露个人 | `platform_order_summary` |
| `group_opinion_slice` | 群友观点、分歧、长期观点记忆 | `message_id`、`opinion_memory_id` |
| `retrieval_evidence` | 说明为什么这些数据被召回，以及缺口 | `context_audit_id`、reason、freshness |
| `quality_feedback` | 回答后如何回填来源、采纳观点和错误上下文 | `main_request_id`、`context_audit_id` |

fb2 可以在内部使用数据库索引、缓存、向量库、MCP 或领域召回器，但给主项目 AI 的最终结果必须是这个可引用、可审计、可裁剪的投影，不直接暴露原始索引或 embedding。

主项目会传 `topic_hint`，fb2 应优先根据它召回：

- “今天比赛/今晚比赛”：按日期和开赛时间召回比赛。
- “我的票/我的订单”：召回当前用户自己的订单摘要。
- “这场/某队/某联赛”：召回相关比赛和赔率。
- “群里怎么看”：召回群友观点和分歧。
- “平台订单”：只召回匿名聚合，除非有更高权限。

如果主项目回退调用 `/api/main-project/context/today-matches`，fb2 也应读取 `group_id`、`topic_hint` 和 `lottery_type`，返回更贴近问题的轻量比赛上下文。

这一阶段 AI 只使用一次性上下文，不自动调用 fb2 细分工具。

## 阶段 2：Declared Tools

fb2 在 Context Pack 里声明可用工具，主项目先把它们投影给模型，状态保持 `declared_only`。

推荐工具：

| 工具 | 权限 | 用途 |
|---|---|---|
| `search_matches` | `group_context` | 按日期、联赛、球队、彩种搜索比赛 |
| `get_match_detail` | `group_context` | 查单场比赛、赔率、伤停、更新时间 |
| `search_user_orders` | `current_user_only` | 查当前用户自己的票据/订单摘要 |
| `get_order_detail` | `current_user_only` | 查当前用户可见的单个订单明细 |
| `search_group_opinions` | `group_context` | 按比赛或关键词检索群友观点 |
| `group_opinion_summary` | `single_group_lightweight_memory` | 生成本群轻量观点摘要，优先服务“群里怎么看/大家怎么看” |
| `match_analysis_brief` | `match_focused_brief` | 组合比赛候选、群观点摘要和可选本人订单，优先服务“今天比赛/某场/我的票” |
| `opinion_memories` | `single_group_persistent_opinion_index` | 查单群长期观点记忆，作为群友历史观点证据 |
| `list_opinion_adoptions` | `answer_opinion_adoption_samples` | 查本群主项目 AI 曾采纳的观点样本 |
| `opinion_adoption_summary` | `answer_opinion_adoption_metrics` | 查本群观点采纳次数、来源和意图汇总 |
| `opinion_result_reviews` | `single_group_opinion_result_review_samples` | 查本群观点赛后复盘样本 |
| `opinion_result_review_summary` | `single_group_opinion_result_review_metrics` | 查本群观点赛后复盘质量汇总 |
| `platform_orders` | `privileged_summary` | 查平台/店铺匿名聚合订单摘要，默认禁用 |
| `get_context_audit` | `audit_metadata_only` | 回查某次 Context Pack 来源、预算、耗时 |
| `context_audit_summary` | `audit_metrics_only` | 长期汇总上下文质量指标 |

这一阶段 AI 可以说“当前上下文不足，需要调用 get_match_detail 补充”，但不能假装已经调用。

## 阶段 3：Tool Execution

等 Context Pack 和权限稳定后，主项目再做工具执行层。

当前主项目群聊 AI 已自动规划并执行：

- `match_analysis_brief`：比赛、赔率、预测、今日场次和“我的票”问题的首选聚合工具。
- `group_opinion_summary`：群友观点、大家怎么看、讨论分歧问题的首选聚合工具。
- `search_matches` / `search_user_orders` / `search_group_opinions` / `opinion_memories` 等细分工具仍作为可追溯展开或补充来源。

按需工具选择规则：

| 用户意图 | 首选路径 | 权限和回答要求 |
|---|---|---|
| 默认业务问答 | 先用 `/context/pack` | Context Pack 已有事实优先，缺口再查工具。 |
| 今日比赛、赔率、预测、某场分析 | `match_analysis_brief` | 必须区分比赛事实、赔率事实和 AI 推断，不承诺命中。 |
| 我的票、我的订单、帮我分析我的票 | `match_analysis_brief`，必要时补 `search_user_orders` | 必须有 `external_user_id` 和同值 `X-FB2-AI-CONTEXT-USER-ID`；只能使用当前用户订单。 |
| 群里大家怎么看、观点分歧 | `group_opinion_summary` | 只标为群友观点，不得当成比赛事实。 |
| 平台今天订单风险、平台汇总 | `platform_orders` | 必须显式 platform scope，只返回匿名聚合，不泄露单个用户。 |
| manifest-only 工具 | 不自动执行 | 只作为能力发现、回调端点或后续接入候选。 |

执行前置条件：

- 工具 schema 稳定。
- 工具返回有 `source_id`。
- 用户订单工具完成权限裁剪。
- 日志记录工具名、参数摘要、耗时、结果条数和失败原因。
- AI 输出中能引用工具结果来源。

## 阶段 4：领域索引和观点记忆

fb2 侧维护领域索引，减少每次全表扫描：

- `match_index`：比赛、联赛、球队、开赛时间。
- `odds_snapshot_index`：赔率快照和更新时间。
- `order_risk_index`：用户票据结构、组合风险、命中/亏损复盘。
- `group_opinion_index`：群友观点、消息 ID、支持/反对理由、比赛关联。
- `context_audit_index`：每次上下文包的来源数量、字符量、耗时和裁剪状态。

AI 逐步从“读取当前上下文”升级到“基于历史观点和复盘持续改进分析”。

## 数据质量评分

fb2 每次返回 Context Pack 时建议输出：

```json
{
  "metrics": {
    "context_pack_latency_ms": 42,
    "context_pack_chars": 12000,
    "budget_status": "ok",
    "retrieved_source_count": 18,
    "source_counts": [
      {"source_type": "match", "count": 8},
      {"source_type": "user_order", "count": 3},
      {"source_type": "group_message", "count": 7}
    ],
    "stale_source_count": 0,
    "permission_denied_count": 0,
    "fallback_used": false
  }
}
```

主项目长期根据这些指标评估：

- AI 是否拿到了有效比赛数据。
- 是否经常缺用户订单。
- 是否经常上下文过大被裁剪。
- 群友观点是否能被引用。
- 回答是否能带来源 ID。
