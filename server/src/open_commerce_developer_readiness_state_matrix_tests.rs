use chrono::{Duration, Utc};

use crate::{
    open_commerce_developer_model::OpenCommerceDeveloperApp,
    open_commerce_developer_production_test_support::{
        approved_developer_fixture, issue_local_credential,
    },
    open_commerce_developer_readiness_model::DeveloperProductionReadinessSummary,
    open_commerce_developer_readiness_service,
    store::Store,
};

#[test]
fn readiness_orders_manifest_domain_and_admission_state_blockers() {
    let fixture = approved_developer_fixture();

    let mut manifest_rejected = fixture.app.clone();
    manifest_rejected.manifest_status = "changes_requested".to_string();
    assert_blocker(
        &readiness(&fixture.store, &manifest_rejected),
        "manifest",
        "manifest_not_approved",
    );

    let mut domain_failed = fixture.app.clone();
    domain_failed.domain_verification_status = "failed".to_string();
    assert_blocker(
        &readiness(&fixture.store, &domain_failed),
        "domain",
        "domain_not_verified_for_current_revision",
    );

    let mut stale_domain = fixture.app.clone();
    stale_domain.domain_verification_revision = Some(fixture.app.manifest_revision - 1);
    assert_blocker(
        &readiness(&fixture.store, &stale_domain),
        "domain",
        "domain_not_verified_for_current_revision",
    );

    fixture
        .store
        .conn()
        .unwrap()
        .execute(
            "UPDATE open_commerce_developer_app_admissions
                SET manifest_revision=?1, status='approved'
              WHERE id=?2",
            rusqlite::params![fixture.app.manifest_revision - 1, fixture.admission_id],
        )
        .unwrap();
    let stale_admission = readiness(&fixture.store, &fixture.app);
    assert_eq!(
        stale_admission.admission_status.as_deref(),
        Some("approved")
    );
    assert_blocker(
        &stale_admission,
        "admission",
        "admission_not_approved_for_current_revision",
    );
}

#[test]
fn readiness_rejects_every_nonapproved_manifest_and_admission_status() {
    let fixture = approved_developer_fixture();

    for status in ["draft", "submitted", "changes_requested"] {
        let mut app = fixture.app.clone();
        app.manifest_status = status.to_string();
        let summary = readiness(&fixture.store, &app);
        assert_blocker(&summary, "manifest", "manifest_not_approved");
    }

    for status in ["submitted", "changes_requested", "suspended"] {
        fixture
            .store
            .conn()
            .unwrap()
            .execute(
                "UPDATE open_commerce_developer_app_admissions
                    SET manifest_revision=?1, status=?2
                  WHERE id=?3",
                rusqlite::params![fixture.app.manifest_revision, status, fixture.admission_id],
            )
            .unwrap();
        let summary = readiness(&fixture.store, &fixture.app);
        assert_eq!(summary.admission_status.as_deref(), Some(status));
        assert_blocker(
            &summary,
            "admission",
            "admission_not_approved_for_current_revision",
        );
    }
}

#[test]
fn readiness_treats_expired_and_revoked_credentials_as_missing() {
    let fixture = approved_developer_fixture();
    let expired = fixture
        .store
        .issue_open_commerce_developer_production_credential(
            &fixture.app,
            &fixture.admission_id,
            &["menu.preview".to_string()],
            "reviewer-user",
            &(Utc::now() - Duration::minutes(1)).to_rfc3339(),
        )
        .unwrap();
    let expired_summary = readiness(&fixture.store, &fixture.app);
    assert!(!expired_summary.current_production_credential_present);
    assert_blocker(
        &expired_summary,
        "credential",
        "current_production_credential_missing",
    );

    let current = issue_local_credential(&fixture);
    let current_summary = readiness(&fixture.store, &fixture.app);
    assert!(current_summary.current_production_credential_present);
    assert_eq!(
        current_summary.next_action_code,
        Some("active_production_webhook_missing")
    );
    fixture
        .store
        .revoke_open_commerce_developer_production_credential(
            &fixture.project_id,
            &fixture.app.id,
            &current.credential.id,
            "readiness_test_revocation",
        )
        .unwrap();
    let revoked_summary = readiness(&fixture.store, &fixture.app);
    assert!(!revoked_summary.current_production_credential_present);
    assert_blocker(
        &revoked_summary,
        "credential",
        "current_production_credential_missing",
    );

    let expired_record = fixture
        .store
        .list_open_commerce_developer_production_credentials(
            &fixture.project_id,
            &fixture.app.id,
            20,
        )
        .unwrap()
        .into_iter()
        .find(|item| item.id == expired.credential.id)
        .unwrap();
    assert_eq!(expired_record.status, "revoked");
}

#[test]
fn readiness_requires_a_verified_active_production_webhook() {
    let fixture = approved_developer_fixture();
    issue_local_credential(&fixture);
    let pending = fixture
        .store
        .create_open_commerce_developer_webhook(
            &fixture.app,
            "https://callback.example.test/readiness-matrix",
            "readiness-matrix-signing-key",
            "production",
            true,
            true,
        )
        .unwrap();
    let pending_summary = readiness(&fixture.store, &fixture.app);
    assert_eq!(pending_summary.active_production_webhook_count, 0);
    assert_blocker(
        &pending_summary,
        "webhook",
        "active_production_webhook_missing",
    );

    fixture
        .store
        .verify_open_commerce_developer_webhook(&fixture.project_id, &fixture.app.id, &pending.id)
        .unwrap();
    let verified_summary = readiness(&fixture.store, &fixture.app);
    assert!(verified_summary.production_invocation_ready);
    assert!(verified_summary.production_webhook_ready);
    assert_eq!(verified_summary.active_production_webhook_count, 1);
    assert!(verified_summary.blocker_codes.is_empty());

    fixture
        .store
        .set_open_commerce_developer_webhook_enabled(
            &fixture.project_id,
            &fixture.app.id,
            &pending.id,
            false,
        )
        .unwrap();
    let disabled_summary = readiness(&fixture.store, &fixture.app);
    assert!(disabled_summary.production_invocation_ready);
    assert!(!disabled_summary.production_webhook_ready);
    assert_eq!(disabled_summary.active_production_webhook_count, 0);
    assert_blocker(
        &disabled_summary,
        "webhook",
        "active_production_webhook_missing",
    );
}

fn readiness(store: &Store, app: &OpenCommerceDeveloperApp) -> DeveloperProductionReadinessSummary {
    open_commerce_developer_readiness_service::readiness_summary_with_feature_switches(
        store, app, true, true,
    )
    .unwrap()
}

fn assert_blocker(
    summary: &DeveloperProductionReadinessSummary,
    step_code: &str,
    blocker_code: &'static str,
) {
    assert_eq!(summary.next_action_code, Some(blocker_code));
    let step = summary
        .steps
        .iter()
        .find(|step| step.code == step_code)
        .unwrap();
    assert!(!step.ready);
    assert_eq!(step.blocker_code, Some(blocker_code));
}
