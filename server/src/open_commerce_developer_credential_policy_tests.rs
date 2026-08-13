use std::process::Command;

use rusqlite::params;

use crate::{
    open_commerce_developer_credential_model::{
        production_credentials_enabled, IssueDeveloperProductionCredentialRequest,
        PRODUCTION_CREDENTIAL_ENV,
    },
    open_commerce_developer_credential_service,
    open_commerce_developer_production_test_support::{
        approved_developer_app_for, approved_developer_fixture_for,
    },
};

const CHILD_ENV: &str = "ELON_TEST_PRODUCTION_CREDENTIAL_POLICY_CHILD";
const CHILD_TEST: &str = "open_commerce_developer_production_state_tests::credential_policy_tests::production_credential_risk_and_scope_policy_child";

#[test]
fn production_credential_risk_and_scope_policy_in_isolated_process() {
    let output = Command::new(std::env::current_exe().unwrap())
        .args(["--exact", CHILD_TEST, "--nocapture"])
        .env(CHILD_ENV, "1")
        .env(PRODUCTION_CREDENTIAL_ENV, "1")
        .output()
        .expect("launch production credential policy test");
    assert!(
        output.status.success(),
        "credential policy child failed: stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn production_credential_risk_and_scope_policy_child() {
    if std::env::var(CHILD_ENV).as_deref() != Ok("1") {
        return;
    }
    assert!(production_credentials_enabled());
    let fixture = approved_developer_fixture_for(
        "consumer.production.policy",
        &["menu.preview", "order.commit", "booking.preview"],
    );

    for (risk_tier, max_days) in [("low", 366), ("standard", 180), ("enhanced", 90)] {
        set_risk_tier(&fixture.store, &fixture.app.id, risk_tier);
        for days in [1, max_days] {
            let issued = issue(
                &fixture.store,
                &fixture.app.id,
                vec!["menu.preview".to_string()],
                days,
            )
            .unwrap();
            assert_eq!(issued.credential.scopes, vec!["menu.preview"]);
        }
        for days in [0, max_days + 1] {
            let error = issue(
                &fixture.store,
                &fixture.app.id,
                vec!["menu.preview".to_string()],
                days,
            )
            .unwrap_err();
            assert!(error.to_string().contains(&format!("1 至 {max_days} 天")));
        }
    }

    set_risk_tier(&fixture.store, &fixture.app.id, "standard");
    let normalized = issue(
        &fixture.store,
        &fixture.app.id,
        vec![
            " ORDER.COMMIT ".to_string(),
            "menu.preview".to_string(),
            "order.commit".to_string(),
        ],
        30,
    )
    .unwrap();
    assert_eq!(
        normalized.credential.scopes,
        vec!["menu.preview", "order.commit"]
    );

    for (scopes, expected) in [
        (Vec::new(), "至少需要一项"),
        (vec!["   ".to_string()], "至少需要一项"),
        (vec!["bad scope".to_string()], "能力键"),
        (vec!["inventory.write".to_string()], "不能超出"),
        (vec!["menu.preview".to_string(); 33], "最多包含 32 项"),
    ] {
        let error = issue(&fixture.store, &fixture.app.id, scopes, 30).unwrap_err();
        assert!(error.to_string().contains(expected), "{error:#}");
    }

    let all_scopes = (0..32)
        .map(|index| format!("catalog.scope{index:02}"))
        .collect::<Vec<_>>();
    let all_scope_refs = all_scopes.iter().map(String::as_str).collect::<Vec<_>>();
    let max_scope_app = approved_developer_app_for(
        &fixture.store,
        &fixture.project_id,
        &fixture.owner_user_id,
        "consumer.production.max-scopes",
        &all_scope_refs,
    );
    let issued = issue(
        &fixture.store,
        &max_scope_app.app.id,
        all_scopes.clone(),
        30,
    )
    .unwrap();
    assert_eq!(issued.credential.scopes, all_scopes);
}

fn set_risk_tier(store: &crate::store::Store, app_record_id: &str, risk_tier: &str) {
    let conn = store.conn().unwrap();
    let changed = conn
        .execute(
            "UPDATE open_commerce_developer_app_admissions SET risk_tier=?1 WHERE app_record_id=?2 AND status='approved'",
            params![risk_tier, app_record_id],
        )
        .unwrap();
    assert_eq!(changed, 1);
}

fn issue(
    store: &crate::store::Store,
    app_record_id: &str,
    scopes: Vec<String>,
    expires_in_days: i64,
) -> anyhow::Result<
    crate::open_commerce_developer_credential_model::DeveloperProductionCredentialSecret,
> {
    let app = store.open_commerce_developer_app_by_record_id(app_record_id)?;
    open_commerce_developer_credential_service::issue_credential(
        store,
        app_record_id,
        IssueDeveloperProductionCredentialRequest {
            expected_manifest_revision: app.manifest_revision,
            scopes,
            expires_in_days,
        },
        "policy-reviewer",
    )
}
