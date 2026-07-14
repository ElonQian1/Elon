use super::public_context_pack_template_guidance;
use serde_json::json;

#[test]
fn exposes_fb2_context_pack_template_contract() {
    let contract = public_context_pack_template_guidance("fb2").unwrap();

    assert_eq!(contract["schema"], "fb2.context_pack_template.v1");
    assert_eq!(contract["body"]["wrapper"], "fb2_context_pack");
    assert_eq!(
        contract["mcp_status"],
        "future_wrapper_not_first_phase_fact_source"
    );
    assert_eq!(contract["complete"], true);

    let sections = contract["required_section_order"].as_array().unwrap();
    for section in [
        "usage_boundary",
        "match_facts",
        "user_order_slice",
        "platform_order_summary",
        "group_opinion_slice",
        "retrieval_evidence",
        "quality_feedback",
    ] {
        assert!(sections.contains(&json!(section)));
    }

    let metadata = contract["required_metadata"].as_array().unwrap();
    assert!(metadata.contains(&json!("citation_sources")));
    assert!(metadata.contains(&json!("preflight_readiness")));

    assert_eq!(
        contract["retrieval_evidence_item_shape"]["schema"],
        "fb2.retrieval_evidence_item.v1"
    );
    let evidence_fields = contract["retrieval_evidence_item_shape"]["required_fields"]
        .as_array()
        .unwrap();
    for field in [
        "source_id",
        "source_kind",
        "lane_id",
        "index_id",
        "reason",
        "freshness",
        "permission_scope",
        "citation_source_id",
    ] {
        assert!(evidence_fields.contains(&json!(field)));
    }

    let business_kinds = contract["citation_source_shape"]["business_source_kinds"]
        .as_array()
        .unwrap();
    assert!(business_kinds.contains(&json!("match")));
    assert!(business_kinds.contains(&json!("user_order")));
    assert!(business_kinds.contains(&json!("opinion_memory")));
    assert!(!business_kinds.contains(&json!("feedback")));
}

#[test]
fn ignores_unknown_apps() {
    assert!(public_context_pack_template_guidance("unknown").is_none());
}
