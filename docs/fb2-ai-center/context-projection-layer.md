# fb2 Context Projection Layer

Schema: `fb2.main_project.context_projection_layer.v1`

This document fixes the long-term shape of the fb2 data layer that feeds the
main-project AI. It applies the repo map / symbol-index discussion to fb2
business data: indexes and search tools live on the fb2 side, while the model
receives a compact, auditable Context Pack.

## Core Rule

The final AI-facing payload is XML-wrapped Markdown with compact JSON metadata.
It is not raw HTML, not a giant JSON dump, not a full database dump, and not a
raw embedding dump.

```text
fb2 live data
  -> fb2 domain indexes
  -> REST Context Pack + tool manifest + tools/execute
  -> main-project prompt Context Pack
  -> AI answer with source references and feedback logging
```

MCP can be added later as a wrapper around the same permission and audit rules.
It must not replace the first-phase REST Context Pack fact source.

## AI-Facing Pack

```md
<fb2_context_pack version="1">

# Identity

Project: fb2
User scope: current user, single group, or platform anonymous aggregate.

## Data Summary

- What this pack can answer
- Which lanes are present
- Which lanes are intentionally absent

## Match Facts

Source kinds: `match`, `odds`, `context_audit`

## User Orders

Source kinds: `user_order`, `ticket`, `context_audit`
Permission: current user only.

## Platform Order Summary

Source kinds: `platform_order_summary`, `context_audit`
Permission: privileged anonymous aggregate only.

## Group Opinions

Source kinds: `group_message`, `opinion_memory`, `context_audit`
Permission: current group only.

## Retrieval Evidence

Every business claim must point to `citation_sources` and a `context_audit_id`.

## Quality Feedback

Only use feedback and opinion adoption as quality history, not as match facts.

</fb2_context_pack>
```

## Domain Lanes

| Lane | User Need | Context Sections | Primary Tools | Permission |
|---|---|---|---|---|
| `match_facts_and_odds` | 今天比赛怎么看 / 赔率怎么变 | `match_facts`, `retrieval_evidence` | `match_analysis_brief`, `search_matches`, `get_match_detail` | `group_context` |
| `current_user_tickets` | 帮我分析我的票 / 我的订单风险 | `user_order_slice`, `match_facts`, `retrieval_evidence` | `match_analysis_brief`, `search_user_orders`, `get_order_detail` | `current_user_only` |
| `platform_order_summary` | 平台今天订单风险怎么样 | `platform_order_summary`, `retrieval_evidence` | `platform_orders` | `privileged_anonymous_summary` |
| `group_opinions` | 群里大家怎么看 / 总结群聊观点 | `group_opinion_slice`, `match_facts`, `retrieval_evidence` | `group_opinion_summary`, `search_group_opinions`, `opinion_memories` | `single_group_context` |
| `opinion_learning_loop` | 采纳用户观点并复盘 | `quality_feedback`, `group_opinion_slice` | `list_opinion_adoptions`, `opinion_adoption_summary`, `opinion_result_reviews` | `single_group_quality_history` |
| `quality_feedback_audit` | 回答有没有引用错来源 | `quality_feedback`, `retrieval_evidence` | `get_context_audit`, `context_audit_summary`, `list_context_feedbacks` | `audit_metadata_only` |

## Domain Indexes

fb2 should maintain or expose these retrieval indexes internally. The main
project consumes only projected evidence and source references.

```text
match_index
odds_snapshot_index
current_user_ticket_index
platform_order_risk_index
group_opinion_index
opinion_memory_index
context_audit_index
feedback_quality_index
```

## User Scenarios

| Scenario | Required Source Kinds | Required Answer Layers |
|---|---|---|
| `today_matches_analysis` | `match`, `odds`, `context_audit` | match facts, odds facts, AI inference, risk boundary |
| `my_ticket_analysis` | `user_order`, `ticket`, `context_audit` | current user orders, match facts, AI inference, risk boundary |
| `platform_order_risk` | `platform_order_summary`, `context_audit` | platform aggregate, AI inference, risk boundary |
| `group_opinion_summary` | `group_message`, `opinion_memory`, `context_audit` | group opinion, match facts, AI inference, risk boundary |
| `selected_message_review` | `group_message`, `match`, `odds`, `context_audit` | reviewed claim, facts, AI inference, risk boundary |
| `group_discussion_summary_post` | `group_message`, `opinion_memory`, `context_audit` | discussion summary, source references, risk boundary |
| `source_reference_audit` | `context_audit`, `feedback` | source registry, data fact boundary, quality feedback |

## Forbidden Outputs

- `fabricated_odds`
- `guaranteed_win`
- `other_user_order_detail`
- `single_user_order_detail`
- `user_identity_leak`
- `fabricated_group_view`
- `group_opinion_as_fact`
- `uncited_source`
- `raw_embedding_dump`
- `full_database_dump`

## Group Chat Evidence

fb2 group/chat verification must use direct API read evidence:

```text
message_id
type
sender_id
created_at
text_len
text_sha256
```

Screenshots and recordings are useful only for UI troubleshooting. They do not
prove that the main-project AI read the group, generated a grounded reply, or
wrote feedback back to fb2.
