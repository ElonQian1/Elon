use super::*;

#[tokio::test]
async fn administrator_can_query_release_lifecycle_and_actor_aware_preflight() {
    let fixture = fixture();
    let empty = json!({});

    assert_eq!(
        call(&fixture.router, Method::GET, release_path(), None, &empty)
            .await
            .0,
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        call(
            &fixture.router,
            Method::GET,
            release_path(),
            Some(&fixture.member_token),
            &empty,
        )
        .await
        .0,
        StatusCode::FORBIDDEN
    );

    let (status, submitted) = call(
        &fixture.router,
        Method::POST,
        release_path(),
        Some(&fixture.submitter_token),
        &submit_body("management-submit", "3.0.0", true),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{submitted}");
    let request_id = submitted["request_id"].as_str().unwrap();
    let detail_path = format!("{}/{request_id}", release_path());
    let preflight_path = format!("{detail_path}/preflight");

    let (status, listed) = call(
        &fixture.router,
        Method::GET,
        &format!("{}?status=submitted&limit=1", release_path()),
        Some(&fixture.reviewer_token),
        &empty,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{listed}");
    let items = listed["adapter_release_requests"].as_array().unwrap();
    assert_eq!(items.len(), 1);
    assert_eq!(items[0]["request"]["request_id"], submitted["request_id"]);
    assert!(items[0]["review"].is_null());
    assert!(items[0]["admission"].is_null());

    let (status, detail) = call(
        &fixture.router,
        Method::GET,
        &detail_path,
        Some(&fixture.reviewer_token),
        &empty,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{detail}");
    assert_eq!(detail["request"]["status"], "submitted");
    assert!(detail["request"].get("candidate_artifact_ref").is_none());
    assert!(detail["request"]
        .get("expected_credential_verifier")
        .is_none());
    assert!(detail["request"]["updated_at"].as_str().is_some());

    let (_, submitter_preflight) = call(
        &fixture.router,
        Method::GET,
        &preflight_path,
        Some(&fixture.submitter_token),
        &empty,
    )
    .await;
    assert_eq!(submitter_preflight["admin_review_allowed"], false);
    assert_eq!(
        submitter_preflight["blockers"][0],
        "current_admin_cannot_review_own_submission"
    );
    let (_, reviewer_preflight) = call(
        &fixture.router,
        Method::GET,
        &preflight_path,
        Some(&fixture.reviewer_token),
        &empty,
    )
    .await;
    assert_eq!(reviewer_preflight["admin_review_allowed"], true);
    assert_eq!(reviewer_preflight["admin_stage_allowed"], false);

    let review_path = format!("{detail_path}/review");
    let (status, reviewed) = call(
        &fixture.router,
        Method::POST,
        &review_path,
        Some(&fixture.reviewer_token),
        &review_body(&submitted, "management-review", "approved", true),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{reviewed}");
    let (_, approved_preflight) = call(
        &fixture.router,
        Method::GET,
        &preflight_path,
        Some(&fixture.applier_token),
        &empty,
    )
    .await;
    assert_eq!(approved_preflight["request_status"], "approved");
    assert_eq!(approved_preflight["review_present"], true);
    assert_eq!(approved_preflight["admin_stage_allowed"], true);

    let stage_path = format!("{detail_path}/stage");
    let (status, staged) = call(
        &fixture.router,
        Method::POST,
        &stage_path,
        Some(&fixture.applier_token),
        &stage_body(&submitted, &reviewed, "management-stage", true),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{staged}");
    let (_, staged_preflight) = call(
        &fixture.router,
        Method::GET,
        &preflight_path,
        Some(&fixture.applier_token),
        &empty,
    )
    .await;
    assert_eq!(staged_preflight["request_status"], "staged");
    assert_eq!(staged_preflight["admission_present"], true);
    assert_eq!(staged_preflight["admin_stage_allowed"], false);
    assert_eq!(
        staged_preflight["blockers"][0],
        "adapter_release_already_staged"
    );

    let (_, staged_list) = call(
        &fixture.router,
        Method::GET,
        &format!("{}?status=staged", release_path()),
        Some(&fixture.applier_token),
        &empty,
    )
    .await;
    assert_eq!(
        staged_list["adapter_release_requests"][0]["admission"]["admission_id"],
        staged["admission_id"]
    );
    assert_eq!(
        call(
            &fixture.router,
            Method::GET,
            &format!("{}?status=canceled", release_path()),
            Some(&fixture.applier_token),
            &empty,
        )
        .await
        .0,
        StatusCode::BAD_REQUEST
    );
    assert_ledger_counts(&fixture, 1, 1, 1);
    fixture.cleanup();
}

#[tokio::test]
async fn changes_requested_is_queryable_but_has_no_remaining_write_action() {
    let fixture = fixture();
    let (status, submitted) = call(
        &fixture.router,
        Method::POST,
        release_path(),
        Some(&fixture.submitter_token),
        &submit_body("management-changes-submit", "4.0.0", true),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{submitted}");
    let request_id = submitted["request_id"].as_str().unwrap();
    let review_path = format!("{}/{request_id}/review", release_path());
    let (status, reviewed) = call(
        &fixture.router,
        Method::POST,
        &review_path,
        Some(&fixture.reviewer_token),
        &review_body(
            &submitted,
            "management-changes-review",
            "changes_requested",
            true,
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{reviewed}");

    let empty = json!({});
    let (_, preflight) = call(
        &fixture.router,
        Method::GET,
        &format!("{}/{request_id}/preflight", release_path()),
        Some(&fixture.applier_token),
        &empty,
    )
    .await;
    assert_eq!(preflight["request_status"], "changes_requested");
    assert_eq!(preflight["admin_review_allowed"], false);
    assert_eq!(preflight["admin_stage_allowed"], false);
    assert_eq!(
        preflight["blockers"][0],
        "changes_requested_requires_new_submission"
    );
    let (_, listed) = call(
        &fixture.router,
        Method::GET,
        &format!("{}?status=changes_requested", release_path()),
        Some(&fixture.reviewer_token),
        &empty,
    )
    .await;
    assert_eq!(
        listed["adapter_release_requests"][0]["review"]["decision"],
        "changes_requested"
    );
    assert_ledger_counts(&fixture, 1, 1, 0);
    fixture.cleanup();
}
