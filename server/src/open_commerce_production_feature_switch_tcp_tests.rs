use super::*;

pub(super) fn owner_session(store: &crate::store::Store, owner_user_id: &str) -> String {
    store
        .create_session(owner_user_id, Some("production-runtime-switch-test"), None)
        .unwrap()
        .0
}

pub(super) async fn assert_production_switch_fails_closed_and_revoke_remains_available(
    client: &reqwest::Client,
    base_url: &str,
    live_token: &str,
    credential_id: &str,
    project_id: &str,
    app_record_id: &str,
    owner_session: &str,
    query: &Value,
    runtime_state: &RuntimeState,
) {
    std::env::remove_var(PRODUCTION_CREDENTIAL_ENV);
    assert!(!production_credentials_enabled());

    let (disabled_invoke_status, disabled_invoke) = post_response(
        client,
        &format!("{base_url}/api/open-commerce/developer/invoke"),
        live_token,
        query,
    )
    .await;
    assert_eq!(disabled_invoke_status, StatusCode::UNAUTHORIZED);
    assert!(disabled_invoke.to_string().contains("当前未启用"));
    assert_developer_event_redacted(&disabled_invoke, live_token);

    let (disabled_events_status, disabled_events) = developer_get_response(
        client,
        &format!("{base_url}/api/open-commerce/developer/events"),
        live_token,
    )
    .await;
    assert_eq!(disabled_events_status, StatusCode::UNAUTHORIZED);
    assert!(disabled_events.to_string().contains("当前未启用"));
    assert_developer_event_redacted(&disabled_events, live_token);
    assert_eq!(runtime_state.invocation_count.load(Ordering::SeqCst), 3);

    let revoked = developer_post(
        client,
        &format!(
            "{base_url}/api/projects/{project_id}/open-commerce/developer-apps/{app_record_id}/production-credentials/{credential_id}/revoke"
        ),
        owner_session,
        &json!({"reason":"生产入口关闭后的应急撤销验证"}),
    )
    .await;
    assert_eq!(revoked["status"], "revoked");
    assert_eq!(revoked["revocation_reason"], "生产入口关闭后的应急撤销验证");
    assert_live_token_redacted(&revoked, live_token);

    let credentials = developer_get(
        client,
        &format!(
            "{base_url}/api/projects/{project_id}/open-commerce/developer-apps/{app_record_id}/production-credentials"
        ),
        owner_session,
    )
    .await;
    assert_eq!(credentials["issuance_enabled"], false);
    assert_eq!(credentials["credentials"][0]["status"], "revoked");
    assert_live_token_redacted(&credentials, live_token);
}

fn assert_live_token_redacted(value: &Value, live_token: &str) {
    assert!(
        !value.to_string().contains(live_token),
        "leaked live credential"
    );
}

async fn post_response(
    client: &reqwest::Client,
    url: &str,
    token: &str,
    body: &Value,
) -> (StatusCode, Value) {
    let response = client
        .post(url)
        .bearer_auth(token)
        .json(body)
        .send()
        .await
        .unwrap();
    let status = response.status();
    let body = response.json().await.unwrap();
    (status, body)
}
