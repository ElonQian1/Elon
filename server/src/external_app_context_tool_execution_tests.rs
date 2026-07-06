    use super::*;

    #[test]
    fn exposes_fb2_tool_execution_contract() {
        let contract = public_tool_execution_guidance("fb2").unwrap();
        assert_eq!(contract["schema"], "fb2.tool_execution.v1");
        assert_eq!(contract["transport"]["path"], FB2_TOOL_EXECUTE_PATH);
        assert_eq!(
            contract["main_project_execution_result"]["results"]["grounding"]["schema"],
            "external_app.tool_result_grounding.v1"
        );
        assert!(contract["allowed_tools"]
            .as_array()
            .unwrap()
            .contains(&json!("get_match_detail")));
        assert!(contract["allowed_tools"]
            .as_array()
            .unwrap()
            .contains(&json!("platform_orders")));
        assert!(contract["allowed_tools"]
            .as_array()
            .unwrap()
            .contains(&json!("opinion_memories")));
        assert!(contract["allowed_tools"]
            .as_array()
            .unwrap()
            .contains(&json!("group_opinion_summary")));
        assert!(contract["allowed_tools"]
            .as_array()
            .unwrap()
            .contains(&json!("match_analysis_brief")));
        assert!(contract["allowed_tools"]
            .as_array()
            .unwrap()
            .contains(&json!("opinion_result_review_summary")));
        assert!(contract["permission_rules"]
            .as_array()
            .unwrap()
            .iter()
            .any(|rule| rule.as_str().unwrap_or("").contains("external_user_id")));
    }

    #[test]
    fn unknown_app_has_no_tool_execution_contract() {
        assert!(public_tool_execution_guidance("unknown").is_none());
    }

    #[test]
    fn exposes_bb64a_tool_execution_contract() {
        let contract = public_tool_execution_guidance("bb64a").unwrap();
        assert_eq!(contract["schema"], "bb64a.tool_execution.v1");
        assert!(contract["allowed_tools"]
            .as_array()
            .unwrap()
            .contains(&json!("bb64a_doctor")));
        assert!(contract["allowed_tools"]
            .as_array()
            .unwrap()
            .contains(&json!("close_all_proxies")));
    }
