use super::*;

#[test]
fn renders_declared_tools_without_claiming_execution() {
    let block = prompt_tool_contract_block(&json!({
        "tool_contract": {
            "schema": "fb2.tools.v1",
            "tools": [{
                "name": "get_match_detail",
                "description": "Load one match",
                "input_schema": {"type": "object"}
            }]
        }
    }));

    assert!(block.contains("get_match_detail"));
    assert!(block.contains("success=true"));
    assert!(block.contains("grounding.status=grounded"));
    assert!(block.contains("未调用工具时只能基于现有上下文回答"));
}

#[test]
fn reports_missing_tool_contract() {
    let block = prompt_tool_contract_block(&json!({}));
    assert!(block.contains("status=\"unavailable\""));
    assert_eq!(
        tool_contract_quality_warning(&json!({"tool_contract": null})),
        Some("missing_tool_contract")
    );
    assert_eq!(
        tool_contract_readiness(&json!({}))["status"].as_str(),
        Some("missing")
    );
}

#[test]
fn reports_partial_tool_readiness() {
    let readiness = tool_contract_readiness(&json!({
        "tool_contract": {
            "tools": [
                {"name": "get_match_detail"},
                {"name": "search_user_orders"},
                {"name": "get_context_audit"}
            ]
        }
    }));

    assert_eq!(readiness["status"], "partial");
    assert_eq!(readiness["declared_count"], 3);
    assert!(readiness["missing_recommended_tools"]
        .as_array()
        .unwrap()
        .contains(&json!("search_group_opinions")));
    assert!(readiness["missing_recommended_tools"]
        .as_array()
        .unwrap()
        .contains(&json!("context_audit_summary")));
}

#[test]
fn exposes_public_fb2_guidance() {
    let guidance = public_tool_contract_guidance("fb2").unwrap();
    assert_eq!(guidance["schema"], "fb2.tools.v1");
    assert!(guidance["recommended_tools"]
        .as_array()
        .unwrap()
        .iter()
        .any(|tool| tool["name"] == "context_audit_summary"));
    assert!(guidance["required_context_fields"]
        .as_array()
        .unwrap()
        .contains(&json!("answer_policy")));
    assert!(guidance["required_context_fields"]
        .as_array()
        .unwrap()
        .contains(&json!("citation_sources")));
    assert!(public_tool_contract_guidance("unknown").is_none());
}

#[test]
fn exposes_public_bb64a_guidance() {
    let guidance = public_tool_contract_guidance("bb64a").unwrap();
    assert_eq!(guidance["schema"], "bb64a.tools.v1");
    assert!(guidance["recommended_tools"]
        .as_array()
        .unwrap()
        .iter()
        .any(|tool| tool["name"] == "bb64a_doctor"));
    assert!(guidance["recommended_tools"]
        .as_array()
        .unwrap()
        .iter()
        .any(|tool| tool["permission"] == "dangerous_local_runtime_control"));
}
