use super::*;

pub(super) async fn assert_second_production_app_isolated(
    client: &reqwest::Client,
    base_url: &str,
    second_live_token: &str,
    first_app_cursor: &str,
    first_app_invocation_ids: &[&str],
) {
    let second_events = developer_get(
        client,
        &format!("{base_url}/api/open-commerce/developer/events"),
        second_live_token,
    )
    .await;
    assert_eq!(second_events["app_id"], SECOND_PRODUCTION_APP_ID);
    assert_eq!(second_events["credential_environment"], "production");
    assert!(second_events["events"].as_array().unwrap().is_empty());
    assert_developer_event_redacted(&second_events, second_live_token);

    let (cursor_status, cursor_body) = developer_get_response(
        client,
        &format!("{base_url}/api/open-commerce/developer/events?cursor={first_app_cursor}"),
        second_live_token,
    )
    .await;
    assert_eq!(cursor_status, StatusCode::BAD_REQUEST);
    assert_developer_event_redacted(&cursor_body, second_live_token);

    for invocation_id in first_app_invocation_ids {
        let (detail_status, detail_body) = developer_get_response(
            client,
            &format!("{base_url}/api/open-commerce/developer/events/{invocation_id}"),
            second_live_token,
        )
        .await;
        assert_eq!(detail_status, StatusCode::NOT_FOUND);
        assert_developer_event_redacted(&detail_body, second_live_token);
    }
}
