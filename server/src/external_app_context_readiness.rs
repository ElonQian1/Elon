//! Readiness guidance for external app context packs.

use serde_json::{json, Value};

pub(crate) fn public_context_readiness_guidance(app_id: &str) -> Option<Value> {
    match app_id {
        "fb2" => Some(json!({
            "app_id": "fb2",
            "schema": "fb2.context_readiness.v1",
            "purpose": "给 fb2 代理和主项目代理做自动接入自检，判断业务上下文是否足够支撑 AI 回答。",
            "required_response_fields": [
                "context_pack_version",
                "generated_at",
                "context_pack",
                "matches",
                "user_orders",
                "group_messages",
                "tool_contract",
                "metrics"
            ],
            "recommended_response_fields": [
                "context_audit_id",
                "platform_order_summary",
                "usage_policy"
            ],
            "main_project_prompt_metadata": [
                "usage_policy",
                "answer_rules",
                "context_quality",
                "context_budget",
                "external_metrics",
                "context_audit_id",
                "tool_contract",
                "executed_external_app_tools"
            ],
            "readiness_levels": [
                {
                    "status": "blocked",
                    "conditions": [
                        "context_pack missing or empty",
                        "generated_at missing and user asks time-sensitive question",
                        "metrics.budget_status=empty",
                        "requested user order analysis but no current_user_only order source"
                    ],
                    "ai_behavior": "必须说明数据不足，不能预测比赛或剖析订单。"
                },
                {
                    "status": "degraded",
                    "conditions": [
                        "tool_contract missing or partial",
                        "matches empty but question does not depend on concrete matches",
                        "context_pack too large and was trimmed",
                        "stale_source_count > 0"
                    ],
                    "ai_behavior": "可以回答，但必须提示缺口、新鲜度或裁剪风险。"
                },
                {
                    "status": "ready",
                    "conditions": [
                        "context_pack present",
                        "generated_at present",
                        "source ids present for relevant claims",
                        "tool_contract declares recommended tools or enough sources are already present"
                    ],
                    "ai_behavior": "可以基于 fb2 上下文回答，并引用 match_id/order_id/message_id。"
                }
            ],
            "automated_checks": [
                {
                    "name": "has_context_pack",
                    "field": "context_pack",
                    "pass_when": "non_empty_string",
                    "failure_warning": "missing_context_pack"
                },
                {
                    "name": "has_generated_at",
                    "field": "generated_at",
                    "pass_when": "non_empty_iso8601_string",
                    "failure_warning": "missing_generated_at"
                },
                {
                    "name": "has_source_ids",
                    "fields": ["matches[].id", "user_orders[].order_id", "group_messages[].message_id"],
                    "pass_when": "present_for_claimed_sources",
                    "failure_warning": "missing_source_ids"
                },
                {
                    "name": "tool_readiness",
                    "field": "tool_contract.tools",
                    "pass_when": "contains recommended tools or context_pack has enough evidence",
                    "failure_warning": "missing_or_partial_tool_contract"
                },
                {
                    "name": "answer_rules_available",
                    "field": "answer_policy_contract.prompt_answer_rules",
                    "pass_when": "provided_by_main_project_context_contract",
                    "failure_warning": "none"
                }
            ]
        })),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exposes_public_context_readiness_guidance() {
        let guidance = public_context_readiness_guidance("fb2").unwrap();
        assert_eq!(guidance["schema"], "fb2.context_readiness.v1");
        assert!(guidance["main_project_prompt_metadata"]
            .as_array()
            .unwrap()
            .contains(&json!("answer_rules")));
        assert!(guidance["automated_checks"]
            .as_array()
            .unwrap()
            .iter()
            .any(|check| check["name"] == "has_source_ids"));
        assert!(public_context_readiness_guidance("unknown").is_none());
    }
}
