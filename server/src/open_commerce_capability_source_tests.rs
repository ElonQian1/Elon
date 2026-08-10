use super::{link_source, remove_source};
use crate::{
    open_commerce_capability_source_migration::migration_v164,
    open_commerce_capability_source_model::LinkCapabilitySourceRequest,
    open_commerce_directory_service, open_commerce_model::UpdateCapabilityRequest,
    open_commerce_service,
};

#[path = "open_commerce_capability_source_test_support.rs"]
mod support;
use support::{assert_source_kind, current_link, fixture, publish_directory, source_audit_count};

#[test]
fn eligible_receipts_rebind_with_revision_audit_and_public_projection() {
    let fixture = fixture("eligible");
    let first = fixture
        .link(&fixture.succeeded_receipt_id, "catalog")
        .unwrap();
    assert!(first.publishable);
    assert_eq!(first.revision, 1);
    assert_eq!(first.capability_version, 1);
    assert_eq!(first.receipt_status, "succeeded");
    assert_eq!(first.receipt_sha256.len(), 64);
    assert!(first
        .receipt_sha256
        .chars()
        .all(|value| value.is_ascii_hexdigit()));

    let rebound = fixture
        .link(&fixture.partial_receipt_id, "inventory")
        .unwrap();
    assert_eq!(rebound.id, first.id);
    assert_eq!(rebound.created_at, first.created_at);
    assert_eq!(rebound.revision, 2);
    assert_eq!(rebound.receipt_status, "partial");
    assert_eq!(rebound.data_domain, "inventory");

    open_commerce_directory_service::set_publication(
        &fixture.store,
        &fixture.project_id,
        &fixture.merchant_id,
        &fixture.owner(),
        true,
    )
    .unwrap();
    let detail =
        open_commerce_directory_service::discover_merchant(&fixture.store, &fixture.merchant_id)
            .unwrap();
    let capability = detail
        .capabilities
        .iter()
        .find(|value| value.capability_key == "catalog.search")
        .unwrap();
    assert_eq!(capability.source.kind, "integration_sync_receipt");
    assert_eq!(capability.source.assertion_authority, "merchant_project");
    assert!(!capability.source.externally_verified);
    assert_eq!(
        capability.source.integration_receipt_id.as_deref(),
        Some(fixture.partial_receipt_id.as_str())
    );
    assert_eq!(
        capability.source.provider_key.as_deref(),
        Some("merchant_erp")
    );
    assert_eq!(
        capability.source.connection_mode.as_deref(),
        Some("local_adapter")
    );
    assert_eq!(capability.source.data_domain.as_deref(), Some("inventory"));
    assert_eq!(capability.source.receipt_status.as_deref(), Some("partial"));
    assert_eq!(
        capability.source.receipt_sha256.as_deref(),
        Some(rebound.receipt_sha256.as_str())
    );
    assert_eq!(capability.freshness.basis, "sync_receipt_completed_at");
    assert!(!capability.freshness.externally_verified);

    let overview = open_commerce_service::overview(&fixture.store, &fixture.project_id).unwrap();
    assert_eq!(overview.capability_source_links.len(), 1);
    assert_eq!(overview.capability_source_links[0].revision, 2);
    let linked_audits = overview
        .recent_audit_events
        .iter()
        .filter(|event| event.action == "capability.source_linked")
        .collect::<Vec<_>>();
    assert_eq!(linked_audits.len(), 2);
    assert_eq!(linked_audits[0].metadata["externally_verified"], false);
    let serialized = serde_json::to_string(&detail).unwrap();
    assert!(!serialized.contains("cursor-secret"));
    assert!(!serialized.contains("scopes"));
}

#[test]
fn invalid_actor_project_merchant_domain_and_receipts_fail_closed() {
    let fixture = fixture("invalid");
    let initial_audits = source_audit_count(&fixture);
    let request = || LinkCapabilitySourceRequest {
        integration_id: fixture.integration_id.clone(),
        sync_receipt_id: fixture.succeeded_receipt_id.clone(),
        data_domain: "catalog".to_string(),
    };
    assert!(link_source(
        &fixture.store,
        &fixture.project_id,
        &fixture.capability_id,
        &fixture.viewer(),
        request(),
    )
    .unwrap_err()
    .to_string()
    .contains("编辑权限"));

    let cases = [
        (
            fixture.other_merchant_integration_id.as_str(),
            fixture.other_integration_receipt_id.as_str(),
            "catalog",
            "同一商户",
        ),
        (
            fixture.other_project_integration_id.as_str(),
            fixture.other_project_receipt_id.as_str(),
            "catalog",
            "当前项目中不存在该数据接入",
        ),
        (
            fixture.integration_id.as_str(),
            fixture.succeeded_receipt_id.as_str(),
            "finance",
            "未在该数据接入中登记",
        ),
        (
            fixture.integration_id.as_str(),
            fixture.health_receipt_id.as_str(),
            "catalog",
            "健康检查回执",
        ),
        (
            fixture.integration_id.as_str(),
            fixture.failed_receipt_id.as_str(),
            "catalog",
            "成功或部分成功",
        ),
        (
            fixture.integration_id.as_str(),
            fixture.other_integration_receipt_id.as_str(),
            "catalog",
            "不属于所选数据接入",
        ),
    ];
    for (integration_id, receipt_id, data_domain, expected) in cases {
        let error = link_source(
            &fixture.store,
            &fixture.project_id,
            &fixture.capability_id,
            &fixture.owner(),
            LinkCapabilitySourceRequest {
                integration_id: integration_id.to_string(),
                sync_receipt_id: receipt_id.to_string(),
                data_domain: data_domain.to_string(),
            },
        )
        .unwrap_err();
        assert!(error.to_string().contains(expected), "{error:#}");
    }

    let other_project_error = link_source(
        &fixture.store,
        &fixture.other_project_id,
        &fixture.capability_id,
        &fixture.owner(),
        request(),
    )
    .unwrap_err();
    assert!(other_project_error.to_string().contains("不存在该商业能力"));
    assert!(fixture
        .store
        .list_project_open_commerce_capability_source_links(&fixture.project_id)
        .unwrap()
        .is_empty());
    assert_eq!(source_audit_count(&fixture), initial_audits);
}

#[test]
fn capability_versions_and_integration_state_fail_closed_then_recover_explicitly() {
    let fixture = fixture("lifecycle");
    fixture
        .link(&fixture.succeeded_receipt_id, "catalog")
        .unwrap();
    publish_directory(&fixture);
    assert_source_kind(&fixture, "integration_sync_receipt");

    open_commerce_service::update_capability(
        &fixture.store,
        &fixture.project_id,
        &fixture.capability_id,
        &fixture.owner(),
        UpdateCapabilityRequest {
            display_name: None,
            description: Some("版本二".to_string()),
            access_level: None,
            input_schema: None,
            output_schema: None,
            handler_type: None,
            handler_config: None,
            unit_price_micros: None,
            currency: None,
            freshness_seconds: None,
            status: None,
        },
    )
    .unwrap();
    let stale = current_link(&fixture);
    assert!(!stale.publishable);
    assert_eq!(
        stale.blocking_reason.as_deref(),
        Some("capability_version_changed")
    );
    assert_source_kind(&fixture, "merchant_static_data");

    let rebound = fixture
        .link(&fixture.partial_receipt_id, "inventory")
        .unwrap();
    assert!(rebound.publishable);
    assert_eq!(rebound.capability_version, 2);
    assert_eq!(rebound.revision, 2);
    assert_source_kind(&fixture, "integration_sync_receipt");

    open_commerce_service::set_integration_enabled(
        &fixture.store,
        &fixture.project_id,
        &fixture.integration_id,
        &fixture.owner(),
        false,
    )
    .unwrap();
    let disabled = current_link(&fixture);
    assert!(!disabled.publishable);
    assert_eq!(
        disabled.blocking_reason.as_deref(),
        Some("integration_disabled")
    );
    assert_source_kind(&fixture, "merchant_static_data");

    open_commerce_service::set_integration_enabled(
        &fixture.store,
        &fixture.project_id,
        &fixture.integration_id,
        &fixture.owner(),
        true,
    )
    .unwrap();
    assert!(current_link(&fixture).publishable);
    assert_source_kind(&fixture, "integration_sync_receipt");
}

#[test]
fn unlink_is_idempotent_and_receipt_foreign_key_cascades() {
    let fixture = fixture("unlink");
    fixture
        .link(&fixture.succeeded_receipt_id, "catalog")
        .unwrap();
    let first = remove_source(
        &fixture.store,
        &fixture.project_id,
        &fixture.capability_id,
        &fixture.owner(),
    )
    .unwrap();
    assert_eq!(first["removed"], true);
    let second = remove_source(
        &fixture.store,
        &fixture.project_id,
        &fixture.capability_id,
        &fixture.owner(),
    )
    .unwrap();
    assert_eq!(second["removed"], false);
    let unlink_audits = fixture
        .store
        .list_project_open_commerce_audit(&fixture.project_id, 200)
        .unwrap()
        .into_iter()
        .filter(|event| event.action == "capability.source_unlinked")
        .count();
    assert_eq!(unlink_audits, 1);

    fixture
        .link(&fixture.succeeded_receipt_id, "catalog")
        .unwrap();
    fixture
        .store
        .conn()
        .unwrap()
        .execute(
            "DELETE FROM open_commerce_sync_receipts WHERE id = ?1",
            [&fixture.succeeded_receipt_id],
        )
        .unwrap();
    assert!(fixture
        .store
        .list_project_open_commerce_capability_source_links(&fixture.project_id)
        .unwrap()
        .is_empty());
}

#[test]
fn source_mutation_and_audit_are_atomic_when_audit_insert_fails() {
    let fixture = fixture("atomic");
    fixture
        .store
        .conn()
        .unwrap()
        .execute_batch(
            "CREATE TRIGGER reject_source_link_audit
             BEFORE INSERT ON open_commerce_audit_events
             WHEN NEW.action = 'capability.source_linked'
             BEGIN SELECT RAISE(ABORT, 'audit blocked'); END;",
        )
        .unwrap();
    assert!(fixture
        .link(&fixture.succeeded_receipt_id, "catalog")
        .unwrap_err()
        .to_string()
        .contains("audit blocked"));
    assert!(fixture
        .store
        .list_project_open_commerce_capability_source_links(&fixture.project_id)
        .unwrap()
        .is_empty());

    fixture
        .store
        .conn()
        .unwrap()
        .execute_batch(
            "DROP TRIGGER reject_source_link_audit;
             CREATE TRIGGER reject_source_unlink_audit
             BEFORE INSERT ON open_commerce_audit_events
             WHEN NEW.action = 'capability.source_unlinked'
             BEGIN SELECT RAISE(ABORT, 'unlink audit blocked'); END;",
        )
        .unwrap();
    fixture
        .link(&fixture.succeeded_receipt_id, "catalog")
        .unwrap();
    let error = remove_source(
        &fixture.store,
        &fixture.project_id,
        &fixture.capability_id,
        &fixture.owner(),
    )
    .unwrap_err();
    assert!(error.to_string().contains("unlink audit blocked"));
    assert_eq!(
        fixture
            .store
            .list_project_open_commerce_capability_source_links(&fixture.project_id)
            .unwrap()
            .len(),
        1
    );
}

#[test]
fn migration_v164_is_idempotent_and_keeps_expected_constraints() {
    let connection = rusqlite::Connection::open_in_memory().unwrap();
    migration_v164(&connection).unwrap();
    migration_v164(&connection).unwrap();
    let table_count: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master
              WHERE type = 'table' AND name = 'open_commerce_capability_source_links'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(table_count, 1);
    let indexes = connection
        .prepare("PRAGMA index_list(open_commerce_capability_source_links)")
        .unwrap()
        .query_map([], |row| row.get::<_, String>(1))
        .unwrap()
        .collect::<rusqlite::Result<Vec<_>>>()
        .unwrap();
    assert!(indexes
        .iter()
        .any(|name| name == "idx_open_commerce_capability_source_project"));
    assert!(indexes
        .iter()
        .any(|name| name == "idx_open_commerce_capability_source_receipt"));
    let foreign_keys: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM pragma_foreign_key_list('open_commerce_capability_source_links')",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(foreign_keys, 5);
}
