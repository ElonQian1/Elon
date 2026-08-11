use super::*;

#[tokio::test]
async fn owner_can_list_preflight_and_idempotently_cancel_only_owned_request() {
    let fixture = fixture();
    let (_, submitted) = call(
        &fixture.router,
        Method::POST,
        owner_path(),
        Some(&fixture.owner_token),
        &submit_body("management-cancel", true),
    )
    .await;
    let request_id = submitted["request_id"].as_str().unwrap();
    let detail_path = format!("{}/{request_id}", owner_path());
    let preflight_path = format!("{detail_path}/preflight");
    let cancel_path = format!("{detail_path}/cancel");

    assert_eq!(
        call(
            &fixture.router,
            Method::GET,
            owner_path(),
            None,
            &Value::Null,
        )
        .await
        .0,
        StatusCode::UNAUTHORIZED
    );
    let (status, listed) = call(
        &fixture.router,
        Method::GET,
        &format!("{}?status=submitted&limit=10", owner_path()),
        Some(&fixture.owner_token),
        &Value::Null,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{listed}");
    assert_eq!(
        listed["onboarding_requests"][0]["request"]["request_id"],
        request_id
    );
    assert_eq!(
        listed["onboarding_requests"][0]["request"]["credential_ref_present"],
        true
    );
    assert!(!listed.to_string().contains("vault-ref:"));

    let (_, member_list) = call(
        &fixture.router,
        Method::GET,
        owner_path(),
        Some(&fixture.member_token),
        &Value::Null,
    )
    .await;
    assert_eq!(
        member_list["onboarding_requests"].as_array().unwrap().len(),
        0
    );
    assert_eq!(
        call(
            &fixture.router,
            Method::GET,
            &detail_path,
            Some(&fixture.member_token),
            &Value::Null,
        )
        .await
        .0,
        StatusCode::BAD_REQUEST
    );

    let (status, preflight) = call(
        &fixture.router,
        Method::GET,
        &preflight_path,
        Some(&fixture.owner_token),
        &Value::Null,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{preflight}");
    assert_eq!(preflight["owner_cancel_allowed"], true);
    assert_eq!(preflight["admin_review_allowed"], true);
    assert_eq!(preflight["admin_apply_allowed"], false);

    let cancel = json!({
        "expected_request_digest": submitted["request_digest"],
        "confirm_cancel": false
    });
    assert_eq!(
        call(
            &fixture.router,
            Method::POST,
            &cancel_path,
            Some(&fixture.owner_token),
            &cancel,
        )
        .await
        .0,
        StatusCode::BAD_REQUEST
    );
    let mut cancel = cancel;
    cancel["confirm_cancel"] = json!(true);
    let (status, canceled) = call(
        &fixture.router,
        Method::POST,
        &cancel_path,
        Some(&fixture.owner_token),
        &cancel,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{canceled}");
    assert_eq!(canceled["status"], "canceled");
    assert_eq!(canceled["replayed"], false);
    assert!(canceled["canceled_at"].is_string());

    let (_, replayed) = call(
        &fixture.router,
        Method::POST,
        &cancel_path,
        Some(&fixture.owner_token),
        &cancel,
    )
    .await;
    assert_eq!(replayed["status"], "canceled");
    assert_eq!(replayed["replayed"], true);

    let (_, blocked) = call(
        &fixture.router,
        Method::GET,
        &preflight_path,
        Some(&fixture.owner_token),
        &Value::Null,
    )
    .await;
    assert_eq!(blocked["owner_cancel_allowed"], false);
    assert_eq!(blocked["blockers"][0], "request_canceled");

    assert_eq!(
        call(
            &fixture.router,
            Method::GET,
            &format!("{}?status=canceled", admin_path()),
            Some(&fixture.member_token),
            &Value::Null,
        )
        .await
        .0,
        StatusCode::FORBIDDEN
    );
    let (_, admin_list) = call(
        &fixture.router,
        Method::GET,
        &format!("{}?status=canceled", admin_path()),
        Some(&fixture.reviewer_token),
        &Value::Null,
    )
    .await;
    assert_eq!(
        admin_list["onboarding_requests"][0]["request"]["request_id"],
        request_id
    );
    fixture.cleanup();
}

#[tokio::test]
async fn admin_detail_and_preflight_follow_review_and_application_history() {
    let fixture = fixture();
    let (_, submitted) = call(
        &fixture.router,
        Method::POST,
        owner_path(),
        Some(&fixture.owner_token),
        &submit_body("management-apply", true),
    )
    .await;
    let request_id = submitted["request_id"].as_str().unwrap();
    let admin_detail = format!("{}/{request_id}", admin_path());
    let admin_preflight = format!("{admin_detail}/preflight");
    let review_path = format!("{admin_detail}/review");
    let application_path = format!("{admin_detail}/application");

    let (_, submitted_preflight) = call(
        &fixture.router,
        Method::GET,
        &admin_preflight,
        Some(&fixture.reviewer_token),
        &Value::Null,
    )
    .await;
    assert_eq!(submitted_preflight["admin_review_allowed"], true);

    let (_, reviewed) = call(
        &fixture.router,
        Method::POST,
        &review_path,
        Some(&fixture.reviewer_token),
        &review_body(&submitted, "management-review", "approved", true),
    )
    .await;
    let (_, approved_preflight) = call(
        &fixture.router,
        Method::GET,
        &admin_preflight,
        Some(&fixture.applier_token),
        &Value::Null,
    )
    .await;
    assert_eq!(approved_preflight["review_present"], true);
    assert_eq!(approved_preflight["admin_apply_allowed"], true);

    let (_, applied) = call(
        &fixture.router,
        Method::POST,
        &application_path,
        Some(&fixture.applier_token),
        &application_body(&submitted, &reviewed, "management-application", true),
    )
    .await;
    assert_eq!(applied["onboarding_effect"], "provider_registered_only");

    let (status, detail) = call(
        &fixture.router,
        Method::GET,
        &admin_detail,
        Some(&fixture.reviewer_token),
        &Value::Null,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{detail}");
    assert_eq!(detail["request"]["status"], "applied");
    assert_eq!(detail["review"]["decision"], "approved");
    assert_eq!(
        detail["application"]["provider_id"],
        submitted["provider_id"]
    );
    assert!(!detail.to_string().contains("vault-ref:"));

    let (_, applied_preflight) = call(
        &fixture.router,
        Method::GET,
        &admin_preflight,
        Some(&fixture.applier_token),
        &Value::Null,
    )
    .await;
    assert_eq!(applied_preflight["application_present"], true);
    assert_eq!(applied_preflight["admin_apply_allowed"], false);
    assert_eq!(
        applied_preflight["blockers"][0],
        "provider_already_registered"
    );
    assert_eq!(
        call(
            &fixture.router,
            Method::GET,
            &format!("{}?status=unknown", admin_path()),
            Some(&fixture.reviewer_token),
            &Value::Null,
        )
        .await
        .0,
        StatusCode::BAD_REQUEST
    );
    fixture.cleanup();
}
