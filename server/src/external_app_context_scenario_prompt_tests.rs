use super::*;
use serde_json::json;

#[test]
fn emits_ticket_match_and_group_guidance_from_plan() {
    let context = json!({
        "app_id": "fb2",
        "context_audit_id": "audit-1",
        "answer_policy": {"schema": "fb2.answer_policy.v1"}
    });
    let execution = json!({
        "app_id": "fb2",
        "plan": {
            "topic_hint": "今天比赛怎么看，帮我分析我的票和群友观点",
            "planned_tools": [
                {"name": "match_analysis_brief"},
                {"name": "search_user_orders"},
                {"name": "opinion_memories"}
            ]
        },
        "results": []
    });

    let block = prompt_domain_scenario_guidance(Some(&context), Some(&execution));

    assert!(block.contains("fb2.domain_scenario_prompt.v1"));
    assert!(block.contains("scenario=today_matches_analysis"));
    assert!(block.contains("scenario=my_ticket_analysis"));
    assert!(block.contains("scenario=group_opinion_summary"));
    assert!(block.contains("current_user_only"));
    assert!(block.contains("order_id/ticket_id/match_id"));
}

#[test]
fn returns_machine_readable_selection_for_planner_metadata() {
    let context = json!({
        "app_id": "fb2",
        "user_orders": [{"order_id": "order-1"}],
        "matches": [{"match_id": "match-1"}]
    });
    let selection = fb2_domain_scenario_selection(
        Some(&context),
        Some("帮我分析我的票"),
        &["match_analysis_brief"],
    );

    assert_eq!(
        selection["schema"].as_str(),
        Some("fb2.domain_scenario_selection.v1")
    );
    let selected = selection["selected_scenarios"].as_array().unwrap();
    assert!(selected
        .iter()
        .any(|scenario| scenario["id"] == "my_ticket_analysis"));
    let ticket = selected
        .iter()
        .find(|scenario| scenario["id"] == "my_ticket_analysis")
        .unwrap();
    assert_eq!(
        ticket["permission_scope"].as_str(),
        Some("current_user_only")
    );
    assert!(ticket["required_citations"]
        .as_array()
        .unwrap()
        .contains(&json!("order_id")));
}

#[test]
fn emits_platform_guidance_without_user_detail_leak() {
    let execution = json!({
        "app_id": "fb2",
        "plan": {
            "topic_hint": "平台今天订单风险怎么样",
            "planned_tools": [{"name": "platform_orders"}]
        },
        "results": []
    });

    let block = prompt_domain_scenario_guidance(None, Some(&execution));

    assert!(block.contains("scenario=platform_order_risk"));
    assert!(block.contains("anonymous_aggregate_only"));
    assert!(block.contains("不得暴露单个用户订单"));
}

#[test]
fn emits_summary_post_guidance_for_group_discussion_summary_intent() {
    let context = json!({
        "app_id": "fb2",
        "context_audit_id": "audit-summary",
        "group_messages": [{"message_id": "msg-1"}],
        "opinion_memories": [{"opinion_memory_id": "mem-1"}]
    });
    let execution = json!({
        "app_id": "fb2",
        "plan": {
            "topic_hint": "总结今天群聊讨论，生成总结帖",
            "planned_tools": [{"name": "group_summary_post"}]
        },
        "results": []
    });

    let block = prompt_domain_scenario_guidance(Some(&context), Some(&execution));

    assert!(block.contains("scenario=group_discussion_summary_post"));
    assert!(block.contains("message_id/opinion_memory_id/context_audit_id"));
    assert!(block.contains("群聊总结帖"));
    assert!(block.contains("不得把群友观点写成比赛事实"));
}

#[test]
fn ignores_non_fb2_context() {
    let context = json!({"app_id": "other"});

    assert!(prompt_domain_scenario_guidance(Some(&context), None).is_empty());
}
