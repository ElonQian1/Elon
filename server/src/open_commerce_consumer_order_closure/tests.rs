use serde_json::json;

use super::{mcp, service, support};

#[test]
fn closure_projects_pending_and_applied_without_leaking_internal_handoff_state() {
    let fixture = support::fixture();
    let pending =
        service::get_order_closure(&fixture.store, &fixture.consumer_id, &fixture.invocation_id)
            .unwrap();
    assert_eq!(pending.closure_status, "merchant_confirmed_erp_pending");
    assert!(pending.erp_handoff.is_none());
    assert_eq!(pending.merchant_order.amount_minor, Some(2600));
    assert_eq!(pending.platform_meter.amount_micros, 1_000);
    assert!(!pending.funds_moved);

    support::record_handoff(
        &fixture,
        &fixture.invocation_id,
        "closure.erp.applied",
        "applied",
        "2026-08-13T12:01:00Z",
    );
    let applied =
        service::get_order_closure(&fixture.store, &fixture.consumer_id, &fixture.invocation_id)
            .unwrap();
    assert_eq!(applied.closure_status, "erp_recorded");
    assert_eq!(applied.erp_handoff.as_ref().unwrap().status, "applied");
    assert_eq!(
        applied
            .erp_handoff
            .as_ref()
            .unwrap()
            .target_reference_sha256
            .as_deref()
            .unwrap()
            .len(),
        64
    );

    let serialized = serde_json::to_string(&applied).unwrap();
    assert!(!serialized.contains("erp-order-private-1001"));
    for forbidden in [
        "project_id",
        "integration_id",
        "adapter_credential",
        "adapter_claim_id",
        "recorded_by_user_id",
        "lease_token",
        "request_hash",
        "grant_id",
    ] {
        assert!(!serialized.contains(forbidden), "leaked {forbidden}");
    }
}

#[test]
fn closure_uses_latest_rejected_and_ignored_handoff_states() {
    let fixture = support::fixture();
    support::record_handoff(
        &fixture,
        &fixture.invocation_id,
        "closure.erp.rejected",
        "rejected",
        "2026-08-13T12:01:00Z",
    );
    let rejected =
        service::get_order_closure(&fixture.store, &fixture.consumer_id, &fixture.invocation_id)
            .unwrap();
    assert_eq!(rejected.closure_status, "erp_retry_required");
    assert_eq!(
        rejected.erp_handoff.as_ref().unwrap().error_code.as_deref(),
        Some("adapter_failed")
    );

    support::record_handoff(
        &fixture,
        &fixture.invocation_id,
        "closure.erp.ignored",
        "ignored",
        "2026-08-13T12:02:00Z",
    );
    let ignored =
        service::get_order_closure(&fixture.store, &fixture.consumer_id, &fixture.invocation_id)
            .unwrap();
    assert_eq!(ignored.closure_status, "erp_ignored");
    assert_eq!(ignored.erp_handoff.as_ref().unwrap().status, "ignored");
}

#[test]
fn closure_is_private_terminal_and_order_only() {
    let fixture = support::fixture();
    assert!(service::get_order_closure(
        &fixture.store,
        &fixture.other_consumer_id,
        &fixture.invocation_id,
    )
    .unwrap_err()
    .to_string()
    .contains("不存在"));

    let started = support::create_started_invocation(&fixture, "closure-started");
    assert!(
        service::get_order_closure(&fixture.store, &fixture.consumer_id, &started)
            .unwrap_err()
            .to_string()
            .contains("不存在")
    );
    let failed = support::create_terminal_invocation(&fixture, "closure-failed", None, true);
    assert!(
        service::get_order_closure(&fixture.store, &fixture.consumer_id, &failed)
            .unwrap_err()
            .to_string()
            .contains("不存在")
    );
    let missing = support::create_terminal_invocation(
        &fixture,
        "closure-no-receipt",
        Some(json!({"order":{"id":"missing-receipt"}})),
        false,
    );
    assert!(
        service::get_order_closure(&fixture.store, &fixture.consumer_id, &missing)
            .unwrap_err()
            .to_string()
            .contains("不存在")
    );
    let non_order = support::create_terminal_invocation(
        &fixture,
        "closure-non-order",
        Some(json!({"_yilong_business_receipt":support::business_receipt("booking")})),
        false,
    );
    assert!(
        service::get_order_closure(&fixture.store, &fixture.consumer_id, &non_order)
            .unwrap_err()
            .to_string()
            .contains("不存在")
    );
    let invalid = support::create_terminal_invocation(
        &fixture,
        "closure-invalid-receipt",
        Some(json!({"_yilong_business_receipt":{"schema":"wrong"}})),
        false,
    );
    assert!(
        service::get_order_closure(&fixture.store, &fixture.consumer_id, &invalid)
            .unwrap_err()
            .to_string()
            .contains("格式无效")
    );
}

#[test]
fn mcp_exposes_the_same_read_only_closure_service() {
    let fixture = support::fixture();
    let definitions = mcp::definitions();
    assert_eq!(definitions.len(), 1);
    assert_eq!(definitions[0]["name"], "open_commerce_get_my_order_closure");
    assert_eq!(definitions[0]["annotations"]["readOnlyHint"], true);

    let from_service =
        service::get_order_closure(&fixture.store, &fixture.consumer_id, &fixture.invocation_id)
            .unwrap();
    let from_mcp = mcp::call_if_handled(
        &fixture.store,
        &fixture.consumer_id,
        "open_commerce_get_my_order_closure",
        json!({"invocation_id":fixture.invocation_id}),
    )
    .unwrap()
    .unwrap();
    assert_eq!(
        from_mcp,
        serde_json::to_value(from_service).unwrap(),
        "HTTP and MCP adapters must share the same projection service"
    );
    assert!(mcp::call_if_handled(
        &fixture.store,
        &fixture.consumer_id,
        "unknown_tool",
        json!({}),
    )
    .unwrap()
    .is_none());
}
