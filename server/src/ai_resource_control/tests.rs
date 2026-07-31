use super::{
    model::{default_policy, AiResourceSummary, AiRoutePreviewRequest, UpdateAiResourcePolicy},
    service::{select_route, validate_policy},
};

fn resource(class: &str, scope: &str) -> AiResourceSummary {
    AiResourceSummary {
        resource_id: format!("{class}:1"),
        resource_class: class.to_string(),
        label: class.to_string(),
        provider: "test".to_string(),
        model: None,
        availability: "ready".to_string(),
        execution_scope: scope.to_string(),
        cost_basis: "test".to_string(),
        quota_state: "external_unverified".to_string(),
        task_kinds: vec!["code".to_string()],
        estimated_unit_cost_micros: None,
        evidence: Vec::new(),
    }
}

#[test]
fn policy_requires_the_same_unique_enabled_and_priority_classes() {
    let invalid = UpdateAiResourcePolicy {
        enabled_classes: vec!["own_codex".to_string(), "remote_node".to_string()],
        priority: vec!["own_codex".to_string()],
        allow_fallback: true,
        privacy_mode: "prefer_local".to_string(),
        max_estimated_unit_cost_micros: None,
    };
    assert!(validate_policy(&invalid).is_err());
}

#[test]
fn preview_respects_priority_and_does_not_start_execution() {
    let mut policy = default_policy("project-1", "user-1");
    policy.privacy_mode = "balanced".to_string();
    let preview = select_route(
        "project-1",
        &policy,
        vec![
            resource("platform_model", "platform_runtime"),
            resource("own_codex", "authorized_account"),
        ],
        AiRoutePreviewRequest {
            task_kind: "code".to_string(),
            preferred_model: None,
            require_local_execution: false,
        },
    );
    assert_eq!(
        preview.selected.unwrap().resource_class,
        "own_codex".to_string()
    );
    assert!(!preview.execution_started);
    assert!(!preview.quota_verified);
}

#[test]
fn local_requirement_only_selects_user_owned_node() {
    let policy = default_policy("project-1", "user-1");
    let preview = select_route(
        "project-1",
        &policy,
        vec![
            resource("own_codex", "authorized_account"),
            resource("remote_node", "user_owned_node"),
        ],
        AiRoutePreviewRequest {
            task_kind: "code".to_string(),
            preferred_model: None,
            require_local_execution: true,
        },
    );
    assert_eq!(
        preview.selected.unwrap().resource_class,
        "remote_node".to_string()
    );
}
