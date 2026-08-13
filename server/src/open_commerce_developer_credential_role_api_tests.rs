use chrono::{Duration, Utc};

use crate::open_commerce_developer_production_test_support::approved_developer_app_for;

use super::*;

#[tokio::test]
async fn app_owner_editor_and_project_admin_can_manage_production_credentials() {
    let fixture = approved_developer_fixture();
    let app_owner = user(&fixture.store, "credential-app-owner@example.com", None);
    let project_admin = user(&fixture.store, "credential-project-admin@example.com", None);
    for (account, role) in [
        ("credential-app-owner@example.com", "editor"),
        ("credential-project-admin@example.com", "admin"),
    ] {
        fixture
            .store
            .add_project_member_by_account(&fixture.project_id, account, role)
            .unwrap();
    }

    let editor_app = approved_developer_app_for(
        &fixture.store,
        &fixture.project_id,
        &app_owner.id,
        "consumer.production.editor-owned",
        &["menu.preview"],
    );
    let editor_credential = issue_for_app(&fixture.store, &editor_app);
    let admin_managed_credential = issue_local_credential(&fixture);
    let app_owner_token = session(&fixture.store, &app_owner.id);
    let project_admin_token = session(&fixture.store, &project_admin.id);
    let root = std::env::temp_dir();
    let state = Arc::new(test_app_state(fixture.store, &root));
    let router = routes().with_state(state);

    assert_can_list_and_revoke(
        &router,
        &fixture.project_id,
        &editor_app.app.id,
        &editor_credential.credential.id,
        &editor_credential.live_token,
        &app_owner_token,
        "App 所有者 editor 主动撤销生产凭据",
    )
    .await;
    assert_can_list_and_revoke(
        &router,
        &fixture.project_id,
        &fixture.app.id,
        &admin_managed_credential.credential.id,
        &admin_managed_credential.live_token,
        &project_admin_token,
        "项目 admin 应急撤销生产凭据",
    )
    .await;
}

fn issue_for_app(
    store: &Store,
    app: &crate::open_commerce_developer_production_test_support::ApprovedDeveloperAppFixture,
) -> crate::open_commerce_developer_credential_model::DeveloperProductionCredentialSecret {
    store
        .issue_open_commerce_developer_production_credential(
            &app.app,
            &app.admission_id,
            &["menu.preview".to_string()],
            "reviewer-user",
            &(Utc::now() + Duration::days(30)).to_rfc3339(),
        )
        .unwrap()
}

async fn assert_can_list_and_revoke(
    router: &Router,
    project_id: &str,
    app_record_id: &str,
    credential_id: &str,
    live_token: &str,
    session_token: &str,
    reason: &str,
) {
    let credentials_path = format!(
        "/api/projects/{project_id}/open-commerce/developer-apps/{app_record_id}/production-credentials"
    );
    let (list_status, listed) = call(
        router,
        Method::GET,
        &credentials_path,
        Some(session_token),
        None,
    )
    .await;
    assert_eq!(list_status, StatusCode::OK, "{listed}");
    assert_eq!(listed["credentials"][0]["id"], credential_id);
    assert!(!listed.to_string().contains(live_token));

    let (revoke_status, revoked) = call(
        router,
        Method::POST,
        &format!("{credentials_path}/{credential_id}/revoke"),
        Some(session_token),
        Some(&json!({"reason":reason})),
    )
    .await;
    assert_eq!(revoke_status, StatusCode::OK, "{revoked}");
    assert_eq!(revoked["status"], "revoked");
    assert_eq!(revoked["revocation_reason"], reason);
    assert!(!revoked.to_string().contains(live_token));
}
