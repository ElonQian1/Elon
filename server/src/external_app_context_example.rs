//! Public example contract for external app context packs.

use serde_json::{json, Value};

use crate::external_app_usage_policy::default_usage_policy;

pub(crate) fn public_context_pack_example(app_id: &str) -> Option<Value> {
    match app_id {
        "fb2" => Some(json!({
            "endpoint": "GET /api/main-project/context/pack",
            "auth_header": "X-FB2-AI-CENTER-TOKEN",
            "query_example": {
                "group_id": "official",
                "external_user_id": "fb2-user-id",
                "topic_hint": "总结预测今天的比赛",
                "limit": 30,
                "discussion_limit": 80,
                "order_limit": 20,
                "include_platform_orders": false
            },
            "response_shape": {
                "success": true,
                "data": fb2_context_pack_example_data()
            },
            "minimum_required_fields": [
                "context_pack_version",
                "generated_at",
                "context_pack",
                "matches",
                "user_orders",
                "group_messages",
                "tool_contract"
            ]
        })),
        _ => None,
    }
}

fn fb2_context_pack_example_data() -> Value {
    json!({
        "context_pack_version": "fb2-chat-pack-v1",
        "generated_at": "2026-06-20T12:00:00+08:00",
        "context_audit_id": "2f6d1d5a-0000-0000-0000-000000000000",
        "context_pack": "<fb2_context_pack version=\"1.0\" project=\"fb2\">\n\n## 使用边界\n\n- 只作为比赛讨论和订单剖析参考。\n- 不承诺命中，不诱导投注。\n- 必须区分数据事实、群友观点和 AI 推断。\n\n## 今日/近期比赛与赔率\n\n- match_id=m-001 联赛示例 主队 vs 客队，赔率更新时间 2026-06-20T11:58:00+08:00。\n\n## 当前用户订单/票据\n\n- order_id=o-001 仅当前用户可见，用于风险拆解。\n\n## 群讨论观点\n\n- message_id=msg-001 群友观点示例。\n\n## 平台/店铺订单摘要\n\n- 仅匿名聚合，不暴露其他用户订单明细。\n\n</fb2_context_pack>",
        "matches": [
            {
                "id": "m-001",
                "lottery_type": "JingCai",
                "league": "示例联赛",
                "home_team": "主队",
                "away_team": "客队",
                "match_time": "2026-06-20T19:30:00+08:00",
                "status": "scheduled",
                "odds": {
                    "updated_at": "2026-06-20T11:58:00+08:00",
                    "win": 1.95,
                    "draw": 3.20,
                    "lose": 3.60
                }
            }
        ],
        "user_orders": [
            {
                "order_id": "o-001",
                "ticket_id": "t-001",
                "match_ids": ["m-001"],
                "visibility": "current_user_only",
                "summary": "当前用户票据摘要"
            }
        ],
        "group_messages": [
            {
                "message_id": "msg-001",
                "match_id": "m-001",
                "author_role": "group_member",
                "opinion": "群友观点摘要",
                "created_at": "2026-06-20T11:50:00+08:00"
            }
        ],
        "platform_order_summary": {
            "visibility": "anonymous_aggregate",
            "note": "普通群聊只返回匿名聚合数据"
        },
        "metrics": {
            "retrieved_source_count": 3,
            "context_pack_chars": 2048,
            "context_pack_latency_ms": 42,
            "budget_status": "ok",
            "budget_recommendation": "可以直接使用当前 Context Pack。",
            "source_counts": [
                {"source_type": "match", "count": 1},
                {"source_type": "user_order", "count": 1},
                {"source_type": "group_message", "count": 1}
            ]
        },
        "tool_contract": {
            "schema": "fb2.tools.v1",
            "tools": [
                {
                    "name": "get_match_detail",
                    "description": "按 match id 查询比赛、赔率、伤停和更新时间",
                    "input_schema": {
                        "type": "object",
                        "required": ["match_id"],
                        "properties": {
                            "match_id": {"type": "string"}
                        }
                    },
                    "permission": "group_context",
                    "when_to_use": "用户追问某一场比赛细节或 context_pack 被截断时"
                }
            ]
        },
        "usage_policy": default_usage_policy()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exposes_public_context_pack_example() {
        let example = public_context_pack_example("fb2").unwrap();
        assert_eq!(example["auth_header"], "X-FB2-AI-CENTER-TOKEN");
        assert!(example["minimum_required_fields"]
            .as_array()
            .unwrap()
            .contains(&json!("context_pack")));
        assert_eq!(
            example["response_shape"]["data"]["usage_policy"]["ai_reply_billable"],
            true
        );
        assert_eq!(
            example["response_shape"]["data"]["metrics"]["budget_status"],
            "ok"
        );
        assert!(public_context_pack_example("unknown").is_none());
    }
}
