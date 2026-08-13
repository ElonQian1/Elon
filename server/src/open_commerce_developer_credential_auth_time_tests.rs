use std::process::Command;

use chrono::{Duration, FixedOffset, Utc};
use rusqlite::params;

use crate::{
    open_commerce_developer_credential_model::{
        production_credentials_enabled, PRODUCTION_CREDENTIAL_ENV,
    },
    open_commerce_developer_production_test_support::{
        approved_developer_fixture, issue_local_credential,
    },
};

const CHILD_ENV: &str = "ELON_TEST_PRODUCTION_CREDENTIAL_AUTH_TIME_CHILD";
const CHILD_TEST: &str = "open_commerce_developer_production_state_tests::credential_auth_time_tests::production_credential_auth_format_and_time_child";

#[test]
fn production_credential_auth_format_and_time_in_isolated_process() {
    let output = Command::new(std::env::current_exe().unwrap())
        .args(["--exact", CHILD_TEST, "--nocapture"])
        .env(CHILD_ENV, "1")
        .env(PRODUCTION_CREDENTIAL_ENV, "1")
        .output()
        .expect("launch production credential auth-time test");
    assert!(
        output.status.success(),
        "credential auth-time child failed: stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn production_credential_auth_format_and_time_child() {
    if std::env::var(CHILD_ENV).as_deref() != Ok("1") {
        return;
    }
    assert!(production_credentials_enabled());
    let fixture = approved_developer_fixture();
    let secret = issue_local_credential(&fixture);

    for token in [
        "",
        "oc_live_short",
        "not_live_0123456789012345678901234567890123456789",
    ] {
        let error = fixture
            .store
            .authenticate_open_commerce_developer_credential(token)
            .unwrap_err();
        assert!(error.to_string().contains("生产开发者凭据无效"));
    }
    let unknown = format!("oc_live_{}", "0".repeat(64));
    let error = fixture
        .store
        .authenticate_open_commerce_developer_credential(&unknown)
        .unwrap_err();
    assert!(error.to_string().contains("无效或已撤销"));

    let offset = FixedOffset::east_opt(8 * 60 * 60).unwrap();
    let future = (Utc::now() + Duration::minutes(10))
        .with_timezone(&offset)
        .to_rfc3339();
    set_auth_state(&fixture, &secret.credential.id, &future);
    let authenticated = fixture
        .store
        .authenticate_open_commerce_developer_credential(&format!("  {}  ", secret.live_token))
        .unwrap();
    assert_eq!(authenticated.environment, "production");
    assert_eq!(
        authenticated.credential_id.as_deref(),
        Some(secret.credential.id.as_str())
    );
    assert_eq!(authenticated.scopes, Some(vec!["menu.preview".to_string()]));
    let used_at = credential(&fixture, &secret.credential.id)
        .last_used_at
        .expect("successful authentication records last_used_at");
    assert!(chrono::DateTime::parse_from_rfc3339(&used_at).is_ok());

    let expired = (Utc::now() - Duration::seconds(1)).to_rfc3339();
    set_auth_state(&fixture, &secret.credential.id, &expired);
    let error = fixture
        .store
        .authenticate_open_commerce_developer_credential(&secret.live_token)
        .unwrap_err();
    assert!(error.to_string().contains("已到期"));
    assert!(credential(&fixture, &secret.credential.id)
        .last_used_at
        .is_none());

    set_auth_state(&fixture, &secret.credential.id, "not-rfc3339");
    let error = fixture
        .store
        .authenticate_open_commerce_developer_credential(&secret.live_token)
        .unwrap_err();
    assert!(error.to_string().contains("到期时间无效"));
    assert!(credential(&fixture, &secret.credential.id)
        .last_used_at
        .is_none());
}

fn set_auth_state(
    fixture: &crate::open_commerce_developer_production_test_support::ApprovedDeveloperFixture,
    credential_id: &str,
    expires_at: &str,
) {
    let conn = fixture.store.conn().unwrap();
    let changed = conn
        .execute(
            "UPDATE open_commerce_developer_production_credentials SET expires_at=?1, last_used_at=NULL WHERE id=?2",
            params![expires_at, credential_id],
        )
        .unwrap();
    assert_eq!(changed, 1);
}

fn credential(
    fixture: &crate::open_commerce_developer_production_test_support::ApprovedDeveloperFixture,
    credential_id: &str,
) -> crate::open_commerce_developer_credential_model::DeveloperProductionCredential {
    fixture
        .store
        .list_open_commerce_developer_production_credentials(
            &fixture.project_id,
            &fixture.app.id,
            20,
        )
        .unwrap()
        .into_iter()
        .find(|credential| credential.id == credential_id)
        .unwrap()
}
