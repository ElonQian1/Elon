#[path = "open_commerce_developer_admission_api_tests.rs"]
mod admission_api_tests;
#[path = "open_commerce_developer_credential_api_tests.rs"]
mod credential_api_tests;
#[path = "open_commerce_developer_credential_restart_tests.rs"]
mod credential_restart_tests;
#[path = "open_commerce_developer_domain_state_tests.rs"]
mod domain_state_tests;
#[path = "open_commerce_developer_manifest_api_tests.rs"]
mod manifest_api_tests;

use crate::{
    open_commerce_developer_admission_model::ReviewDeveloperAppAdmissionRequest,
    open_commerce_developer_admission_service, open_commerce_developer_manifest_service,
    open_commerce_developer_model::UpdateDeveloperAppManifestRequest,
    open_commerce_developer_production_test_support::{
        approved_developer_fixture, issue_local_credential,
    },
    open_commerce_service::OpenCommerceActor,
};

#[test]
fn manifest_revision_revokes_hashed_production_credential_and_admission() {
    let fixture = approved_developer_fixture();
    let secret = issue_local_credential(&fixture);

    assert!(secret.live_token.starts_with("oc_live_"));
    assert!(secret.token_visible_once);
    assert!(!secret.funds_moved);
    let token_hash: String = {
        let conn = fixture.store.conn().unwrap();
        conn.query_row(
            "SELECT token_hash FROM open_commerce_developer_production_credentials WHERE id=?1",
            [secret.credential.id.as_str()],
            |row| row.get(0),
        )
        .unwrap()
    };
    assert_eq!(token_hash.len(), 64);
    assert_ne!(token_hash, secret.live_token);
    let listed = fixture
        .store
        .list_open_commerce_developer_production_credentials(
            &fixture.project_id,
            &fixture.app.id,
            20,
        )
        .unwrap();
    assert!(!serde_json::to_string(&listed)
        .unwrap()
        .contains(&secret.live_token));
    assert!(fixture
        .store
        .has_current_open_commerce_production_credential(&fixture.project_id, &fixture.app.id,)
        .unwrap());

    let actor = OpenCommerceActor {
        user_id: &fixture.owner_user_id,
        app_id: "pc-web",
        project_role: Some("owner"),
    };
    let revised = open_commerce_developer_manifest_service::update_manifest(
        &fixture.store,
        &fixture.project_id,
        &fixture.app.id,
        UpdateDeveloperAppManifestRequest {
            expected_manifest_revision: fixture.app.manifest_revision,
            homepage_url: Some("https://shop.example.test/v2".to_string()),
            privacy_policy_url: Some("https://shop.example.test/privacy".to_string()),
            terms_url: Some("https://shop.example.test/terms".to_string()),
            support_email: Some("support@example.test".to_string()),
            requested_scopes: vec!["menu.preview".to_string()],
        },
        &actor,
    )
    .unwrap();

    assert_eq!(revised.manifest_revision, fixture.app.manifest_revision + 1);
    assert_eq!(revised.manifest_status, "draft");
    assert_eq!(revised.domain_verification_status, "pending");
    let admission = fixture
        .store
        .open_commerce_developer_app_admission(&fixture.app.id)
        .unwrap()
        .unwrap();
    assert_eq!(admission.status, "changes_requested");
    assert_eq!(
        admission.review_note.as_deref(),
        Some("manifest_revision_changed")
    );
    let credential = fixture
        .store
        .list_open_commerce_developer_production_credentials(
            &fixture.project_id,
            &fixture.app.id,
            20,
        )
        .unwrap()
        .into_iter()
        .find(|item| item.id == secret.credential.id)
        .unwrap();
    assert_eq!(credential.status, "revoked");
    assert_eq!(
        credential.revocation_reason.as_deref(),
        Some("manifest_revision_changed")
    );
    assert!(!fixture
        .store
        .has_current_open_commerce_production_credential(&fixture.project_id, &fixture.app.id,)
        .unwrap());
}

#[test]
fn credential_rotation_and_admission_suspension_disable_production_webhook() {
    let fixture = approved_developer_fixture();
    let first = issue_local_credential(&fixture);
    let second = issue_local_credential(&fixture);
    let credentials = fixture
        .store
        .list_open_commerce_developer_production_credentials(
            &fixture.project_id,
            &fixture.app.id,
            20,
        )
        .unwrap();
    let first = credentials
        .iter()
        .find(|item| item.id == first.credential.id)
        .unwrap();
    let second_credential = credentials
        .iter()
        .find(|item| item.id == second.credential.id)
        .unwrap();
    assert_eq!(first.status, "revoked");
    assert_eq!(
        first.revocation_reason.as_deref(),
        Some("credential_rotated")
    );
    assert_eq!(second_credential.status, "active");
    assert_eq!(
        credentials
            .iter()
            .filter(|credential| credential.status == "active")
            .count(),
        1
    );

    let webhook = fixture
        .store
        .create_open_commerce_developer_webhook(
            &fixture.app,
            "https://callback.example.test/open-commerce",
            "local-signing-key",
            "production",
            true,
            true,
        )
        .unwrap();
    // This is a local state transition, not proof that the callback is reachable.
    let webhook = fixture
        .store
        .verify_open_commerce_developer_webhook(&fixture.project_id, &fixture.app.id, &webhook.id)
        .unwrap();
    assert_eq!(webhook.status, "active");

    let admission = open_commerce_developer_admission_service::review_admission(
        &fixture.store,
        &fixture.app.id,
        ReviewDeveloperAppAdmissionRequest {
            expected_manifest_revision: fixture.app.manifest_revision,
            decision: "suspended".to_string(),
            risk_tier: String::new(),
            note: "local security suspension".to_string(),
        },
        "reviewer-user",
    )
    .unwrap();
    assert_eq!(admission.status, "suspended");

    let credential = fixture
        .store
        .list_open_commerce_developer_production_credentials(
            &fixture.project_id,
            &fixture.app.id,
            20,
        )
        .unwrap()
        .into_iter()
        .find(|item| item.id == second.credential.id)
        .unwrap();
    assert_eq!(credential.status, "revoked");
    assert_eq!(
        credential.revocation_reason.as_deref(),
        Some("admission_suspended")
    );
    let webhook = fixture
        .store
        .open_commerce_developer_webhook_for_app(&fixture.project_id, &fixture.app.id, &webhook.id)
        .unwrap();
    assert_eq!(webhook.status, "disabled");
    assert_eq!(
        webhook.last_error_code.as_deref(),
        Some("production_credential_revoked")
    );
}
