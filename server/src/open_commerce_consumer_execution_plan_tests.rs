use serde_json::json;

use crate::open_commerce_app_block_model::OpenCommerceAppBlocked;

use super::{
    test_support::{
        fixture, AUTHORIZED_ACTION, AUTHORIZED_QUERY, FREE_AUTHORIZED_QUERY, PUBLIC_ACTION,
        PUBLIC_QUERY,
    },
    ConsumerCapabilityExecutionPlan,
};

fn plan(
    fixture: &super::test_support::Fixture,
    app_id: &str,
    uses_default_mcp_identity: bool,
    capability_key: &str,
) -> anyhow::Result<ConsumerCapabilityExecutionPlan> {
    super::plan(
        &fixture.store,
        &fixture.consumer_id,
        app_id,
        uses_default_mcp_identity,
        &fixture.merchant_id,
        capability_key,
        &json!({"sku":"latte"}),
    )
}

#[tokio::test]
async fn public_default_identity_and_action_steps_are_stable() {
    let fixture = fixture();
    let public_query = plan(&fixture, "mcp-client", true, PUBLIC_QUERY).unwrap();
    assert_eq!(public_query.readiness, "invoke_ready");
    assert_eq!(public_query.app_identity_kind, "mcp_default_system");
    assert!(public_query.grant_id.is_none());
    assert_eq!(
        public_query
            .next_steps
            .iter()
            .map(|step| step.key)
            .collect::<Vec<_>>(),
        ["invoke"]
    );

    let public_action = plan(&fixture, "mcp-client", true, PUBLIC_ACTION).unwrap();
    assert_eq!(public_action.readiness, "action_confirmation_required");
    assert_eq!(
        public_action
            .next_steps
            .iter()
            .map(|step| (
                step.order,
                step.key,
                step.requires_explicit_user_confirmation
            ))
            .collect::<Vec<_>>(),
        [
            (1, "prepare_action_confirmation", false),
            (2, "obtain_explicit_user_confirmation", true),
            (3, "confirm_action_confirmation", true),
            (4, "invoke", false),
        ]
    );
    assert_eq!(
        public_action.next_steps[0].mcp_tool,
        Some("open_commerce_prepare_action_confirmation")
    );
    assert_eq!(
        public_action.next_steps[2].mcp_tool,
        Some("open_commerce_confirm_action_confirmation")
    );

    let authorized_default = plan(&fixture, "mcp-client", true, AUTHORIZED_QUERY).unwrap();
    assert_eq!(authorized_default.readiness, "app_registration_required");
    assert_eq!(
        authorized_default.next_steps[0].key,
        "register_developer_app"
    );

    let routed = crate::open_commerce_mcp::call_tool(
        &fixture.store,
        &fixture.consumer_project_id,
        &fixture.consumer_id,
        "owner",
        "mcp-client",
        json!({
            "name":"open_commerce_plan_consumer_capability",
            "arguments":{
                "merchant_id":fixture.merchant_id,
                "capability_key":PUBLIC_QUERY,
                "input":{"sku":"latte"}
            }
        }),
    )
    .await
    .unwrap();
    assert_eq!(
        routed["structuredContent"]["schema"],
        "open_commerce.consumer_capability_execution_plan.v1"
    );
    assert_eq!(routed["structuredContent"]["readiness"], "invoke_ready");
    assert_eq!(routed["structuredContent"]["side_effects_created"], false);

    let definition = crate::open_commerce_consumer_discovery_mcp::definitions()
        .into_iter()
        .find(|definition| definition["name"] == "open_commerce_plan_consumer_capability")
        .unwrap();
    assert_eq!(definition["annotations"]["readOnlyHint"], true);
    assert_eq!(definition["annotations"]["destructiveHint"], false);
    assert_eq!(definition["inputSchema"]["additionalProperties"], false);
}

#[test]
fn authorization_lifecycle_requires_a_current_active_grant() {
    let lifecycle = fixture();
    let missing = plan(&lifecycle, &lifecycle.app_id, false, AUTHORIZED_QUERY).unwrap();
    assert_eq!(missing.readiness, "authorization_request_required");

    let pending_request = lifecycle.create_request(AUTHORIZED_QUERY);
    let pending = plan(&lifecycle, &lifecycle.app_id, false, AUTHORIZED_QUERY).unwrap();
    assert_eq!(pending.readiness, "authorization_pending");
    assert_eq!(
        pending.authorization_request_id.as_deref(),
        Some(pending_request.id.as_str())
    );

    let grant = lifecycle.grant(AUTHORIZED_QUERY, Some(5), Some(5_000), "CNY");
    lifecycle.approve_request(&pending_request.id, &grant.id);
    let approved = plan(&lifecycle, &lifecycle.app_id, false, AUTHORIZED_QUERY).unwrap();
    assert_eq!(approved.readiness, "invoke_ready");
    assert_eq!(approved.grant_id.as_deref(), Some(grant.id.as_str()));
    assert_eq!(approved.grant_budget_status, Some("available"));

    lifecycle
        .store
        .revoke_open_commerce_grant(&lifecycle.merchant_project_id, &grant.id)
        .unwrap();
    let revoked = plan(&lifecycle, &lifecycle.app_id, false, AUTHORIZED_QUERY).unwrap();
    assert_eq!(revoked.readiness, "authorization_request_required");
    assert!(revoked.grant_id.is_none());
    assert!(revoked.authorization_request_id.is_none());

    let expired_fixture = fixture();
    let expired_request = expired_fixture.create_request(AUTHORIZED_QUERY);
    let expired_grant = expired_fixture.grant(AUTHORIZED_QUERY, Some(5), Some(5_000), "CNY");
    expired_fixture.approve_request(&expired_request.id, &expired_grant.id);
    expired_fixture.expire_grant(&expired_grant.id);
    let expired = plan(
        &expired_fixture,
        &expired_fixture.app_id,
        false,
        AUTHORIZED_QUERY,
    )
    .unwrap();
    assert_eq!(expired.readiness, "authorization_request_required");
    assert!(expired.grant_id.is_none());
}

#[test]
fn blocked_or_unowned_apps_fail_closed_before_grant_selection() {
    let fixture = fixture();
    let grant = fixture.grant(AUTHORIZED_QUERY, None, None, "CNY");
    fixture.block_app();
    assert!(fixture
        .store
        .open_commerce_grant(&grant.id)
        .unwrap()
        .revoked_at
        .is_some());
    let blocked = plan(&fixture, &fixture.app_id, false, AUTHORIZED_QUERY).unwrap_err();
    assert!(blocked.is::<OpenCommerceAppBlocked>());

    let other_app = plan(&fixture, &fixture.other_app_id, false, AUTHORIZED_QUERY).unwrap_err();
    assert!(other_app
        .to_string()
        .contains("当前用户不能代表该开发者应用"));
    assert!(!other_app.to_string().contains(&fixture.other_user_id));
}

#[test]
fn grant_selection_checks_all_active_grants_and_reports_stable_budget_reasons() {
    let multi = fixture();
    let old_available = multi.grant(AUTHORIZED_QUERY, Some(5), Some(5_000), "CNY");
    multi.set_grant_created_at(&old_available.id, "2025-01-01T00:00:00Z");
    let newest_exhausted = multi.grant(AUTHORIZED_QUERY, Some(1), Some(5_000), "CNY");
    multi.set_grant_usage(&newest_exhausted.id, 1, 1_000);
    multi.set_grant_created_at(&newest_exhausted.id, "2026-01-01T00:00:00Z");
    let selected = plan(&multi, &multi.app_id, false, AUTHORIZED_QUERY).unwrap();
    assert_eq!(selected.readiness, "invoke_ready");
    assert_eq!(
        selected.grant_id.as_deref(),
        Some(old_available.id.as_str())
    );
    assert_eq!(selected.grant_budget_status, Some("available"));

    let count = fixture();
    let count_grant = count.grant(AUTHORIZED_QUERY, Some(1), Some(5_000), "CNY");
    count.set_grant_usage(&count_grant.id, 1, 0);
    assert_budget_status(
        &count,
        AUTHORIZED_QUERY,
        "grant_refresh_required",
        "invocation_budget_exhausted",
    );
    let renewal = count.create_request(AUTHORIZED_QUERY);
    let pending = plan(&count, &count.app_id, false, AUTHORIZED_QUERY).unwrap();
    assert_eq!(pending.readiness, "authorization_pending");
    assert_eq!(
        pending.authorization_request_id.as_deref(),
        Some(renewal.id.as_str())
    );
    assert_eq!(
        pending.grant_budget_status,
        Some("invocation_budget_exhausted")
    );

    let amount = fixture();
    let amount_grant = amount.grant(AUTHORIZED_QUERY, None, Some(1_000), "CNY");
    amount.set_grant_usage(&amount_grant.id, 0, 1);
    assert_budget_status(
        &amount,
        AUTHORIZED_QUERY,
        "grant_refresh_required",
        "amount_budget_exhausted",
    );

    let currency = fixture();
    currency.grant(AUTHORIZED_QUERY, None, Some(5_000), "USD");
    assert_budget_status(
        &currency,
        AUTHORIZED_QUERY,
        "grant_refresh_required",
        "budget_currency_mismatch",
    );

    let zero_price = fixture();
    let zero_grant = zero_price.grant(FREE_AUTHORIZED_QUERY, None, Some(1), "CNY");
    zero_price.set_grant_usage(&zero_grant.id, 0, 1);
    let zero_plan = plan(
        &zero_price,
        &zero_price.app_id,
        false,
        FREE_AUTHORIZED_QUERY,
    )
    .unwrap();
    assert_eq!(zero_plan.readiness, "invoke_ready");
    assert_eq!(zero_plan.grant_budget_status, Some("available"));
}

#[test]
fn invalid_input_disabled_app_unknown_capability_and_unpublished_merchant_fail_closed() {
    let invalid = fixture();
    let input_error = super::plan(
        &invalid.store,
        &invalid.consumer_id,
        &invalid.app_id,
        false,
        &invalid.merchant_id,
        PUBLIC_QUERY,
        &json!({"unexpected":true}),
    )
    .unwrap_err();
    assert!(input_error
        .to_string()
        .contains("调用输入 schema不符合能力契约"));

    let unknown = plan(&invalid, &invalid.app_id, false, "missing.capability").unwrap_err();
    assert!(unknown.to_string().contains("公开目录中不存在该能力"));

    invalid
        .store
        .disable_open_commerce_developer_app(&invalid.consumer_project_id, &invalid.app_record_id)
        .unwrap();
    let disabled = plan(&invalid, &invalid.app_id, false, PUBLIC_QUERY).unwrap_err();
    assert!(disabled.to_string().contains("开发者应用已停用"));

    let unpublished = fixture();
    unpublished.set_published(false);
    let hidden = plan(&unpublished, &unpublished.app_id, false, PUBLIC_QUERY).unwrap_err();
    assert!(hidden.to_string().contains("商户节点未发布到开放目录"));
}

#[tokio::test]
async fn repeated_query_and_action_planning_create_no_side_effects_or_budget_changes() {
    let fixture = fixture();
    let query_grant = fixture.grant(AUTHORIZED_QUERY, Some(3), Some(3_000), "CNY");
    let action_grant = fixture.grant(AUTHORIZED_ACTION, Some(3), Some(3_000), "CNY");
    let before = fixture.snapshot();

    for _ in 0..3 {
        let query = plan(&fixture, &fixture.app_id, false, AUTHORIZED_QUERY).unwrap();
        assert_eq!(query.readiness, "invoke_ready");
        assert!(!query.side_effects_created);
        let action = plan(&fixture, &fixture.app_id, false, AUTHORIZED_ACTION).unwrap();
        assert_eq!(action.readiness, "action_confirmation_required");
        assert!(!action.side_effects_created);
    }
    let routed = crate::open_commerce_mcp::call_tool(
        &fixture.store,
        &fixture.consumer_project_id,
        &fixture.consumer_id,
        "owner",
        &fixture.app_id,
        json!({
            "name":"open_commerce_plan_consumer_capability",
            "arguments":{
                "merchant_id":fixture.merchant_id,
                "capability_key":AUTHORIZED_ACTION,
                "input":{"sku":"latte"}
            }
        }),
    )
    .await
    .unwrap();
    assert_eq!(
        routed["structuredContent"]["readiness"],
        "action_confirmation_required"
    );
    assert_eq!(before, fixture.snapshot());

    for grant_id in [query_grant.id, action_grant.id] {
        let grant = fixture.store.open_commerce_grant(&grant_id).unwrap();
        assert_eq!(grant.used_invocations, 0);
        assert_eq!(grant.used_amount_micros, 0);
    }
}

fn assert_budget_status(
    fixture: &super::test_support::Fixture,
    capability_key: &str,
    readiness: &str,
    budget_status: &str,
) {
    let result = plan(fixture, &fixture.app_id, false, capability_key).unwrap();
    assert_eq!(result.readiness, readiness);
    assert_eq!(result.grant_budget_status, Some(budget_status));
    assert_eq!(result.next_steps.len(), 1);
    assert_eq!(result.next_steps[0].key, "request_grant_refresh");
}
