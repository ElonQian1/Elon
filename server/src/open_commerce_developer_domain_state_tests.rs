use chrono::{Duration, Utc};

use crate::{
    open_commerce_developer_domain_service, open_commerce_developer_manifest_service,
    open_commerce_developer_model::UpdateDeveloperAppManifestRequest,
    open_commerce_developer_production_test_support::approved_developer_fixture,
    open_commerce_service::OpenCommerceActor,
};

#[tokio::test]
async fn expired_domain_challenge_fails_before_any_network_request() {
    let fixture = approved_developer_fixture();
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
            homepage_url: Some("https://expired.example.test/app".to_string()),
            privacy_policy_url: Some("https://expired.example.test/privacy".to_string()),
            terms_url: Some("https://expired.example.test/terms".to_string()),
            support_email: Some("support@expired.example.test".to_string()),
            requested_scopes: vec!["menu.preview".to_string()],
        },
        &actor,
    )
    .unwrap();
    fixture
        .store
        .issue_open_commerce_developer_app_domain_challenge(
            &fixture.project_id,
            &revised.id,
            revised.manifest_revision,
            "expired.example.test",
            "expired-challenge-hash",
            &(Utc::now() - Duration::minutes(1)).to_rfc3339(),
        )
        .unwrap();

    let error = open_commerce_developer_domain_service::verify_domain(
        &fixture.store,
        &fixture.project_id,
        &revised.id,
        &actor,
    )
    .await
    .unwrap_err();
    assert!(error.to_string().contains("challenge 已过期"), "{error}");

    let challenge = fixture
        .store
        .open_commerce_developer_app_domain_challenge(&fixture.project_id, &revised.id)
        .unwrap();
    assert_eq!(challenge.status, "pending");
    let app = fixture
        .store
        .open_commerce_developer_app_for_project(&fixture.project_id, &revised.id)
        .unwrap();
    assert!(app.domain_verification_attempted_at.is_none());
}
