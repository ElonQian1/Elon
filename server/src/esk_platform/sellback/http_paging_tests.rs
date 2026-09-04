use super::*;

#[tokio::test]
async fn maximum_utf8_terms_and_twenty_record_page_fit_android_budget() {
    let fixture = Fixture::new();
    let _allocation_policy = enable_fixture_policy();
    credit(&fixture).await;
    let (_configuration, policy) = configure(&fixture);
    let mut body = policy.body;
    body.revision = "r".repeat(80);
    body.terms_text = format!("{}ab", "界".repeat(682));
    body.terms_digest = text_digest(&body.terms_text);
    body.disabled_account_recovery_text = format!("{}a", "界".repeat(341));
    body.max_open_requests_per_user = "30".into();
    let policy = validate_policy(body).unwrap();
    let _bounded = override_configuration(SellbackConfiguration::Enabled(policy.clone()));
    for number in 0..21 {
        let current = page(&fixture).await;
        let key = format!("{}{:02}", "k".repeat(94), number);
        let (status, _) = send(
            &fixture,
            "POST",
            BASE,
            Some(&fixture.user_token),
            submit_body(&current, &key, "1"),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
    }
    let first = page(&fixture).await;
    assert_base(&first, true);
    assert_eq!(first["requests"].as_array().unwrap().len(), 20);
    assert_eq!(first["range_start"], "1");
    assert_eq!(first["range_end"], "20");
    assert_eq!(first["has_more"], true);
    assert_eq!(
        first["summary"]["policy"]["terms_text"]
            .as_str()
            .unwrap()
            .len(),
        2048
    );
    assert_eq!(
        first["summary"]["policy"]["disabled_account_recovery_text"]
            .as_str()
            .unwrap()
            .len(),
        1024
    );
    assert!(serde_json::to_vec(&first).unwrap().len() <= 64 * 1024);
    let cursor = first["next_cursor"].as_str().unwrap();
    let path = format!("{BASE}?limit=20&cursor={cursor}");
    let (status, last) = send(
        &fixture,
        "GET",
        &path,
        Some(&fixture.user_token),
        Value::Null,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(last["summary"], first["summary"]);
    assert_eq!(last["range_start"], "21");
    assert_eq!(last["range_end"], "21");
    assert_eq!(last["has_more"], false);
    assert_eq!(last["next_cursor"], Value::Null);
    assert_eq!(last["requests"].as_array().unwrap().len(), 1);
    let id = last["requests"][0]["request_id"].as_str().unwrap();
    assert_eq!(
        send(
            &fixture,
            "POST",
            &format!("{BASE}/{id}/cancel"),
            Some(&fixture.user_token),
            cancel_body()
        )
        .await
        .0,
        StatusCode::OK
    );
    let (status, error) = send(
        &fixture,
        "GET",
        &path,
        Some(&fixture.user_token),
        Value::Null,
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(error["error"], "ESK_PLATFORM_SELLBACK_SNAPSHOT_CHANGED");
    fixture.cleanup();
}
