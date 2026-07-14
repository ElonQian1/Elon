use super::*;

#[test]
fn plans_match_order_and_opinion_tools_from_user_request() {
    let plan = plan_fb2_tools(
        &json!({
            "context_quality": {"warnings": []}
        }),
        Some("今天比赛怎么预测？顺便看看我的票和群友观点"),
    );

    let names = plan.tool_names();
    assert_eq!(
        names,
        vec![
            "match_analysis_brief",
            "search_matches",
            "search_user_orders",
            "group_opinion_summary",
            "opinion_memories",
        ]
    );
    assert!(plan.tools[2].requires_external_user);
    assert_recent_group_opinion_memory_arguments(&plan);
    assert_eq!(
        plan.to_metadata()["planned_tools"][0]["trigger"].as_str(),
        Some("match_analysis_brief_needed")
    );
    assert_plan_scenario(
        &plan,
        "my_ticket_analysis",
        "current_user_only",
        "order_id",
        "guaranteed_win",
    );
    assert_plan_scenario(
        &plan,
        "group_opinion_summary",
        "group_visible",
        "message_id",
        "fabricated_group_view",
    );
}

#[test]
fn skips_user_order_search_when_context_pack_already_has_orders() {
    let plan = plan_fb2_tools(
        &json!({
            "context_quality": {"warnings": []},
            "user_orders": [{"order_id": "order-1", "visibility": "current_user_only"}],
            "metrics": {"source_counts": [{"source_type": "user_order", "count": 1}]}
        }),
        Some("今天比赛怎么预测？顺便看看我的票和群友观点"),
    );

    let names = plan.tool_names();
    assert_eq!(
        names,
        vec![
            "match_analysis_brief",
            "search_matches",
            "group_opinion_summary",
            "opinion_memories",
            "search_group_opinions",
        ]
    );
    assert_recent_group_opinion_memory_arguments(&plan);
    assert!(plan.to_metadata()["skipped_reasons"]
        .as_array()
        .unwrap()
        .contains(&json!("current_user_orders_already_in_context_pack")));
    assert_plan_scenario(
        &plan,
        "my_ticket_analysis",
        "current_user_only",
        "ticket_id",
        "other_user_order_detail",
    );
}

#[test]
fn plans_audit_when_context_pack_quality_is_blocking() {
    let plan = plan_fb2_tools(
        &json!({
            "context_audit_id": "audit-1",
            "context_quality": {"warnings": ["missing_context_pack"]}
        }),
        Some("帮我看看"),
    );

    assert!(plan
        .tools
        .iter()
        .any(|tool| tool.name == "get_context_audit"));
    assert!(plan.to_metadata()["planned_tools"][0]["evidence"]
        .as_array()
        .unwrap()
        .contains(&json!("context_quality.warning.missing_context_pack")));
}

#[test]
fn plans_platform_orders_only_when_privileged_scope_is_enabled() {
    let disabled = plan_fb2_tools_with_platform_scope(
        &json!({
            "context_quality": {"warnings": []}
        }),
        Some("平台今天订单风险集中在哪些方向？"),
        false,
    );
    assert!(!disabled.tool_names().contains(&"platform_orders"));
    assert!(disabled.to_metadata()["skipped_reasons"]
        .as_array()
        .unwrap()
        .contains(&json!("platform_order_summary_disabled")));

    let enabled = plan_fb2_tools_with_platform_scope(
        &json!({
            "context_quality": {"warnings": []}
        }),
        Some("平台今天订单风险集中在哪些方向？"),
        true,
    );
    assert!(enabled.tool_names().contains(&"platform_orders"));
    let metadata = enabled.to_metadata();
    let platform_tool = metadata["planned_tools"]
        .as_array()
        .unwrap()
        .iter()
        .find(|tool| tool["name"].as_str() == Some("platform_orders"))
        .unwrap();
    assert_eq!(
        platform_tool["trigger"].as_str(),
        Some("platform_order_summary_needed")
    );
    assert_plan_scenario(
        &enabled,
        "platform_order_risk",
        "anonymous_aggregate_only",
        "platform_order_summary",
        "user_identity_leak",
    );
}

#[test]
fn does_not_plan_platform_orders_for_personal_or_group_risk_questions() {
    for query in [
        "帮我分析我的票有什么风险",
        "群里大家怎么看西班牙这场风险",
        "群里大家怎么看西班牙这场？只说群友观点和AI推断，不要平台订单汇总。",
        "这条消息说得对吗，有没有重注风险",
    ] {
        let plan = plan_fb2_tools_with_platform_scope(
            &json!({
                "context_quality": {"warnings": []}
            }),
            Some(query),
            true,
        );

        assert!(
            !plan.tool_names().contains(&"platform_orders"),
            "personal/group query should not plan platform summary: {query}"
        );
    }
}

#[test]
fn plans_opinion_memories_for_group_history_questions() {
    let plan = plan_fb2_tools(
        &json!({
            "context_quality": {"warnings": []}
        }),
        Some("群里大家以前对这场有什么观点和建议？"),
    );

    assert!(plan.tool_names().contains(&"search_group_opinions"));
    assert!(plan.tool_names().contains(&"opinion_memories"));
    assert_recent_group_opinion_memory_arguments(&plan);
    let metadata = plan.to_metadata();
    assert!(metadata["planned_tools"]
        .as_array()
        .unwrap()
        .iter()
        .any(|tool| {
            tool["name"].as_str() == Some("opinion_memories")
                && tool["trigger"].as_str() == Some("group_opinion_memory_needed")
        }));
}

#[test]
fn plans_opinion_result_review_tools_for_quality_questions() {
    let plan = plan_fb2_tools(
        &json!({
            "context_quality": {"warnings": []}
        }),
        Some("群里大家以前观点复盘准不准？具体哪些观点说对了？"),
    );

    let names = plan.tool_names();
    assert!(names.contains(&"opinion_memories"));
    assert!(names.contains(&"opinion_result_review_summary"));
    assert!(names.contains(&"opinion_result_reviews"));
    let metadata = plan.to_metadata();
    assert!(metadata["planned_tools"]
        .as_array()
        .unwrap()
        .iter()
        .any(|tool| {
            tool["name"].as_str() == Some("opinion_result_review_summary")
                && tool["trigger"].as_str() == Some("opinion_result_review_summary_needed")
        }));
}

#[test]
fn plans_opinion_result_review_for_message_correctness_questions() {
    let plan = plan_fb2_tools(
        &json!({
            "context_quality": {"warnings": []}
        }),
        Some("这条消息说得对吗？靠谱吗？"),
    );

    let names = plan.tool_names();
    assert!(names.contains(&"opinion_result_review_summary"));
    assert!(names.contains(&"opinion_result_reviews"));
    assert!(plan
        .tools
        .iter()
        .filter(|tool| {
            matches!(
                tool.name,
                "opinion_result_review_summary" | "opinion_result_reviews"
            )
        })
        .all(|tool| !tool.requires_external_user));
}

#[test]
fn plans_opinion_adoption_tools_for_adoption_questions() {
    let plan = plan_fb2_tools(
        &json!({
            "context_quality": {"warnings": []}
        }),
        Some("AI 之前采纳了哪些群友观点？列出具体样本"),
    );

    let names = plan.tool_names();
    assert!(names.contains(&"opinion_adoption_summary"));
    assert!(names.contains(&"list_opinion_adoptions"));
}

#[test]
fn records_skipped_reason_when_no_tool_matches() {
    let plan = plan_fb2_tools(&json!({"context_quality": {"warnings": []}}), Some("你好"));

    assert!(plan.is_empty());
    assert!(plan.to_metadata()["skipped_reasons"]
        .as_array()
        .unwrap()
        .contains(&json!("no_fb2_tool_trigger_matched")));
}

fn assert_recent_group_opinion_memory_arguments(plan: &Fb2ToolPlan) {
    let tool = plan
        .tools
        .iter()
        .find(|tool| tool.name == "opinion_memories")
        .expect("opinion_memories should be planned");
    assert_eq!(tool.arguments["include_expired"].as_bool(), Some(false));
    assert_eq!(tool.arguments["limit"].as_u64(), Some(12));
    assert!(
            tool.arguments.get("query").is_none(),
            "opinion_memories must default to recent group memories instead of over-filtering by the raw user query"
        );
}

fn assert_plan_scenario(
    plan: &Fb2ToolPlan,
    scenario_id: &str,
    permission_scope: &str,
    required_citation: &str,
    forbidden_output: &str,
) {
    let metadata = plan.to_metadata();
    let scenarios = metadata["domain_scenario_selection"]["selected_scenarios"]
        .as_array()
        .expect("selected scenarios");
    let scenario = scenarios
        .iter()
        .find(|scenario| scenario["id"].as_str() == Some(scenario_id))
        .unwrap_or_else(|| panic!("missing scenario {scenario_id}: {scenarios:?}"));
    assert_eq!(
        scenario["permission_scope"].as_str(),
        Some(permission_scope)
    );
    assert!(scenario["required_citations"]
        .as_array()
        .unwrap()
        .contains(&json!(required_citation)));
    assert!(scenario["forbidden_outputs"]
        .as_array()
        .unwrap()
        .contains(&json!(forbidden_output)));
}
