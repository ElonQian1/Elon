# fb2 业务数据工具路线

fb2 用户真正需要的是“AI 能读懂 fb2 平台数据并帮助分析”，不是简单联网搜索。主项目应该把 fb2 提供的数据变成可检索、可引用、可评测的上下文。

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
