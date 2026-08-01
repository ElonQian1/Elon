use super::*;

fn matter(contract: bool, decision: Option<&str>) -> ProjectAiMatter {
    ProjectAiMatter {
        id: "matter-start-gate".into(),
        project_id: "project-a".into(),
        channel_id: "channel-a".into(),
        requester_user_id: "merchant-a".into(),
        decision_user_id: decision.map(str::to_string),
        source_message_id: None,
        title: "ERP initialization".into(),
        brief: "Materialize the merchant project".into(),
        collaboration_mode: "solo".into(),
        status: MATTER_STATUS_PLAN_READY.into(),
        participant_user_ids: vec!["merchant-a".into()],
        node_policy: json!({}),
        acceptance_criteria: vec![],
        plan: if contract {
            json!({"execution_contract":{"schema":"yilong.erp.materialization_contract.v1"}})
        } else {
            json!({})
        },
        final_summary: None,
        final_decision: decision.map(str::to_string),
        created_at: "now".into(),
        updated_at: "now".into(),
    }
}

#[test]
fn erp_blueprint_execution_contract_requires_explicit_approval_before_start() {
    let error = ensure_execution_contract_approved(&matter(true, None)).unwrap_err();
    assert!(error.to_string().contains("必须先由项目成员批准"));
    ensure_execution_contract_approved(&matter(true, Some("approved"))).unwrap();
}

#[test]
fn erp_blueprint_legacy_matter_without_contract_keeps_existing_start_behavior() {
    ensure_execution_contract_approved(&matter(false, None)).unwrap();
}
