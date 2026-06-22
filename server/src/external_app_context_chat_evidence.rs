//! server/src/external_app_context_chat_evidence.rs
//! Group chat evidence contract for external app AI-center validation.

use serde_json::{json, Value};

pub(crate) fn public_group_chat_evidence_guidance(app_id: &str) -> Option<Value> {
    match app_id {
        "fb2" => Some(json!({
            "app_id": "fb2",
            "schema": "fb2.main_project.group_chat_evidence.v1",
            "complete": true,
            "group_chat_test_method": "direct_api_read",
            "screenshots_accepted": false,
            "screenshots_role": "ui_debug_only",
            "write_policy": {
                "no_write_preflight": true,
                "visible_message_test_requires_authorization": true,
                "default_group_alias": "official",
                "default_group_id": "ext_fb2_official"
            },
            "allowed_read_routes": [
                "/api/me/groups/{group_id}/messages",
                "/api/main-project/context/group-opinion-summary",
                "/api/main-project/context/feedbacks",
                "/api/main-project/context/quality-summary"
            ],
            "required_group_message_fields": [
                "message_id",
                "type",
                "sender_id",
                "created_at",
                "text_len",
                "text_sha256"
            ],
            "required_visible_flow_evidence": [
                "baseline_messages_read",
                "visible_mention_seed_read",
                "visible_mention_ai_reply_read",
                "selected_message_seed_read",
                "selected_message_ai_reply_read",
                "summary_post_read",
                "feedback_quality_read"
            ],
            "summary_privacy": {
                "store_raw_message_text": false,
                "store_message_hash": true,
                "store_order_detail": false
            },
            "success_rule": "fb2 对话验收必须通过群聊/summary/feedback/quality 接口直接回读消息和来源证据；截图、录屏、人工日志或只有 message_id 不能证明 AI 已读群聊、已回复、已引用来源或已写回 feedback。"
        })),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::public_group_chat_evidence_guidance;
    use serde_json::Value;

    fn array_contains(value: &Value, expected: &str) -> bool {
        value
            .as_array()
            .map(|items| items.iter().any(|item| item == expected))
            .unwrap_or(false)
    }

    #[test]
    fn exposes_fb2_group_chat_direct_read_contract() {
        let contract = public_group_chat_evidence_guidance("fb2").unwrap();
        assert_eq!(
            contract["schema"],
            "fb2.main_project.group_chat_evidence.v1"
        );
        assert_eq!(contract["group_chat_test_method"], "direct_api_read");
        assert_eq!(contract["screenshots_accepted"], false);
        assert_eq!(contract["write_policy"]["no_write_preflight"], true);
        assert_eq!(
            contract["write_policy"]["visible_message_test_requires_authorization"],
            true
        );

        for field in ["message_id", "text_len", "text_sha256"] {
            assert!(array_contains(
                &contract["required_group_message_fields"],
                field
            ));
        }
        for evidence in [
            "baseline_messages_read",
            "visible_mention_ai_reply_read",
            "selected_message_ai_reply_read",
            "summary_post_read",
            "feedback_quality_read",
        ] {
            assert!(array_contains(
                &contract["required_visible_flow_evidence"],
                evidence
            ));
        }
        assert_eq!(contract["summary_privacy"]["store_raw_message_text"], false);
        assert!(public_group_chat_evidence_guidance("unknown").is_none());
    }
}
