use super::*;

#[test]
fn exposes_public_fb2_answer_policy() {
    let guidance = public_answer_policy_guidance("fb2").unwrap();
    assert_eq!(guidance["schema"], "fb2.answer_policy.v1");
    assert!(guidance["grounding_sections"]
        .as_array()
        .unwrap()
        .iter()
        .any(|section| section["name"] == "group_opinions"));
    assert!(guidance["grounding_sections"]
        .as_array()
        .unwrap()
        .iter()
        .any(|section| section["name"] == "platform_order_summary"
            && section["visibility"] == "anonymous_aggregate_only"));
    assert!(guidance["forbidden_behaviors"]
        .as_array()
        .unwrap()
        .iter()
        .any(|rule| rule.as_str().unwrap_or("").contains("编造")));
    assert!(guidance["canonical_eval_questions"]
        .as_array()
        .unwrap()
        .contains(&json!("帮我看看我今天的票风险在哪里？")));
    let scenarios = guidance["eval_scenarios"].as_array().unwrap();
    for expected_id in [
        "today_matches_analysis",
        "my_ticket_analysis",
        "platform_order_risk",
        "group_opinion_summary",
        "selected_message_review",
        "group_discussion_summary_post",
        "source_reference_audit",
    ] {
        assert!(scenarios
            .iter()
            .any(|scenario| scenario["id"] == expected_id));
    }
    assert!(scenarios
        .iter()
        .any(|scenario| scenario["id"] == "my_ticket_analysis"
            && scenario["permission_boundary"] == "current_user_only"
            && scenario["required_headers"]
                .as_array()
                .unwrap()
                .contains(&json!("X-FB2-AI-CONTEXT-USER-ID"))));
    assert!(scenarios
        .iter()
        .any(|scenario| scenario["id"] == "platform_order_risk"
            && scenario["permission_boundary"] == "anonymous_aggregate_only"
            && scenario["forbidden_outputs"]
                .as_array()
                .unwrap()
                .contains(&json!("single_user_order_detail"))));
    assert_eq!(
        guidance["default_answer_policy"]["risk_rules"]["no_guaranteed_win"],
        true
    );
    assert!(public_answer_policy_guidance("unknown").is_none());
}

#[test]
fn prompt_rules_keep_source_boundaries() {
    let block = prompt_answer_rules_block(&json!({}));
    assert!(block.contains("<answer_rules>"));
    assert!(block.contains("必须区分"));
    assert!(block.contains("数据事实："));
    assert!(block.contains("风险边界："));
    assert!(block.contains("不保证命中"));
    assert!(block.contains("message id"));
    assert!(block.contains("platform_order_summary source id"));
    assert!(block.contains("不能编造"));
}
