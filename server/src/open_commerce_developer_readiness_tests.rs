#[path = "open_commerce_developer_readiness_api_tests.rs"]
mod api_tests;
#[path = "open_commerce_developer_readiness_state_matrix_tests.rs"]
mod state_matrix_tests;

use uuid::Uuid;

use crate::{
    open_commerce_developer_admission_model::ReviewDeveloperAppAdmissionRequest,
    open_commerce_developer_admission_service,
    open_commerce_developer_model::{CreateDeveloperAppRequest, OpenCommerceDeveloperApp},
    open_commerce_developer_production_test_support::{
        approved_developer_fixture, issue_local_credential,
    },
    open_commerce_developer_readiness_model::DeveloperProductionReadinessSummary,
    open_commerce_developer_readiness_service,
    store::Store,
};

#[test]
fn new_app_reports_all_blockers_in_dependency_order() {
    let (store, app) = new_app();
    let summary =
        open_commerce_developer_readiness_service::readiness_summary_with_feature_switches(
            &store, &app, false, false,
        )
        .unwrap();

    assert_eq!(
        step_codes(&summary),
        vec![
            "app",
            "manifest",
            "domain",
            "admission",
            "credential_gateway",
            "credential",
            "webhook_gateway",
            "webhook",
        ]
    );
    assert_eq!(
        summary.blocker_codes,
        vec![
            "manifest_not_approved",
            "domain_not_verified_for_current_revision",
            "admission_not_approved_for_current_revision",
            "production_credentials_disabled",
            "current_production_credential_missing",
            "production_webhooks_disabled",
            "active_production_webhook_missing",
        ]
    );
    assert_eq!(summary.next_action_code, Some("manifest_not_approved"));
    assert!(!summary.production_invocation_ready);
    assert!(!summary.production_webhook_ready);
    assert_eq!(summary.active_production_webhook_count, 0);
}

#[test]
fn readiness_separates_invocation_and_webhook_then_fails_closed_on_suspension() {
    let fixture = approved_developer_fixture();
    issue_local_credential(&fixture);
    let webhook = fixture
        .store
        .create_open_commerce_developer_webhook(
            &fixture.app,
            "https://callback.example.test/readiness",
            "local-readiness-signing-key",
            "production",
            true,
            true,
        )
        .unwrap();
    fixture
        .store
        .verify_open_commerce_developer_webhook(&fixture.project_id, &fixture.app.id, &webhook.id)
        .unwrap();

    let closed = readiness(&fixture.store, &fixture.app, false, false);
    assert_eq!(
        closed.blocker_codes,
        vec![
            "production_credentials_disabled",
            "production_webhooks_disabled"
        ]
    );
    assert_eq!(
        closed.next_action_code,
        Some("production_credentials_disabled")
    );
    assert!(closed.current_production_credential_present);
    assert_eq!(closed.active_production_webhook_count, 1);
    assert!(!closed.production_invocation_ready);
    assert!(!closed.production_webhook_ready);

    let invocation_only = readiness(&fixture.store, &fixture.app, true, false);
    assert!(invocation_only.production_invocation_ready);
    assert!(!invocation_only.production_webhook_ready);
    assert_eq!(
        invocation_only.blocker_codes,
        vec!["production_webhooks_disabled"]
    );
    assert_eq!(
        invocation_only.next_action_code,
        Some("production_webhooks_disabled")
    );

    let ready = readiness(&fixture.store, &fixture.app, true, true);
    assert!(ready.production_invocation_ready);
    assert!(ready.production_webhook_ready);
    assert!(ready.blocker_codes.is_empty());
    assert_eq!(ready.next_action_code, None);
    assert!(ready.steps.iter().all(|step| step.ready));

    open_commerce_developer_admission_service::review_admission(
        &fixture.store,
        &fixture.app.id,
        ReviewDeveloperAppAdmissionRequest {
            expected_manifest_revision: fixture.app.manifest_revision,
            decision: "suspended".to_string(),
            risk_tier: String::new(),
            note: "readiness suspension".to_string(),
        },
        "reviewer-user",
    )
    .unwrap();
    let suspended = readiness(&fixture.store, &fixture.app, true, true);
    assert_eq!(suspended.admission_status.as_deref(), Some("suspended"));
    assert_eq!(
        suspended.blocker_codes,
        vec![
            "admission_not_approved_for_current_revision",
            "current_production_credential_missing",
            "active_production_webhook_missing",
        ]
    );
    assert_eq!(
        suspended.next_action_code,
        Some("admission_not_approved_for_current_revision")
    );
    assert!(!suspended.current_production_credential_present);
    assert_eq!(suspended.active_production_webhook_count, 0);
    assert!(!suspended.production_invocation_ready);
    assert!(!suspended.production_webhook_ready);
}

fn readiness(
    store: &Store,
    app: &OpenCommerceDeveloperApp,
    credentials_enabled: bool,
    webhooks_enabled: bool,
) -> DeveloperProductionReadinessSummary {
    open_commerce_developer_readiness_service::readiness_summary_with_feature_switches(
        store,
        app,
        credentials_enabled,
        webhooks_enabled,
    )
    .unwrap()
}

fn step_codes(summary: &DeveloperProductionReadinessSummary) -> Vec<&str> {
    summary.steps.iter().map(|step| step.code).collect()
}

fn new_app() -> (Store, OpenCommerceDeveloperApp) {
    let path = std::env::temp_dir().join(format!(
        "elon_open_commerce_readiness_{}.db",
        Uuid::new_v4().simple()
    ));
    let store = Store::open(&path).expect("readiness test database should open");
    let owner = store
        .create_user("readiness@example.com", "secret1", Some("Readiness"), None)
        .unwrap();
    let project = store
        .create_project(&owner.id, "Readiness", None, None)
        .unwrap()
        .project;
    let app = store
        .create_open_commerce_developer_app(
            &project.id,
            &owner.id,
            CreateDeveloperAppRequest {
                app_id: "consumer.readiness".to_string(),
                display_name: "Readiness Consumer".to_string(),
            },
        )
        .unwrap()
        .app;
    (store, app)
}
