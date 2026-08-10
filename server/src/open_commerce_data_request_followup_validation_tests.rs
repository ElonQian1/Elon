use chrono::FixedOffset;
use rusqlite::params;
use serde_json::Value;

use super::*;
use crate::open_commerce_service::OpenCommerceActor;

#[test]
fn operational_windows_fail_closed_and_raw_export_stays_compatible() {
    let fixture = fixture();
    let request = initial_request(&fixture);

    let raw = fixture
        .store
        .list_open_commerce_consumer_data_requests(
            &fixture.consumer_project_id,
            &fixture.consumer_user_id,
            10,
        )
        .unwrap();
    let raw_json = serde_json::to_value(&raw).unwrap();
    for field in [
        "operational_target_at",
        "is_operationally_overdue",
        "reminder_count",
        "last_reminded_at",
        "next_reminder_at",
        "consumer_escalated_at",
        "can_send_reminder",
        "can_escalate_attention",
    ] {
        assert!(!raw_json[0].as_object().unwrap().contains_key(field));
    }

    set_request_times(
        &fixture,
        &request.id,
        Utc::now() - Duration::hours(23) - Duration::minutes(59),
    );
    assert!(!consumer_request(&fixture, &request.id).can_send_reminder);

    set_request_times(&fixture, &request.id, Utc::now() - Duration::hours(24));
    let exact_boundary = consumer_request(&fixture, &request.id);
    assert!(exact_boundary.can_send_reminder);
    assert!(exact_boundary.operational_target_at.is_some());

    let offset = FixedOffset::east_opt(8 * 60 * 60).unwrap();
    let cross_offset = (Utc::now() - Duration::hours(25))
        .with_timezone(&offset)
        .to_rfc3339();
    set_request_time_text(&fixture, &request.id, &cross_offset, &cross_offset);
    assert!(consumer_request(&fixture, &request.id).can_send_reminder);

    set_request_time_text(&fixture, &request.id, "not-a-time", "not-a-time");
    assert!(consumer_requests(&fixture)
        .unwrap_err()
        .to_string()
        .contains("时间无效"));

    let future = (Utc::now() + Duration::hours(1)).to_rfc3339();
    set_request_time_text(&fixture, &request.id, &future, &future);
    assert!(consumer_requests(&fixture)
        .unwrap_err()
        .to_string()
        .contains("不能晚于当前时间"));
}

#[test]
fn followups_are_consumer_scoped_and_audit_metadata_is_safe() {
    let fixture = fixture();
    let request = initial_request(&fixture);
    set_request_times(&fixture, &request.id, Utc::now() - Duration::days(2));
    follow_up(
        &fixture,
        &request.id,
        DATA_REQUEST_FOLLOWUP_ACTION_REMINDER,
        "safe-audit-key",
        "原始私密说明",
    )
    .unwrap();

    let stranger = fixture
        .store
        .create_user("followup-stranger@example.com", "secret1", None, None)
        .unwrap();
    let stranger_project = fixture
        .store
        .create_project(&stranger.id, "Followup stranger", None, None)
        .unwrap()
        .project;
    let stranger_actor = OpenCommerceActor {
        user_id: &stranger.id,
        app_id: "pc-web",
        project_role: Some("owner"),
    };
    assert!(open_commerce_data_request_service::follow_up_request(
        &fixture.store,
        &fixture.consumer_project_id,
        &request.id,
        &stranger_actor,
        followup_request("stranger"),
    )
    .is_err());
    assert!(open_commerce_data_request_service::follow_up_request(
        &fixture.store,
        &stranger_project.id,
        &request.id,
        &fixture.consumer_actor("owner"),
        followup_request("wrong-project"),
    )
    .is_err());
    assert!(open_commerce_data_request_service::follow_up_request(
        &fixture.store,
        &fixture.consumer_project_id,
        "missing-request",
        &fixture.consumer_actor("owner"),
        followup_request("missing"),
    )
    .is_err());
    assert!(open_commerce_data_request_service::list_merchant_requests(
        &fixture.store,
        &fixture.consumer_project_id,
        &fixture.merchant_id,
        &fixture.consumer_actor("owner"),
        10,
    )
    .is_err());

    let metadata: String = fixture
        .store
        .conn()
        .unwrap()
        .query_row(
            "SELECT metadata_json FROM open_commerce_audit_events
              WHERE action='consumer_data_erasure.reminder' AND subject_id=?1",
            params![request.id],
            |row| row.get(0),
        )
        .unwrap();
    let metadata: Value = serde_json::from_str(&metadata).unwrap();
    assert_eq!(metadata["legal_deadline_asserted"], false);
    assert_eq!(metadata["platform_adjudication_started"], false);
    let serialized = metadata.to_string();
    for secret in [
        fixture.consumer_user_id.as_str(),
        fixture.consumer_project_id.as_str(),
        "safe-audit-key",
        "原始私密说明",
        "consumer_user_id",
        "consumer_project_id",
        "idempotency_key",
        "note",
    ] {
        assert!(!serialized.contains(secret));
    }
}

#[test]
fn corrupt_followup_times_fail_closed() {
    let fixture = fixture();
    let request = initial_request(&fixture);
    set_request_times(&fixture, &request.id, Utc::now() - Duration::days(2));
    follow_up(
        &fixture,
        &request.id,
        DATA_REQUEST_FOLLOWUP_ACTION_REMINDER,
        "corrupt-time",
        "",
    )
    .unwrap();

    set_followup_time(&fixture, &request.id, "corrupt-time", "not-a-time");
    assert!(consumer_requests(&fixture)
        .unwrap_err()
        .to_string()
        .contains("时间无效"));
    let future = (Utc::now() + Duration::hours(1)).to_rfc3339();
    set_followup_time(&fixture, &request.id, "corrupt-time", &future);
    assert!(consumer_requests(&fixture)
        .unwrap_err()
        .to_string()
        .contains("超出有效范围"));
}
