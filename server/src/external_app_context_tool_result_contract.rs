//! server/src/external_app_context_tool_result_contract.rs
//! Public contract for normalized external app tool result envelopes.

use serde_json::{json, Value};

pub(crate) fn public_tool_result_envelope_guidance(app_id: &str) -> Option<Value> {
    match app_id {
        "fb2" => Some(json!({
            "app_id": "fb2",
            "schema": "fb2.tool_result_envelope.v1",
            "normalized_result_schema": "external_app.normalized_tool_result.v1",
            "normalizer": "external_app_context_tool_result::normalize_parsed_tool_result",
            "normalized_envelope": {
                "required_fields": [
                    "schema",
                    "tool_name",
                    "request_id",
                    "status",
                    "success",
                    "data",
                    "error",
                    "generated_at",
                    "source_ids",
                    "visibility",
                    "metrics",
                    "grounding",
                    "reason"
                ],
                "rule": "主项目只把 normalized tool result 注入 prompt；fb2 原始响应必须先归一化，避免模型直接读取不稳定字段。"
            },
            "source_registry": {
                "business_source_kinds": business_source_kinds(),
                "quality_history_kinds": quality_history_kinds(),
                "rule": "工具结果的 source_ids 必须能回查到业务 source registry；feedback/opinion_adoption 默认是质量历史，不得冒充比赛、赔率或订单事实。"
            },
            "grounding": {
                "schema": "external_app.tool_result_grounding.v1",
                "statuses": [
                    {
                        "status": "grounded",
                        "facts_allowed": true,
                        "requires_caveat": false,
                        "meaning": "visibility 匹配且必需 source_ids 齐全，可作为强事实引用。"
                    },
                    {
                        "status": "weak",
                        "facts_allowed": true,
                        "requires_caveat": true,
                        "meaning": "工具成功但缺少 source_ids 或 visibility，回答必须说明证据不足。"
                    },
                    {
                        "status": "unsafe",
                        "facts_allowed": false,
                        "requires_caveat": true,
                        "meaning": "visibility 与权限预期不一致，不能用于事实回答。"
                    },
                    {
                        "status": "unavailable",
                        "facts_allowed": false,
                        "requires_caveat": true,
                        "meaning": "工具失败或不可用，只能说明缺口。"
                    }
                ],
                "required_fields": [
                    "schema",
                    "status",
                    "source_id_count",
                    "source_ids_required",
                    "expected_visibility",
                    "actual_visibility",
                    "warnings",
                    "facts_allowed",
                    "requires_caveat"
                ]
            },
            "visibility_contract": visibility_contract(),
            "feedback_writeback_rule": "只有 AI 回复正文显式提到的 source_id，且工具结果 success=true、grounding.status=grounded/weak，才允许写回 fb2 feedback.cited_sources；unsafe/unavailable 或未被提到的 source_id 不能写回。",
            "answer_source_validation": {
                "schema": "external_app.answer_source_validation.v1",
                "included_in": "/api/main-project/context/feedback payload",
                "rule": "主项目会在 feedback payload 中附带单次回答引用闭环摘要，记录 candidate/matched/unmatched source ids、matched_tool_source_ids、allowed_tool_source_ids 和 has_missing_explicit_sources；无显式来源 ID 时 feedback.missing_context=true/status=no_explicit_source_ids，未匹配来源 ID 时 feedback.wrong_context=true/status=unmatched。该字段用于审计回答是否缺来源或伪造来源，不会把 tool-only source 临时合成到 fb2 cited_sources。"
            },
            "anti_patterns": [
                "raw_tool_response_in_prompt",
                "tool_result_without_source_ids",
                "wrong_visibility_as_fact",
                "quality_metric_as_match_fact",
                "other_user_order_detail",
                "fabricated_tool_source_id"
            ]
        })),
        _ => None,
    }
}

fn business_source_kinds() -> Value {
    json!([
        "context_audit",
        "match",
        "odds",
        "user_order",
        "ticket",
        "group_message",
        "opinion_memory",
        "platform_order_summary"
    ])
}

fn quality_history_kinds() -> Value {
    json!([
        {
            "kind": "feedback",
            "scope": "quality_history",
            "default_chat_fact": false
        },
        {
            "kind": "opinion_adoption",
            "scope": "quality_history",
            "default_chat_fact": false
        }
    ])
}

fn visibility_contract() -> Value {
    json!([
        {
            "tools": ["search_matches", "get_match_detail", "search_group_opinions"],
            "expected_visibility": "group_context",
            "source_ids_required": true
        },
        {
            "tools": ["match_analysis_brief"],
            "expected_visibility": "match_focused_brief",
            "source_ids_required": true
        },
        {
            "tools": ["group_opinion_summary"],
            "expected_visibility": "single_group_lightweight_memory",
            "source_ids_required": true
        },
        {
            "tools": ["opinion_memories"],
            "expected_visibility": "single_group_persistent_opinion_index",
            "source_ids_required": true
        },
        {
            "tools": ["search_user_orders", "get_order_detail"],
            "expected_visibility": "current_user_only",
            "source_ids_required": true
        },
        {
            "tools": ["platform_orders"],
            "expected_visibility": "privileged_summary",
            "source_ids_required": true
        },
        {
            "tools": ["get_context_audit"],
            "expected_visibility": "audit_metadata_only",
            "source_ids_required": false
        },
        {
            "tools": ["context_audit_summary"],
            "expected_visibility": "audit_metrics_only",
            "source_ids_required": false
        }
    ])
}

#[cfg(test)]
mod tests {
    use super::*;

    fn contains_str(array: &Value, expected: &str) -> bool {
        array.as_array().unwrap().contains(&json!(expected))
    }

    #[test]
    fn exposes_fb2_tool_result_envelope_contract() {
        let contract = public_tool_result_envelope_guidance("fb2").unwrap();
        assert_eq!(contract["schema"], "fb2.tool_result_envelope.v1");
        assert_eq!(
            contract["normalized_result_schema"],
            "external_app.normalized_tool_result.v1"
        );

        let fields = &contract["normalized_envelope"]["required_fields"];
        for field in ["schema", "source_ids", "visibility", "grounding"] {
            assert!(contains_str(fields, field));
        }

        let business_kinds = &contract["source_registry"]["business_source_kinds"];
        assert!(contains_str(business_kinds, "match"));
        assert!(contains_str(business_kinds, "user_order"));
        assert!(contains_str(business_kinds, "platform_order_summary"));
        assert!(!contains_str(business_kinds, "feedback"));

        let quality_kinds = contract["source_registry"]["quality_history_kinds"]
            .as_array()
            .unwrap();
        assert!(quality_kinds.iter().any(|kind| kind["kind"] == "feedback"
            && kind["scope"] == "quality_history"
            && kind["default_chat_fact"] == false));
        assert_eq!(
            contract["answer_source_validation"]["schema"],
            "external_app.answer_source_validation.v1"
        );
        assert!(contract["answer_source_validation"]["rule"]
            .as_str()
            .unwrap()
            .contains("matched_tool_source_ids"));
        assert!(contract["answer_source_validation"]["rule"]
            .as_str()
            .unwrap()
            .contains("has_missing_explicit_sources"));
    }

    #[test]
    fn exposes_grounding_status_and_visibility_rules() {
        let contract = public_tool_result_envelope_guidance("fb2").unwrap();
        let statuses = contract["grounding"]["statuses"].as_array().unwrap();
        for status in ["grounded", "weak", "unsafe", "unavailable"] {
            assert!(statuses.iter().any(|item| item["status"] == status));
        }
        assert!(statuses
            .iter()
            .any(|item| item["status"] == "unsafe" && item["facts_allowed"] == false));

        let visibility = contract["visibility_contract"].as_array().unwrap();
        assert!(visibility.iter().any(|item| {
            item["expected_visibility"] == "current_user_only"
                && item["source_ids_required"] == true
                && item["tools"]
                    .as_array()
                    .unwrap()
                    .contains(&json!("search_user_orders"))
        }));
        assert!(visibility.iter().any(|item| {
            item["expected_visibility"] == "privileged_summary"
                && item["tools"]
                    .as_array()
                    .unwrap()
                    .contains(&json!("platform_orders"))
        }));
    }
}
