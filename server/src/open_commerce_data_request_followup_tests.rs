use chrono::{Duration, Utc};
use rusqlite::params;

use super::{decision, fixture, Fixture};
use crate::{
    open_commerce_data_request_model::{
        CreateConsumerDataErasureRequest, FollowUpConsumerDataRequest,
        OpenCommerceConsumerDataRequest, DATA_REQUEST_FOLLOWUP_ACTION_ESCALATE,
        DATA_REQUEST_FOLLOWUP_ACTION_REMINDER,
    },
    open_commerce_data_request_service,
    open_commerce_relationship_model::{
        CreateConsumerRelationshipRequest, RELATIONSHIP_SCOPE_PREFERENCE_REMEMBER,
    },
    open_commerce_relationship_service,
};

#[test]
fn reminders_are_bounded_idempotent_and_replay_after_terminal() {
    let fixture = fixture();
    let request = initial_request(&fixture);
    set_request_times(&fixture, &request.id, Utc::now() - Duration::days(8));

    assert!(follow_up(
        &fixture,
        &request.id,
        DATA_REQUEST_FOLLOWUP_ACTION_REMINDER,
        "",
        "",
    )
    .is_err());
    assert!(follow_up(
        &fixture,
        &request.id,
        DATA_REQUEST_FOLLOWUP_ACTION_REMINDER,
        &"x".repeat(121),
        "",
    )
    .is_err());
    assert!(follow_up(
        &fixture,
        &request.id,
        DATA_REQUEST_FOLLOWUP_ACTION_REMINDER,
        "oversized-note",
        &"字".repeat(501),
    )
    .is_err());

    let first = follow_up(
        &fixture,
        &request.id,
        DATA_REQUEST_FOLLOWUP_ACTION_REMINDER,
        "reminder-1",
        " 第一次催办 ",
    )
    .unwrap();
    assert_eq!(first.reminder_count, 1);
    assert!(!first.can_send_reminder);
    assert_eq!(followup_count(&fixture, &request.id, "reminder"), 1);
    assert_eq!(
        audit_count(&fixture, &request.id, "consumer_data_erasure.reminder"),
        1
    );

    let replay = follow_up(
        &fixture,
        &request.id,
        DATA_REQUEST_FOLLOWUP_ACTION_REMINDER,
        "reminder-1",
        "第一次催办",
    )
    .unwrap();
    assert_eq!(replay.reminder_count, 1);
    assert_eq!(followup_count(&fixture, &request.id, "reminder"), 1);
    assert_eq!(
        audit_count(&fixture, &request.id, "consumer_data_erasure.reminder"),
        1
    );
    assert!(follow_up(
        &fixture,
        &request.id,
        DATA_REQUEST_FOLLOWUP_ACTION_REMINDER,
        "reminder-1",
        "不同说明",
    )
    .unwrap_err()
    .to_string()
    .contains("不同"));
    assert!(follow_up(
        &fixture,
        &request.id,
        DATA_REQUEST_FOLLOWUP_ACTION_ESCALATE,
        "reminder-1",
        "第一次催办",
    )
    .unwrap_err()
    .to_string()
    .contains("不同"));
    assert!(follow_up(
        &fixture,
        &request.id,
        DATA_REQUEST_FOLLOWUP_ACTION_REMINDER,
        "reminder-2",
        "",
    )
    .unwrap_err()
    .to_string()
    .contains("下一次"));

    age_followup(&fixture, &request.id, "reminder-1", Duration::hours(25));
    follow_up(
        &fixture,
        &request.id,
        DATA_REQUEST_FOLLOWUP_ACTION_REMINDER,
        "reminder-2",
        "",
    )
    .unwrap();
    age_followup(&fixture, &request.id, "reminder-2", Duration::hours(25));
    let third = follow_up(
        &fixture,
        &request.id,
        DATA_REQUEST_FOLLOWUP_ACTION_REMINDER,
        "reminder-3",
        "",
    )
    .unwrap();
    assert_eq!(third.reminder_count, 3);
    assert!(third.next_reminder_at.is_none());
    age_followup(&fixture, &request.id, "reminder-3", Duration::hours(25));
    assert!(follow_up(
        &fixture,
        &request.id,
        DATA_REQUEST_FOLLOWUP_ACTION_REMINDER,
        "reminder-4",
        "",
    )
    .unwrap_err()
    .to_string()
    .contains("最多三次"));

    let merchant = fixture.merchant_actor("owner");
    open_commerce_data_request_service::decide_request(
        &fixture.store,
        &fixture.merchant_project_id,
        &fixture.merchant_id,
        &request.id,
        &merchant,
        decision("accept", "开始处理"),
    )
    .unwrap();
    open_commerce_data_request_service::decide_request(
        &fixture.store,
        &fixture.merchant_project_id,
        &fixture.merchant_id,
        &request.id,
        &merchant,
        decision("complete", "已完成外部系统清理"),
    )
    .unwrap();
    let terminal_replay = follow_up(
        &fixture,
        &request.id,
        DATA_REQUEST_FOLLOWUP_ACTION_REMINDER,
        "reminder-1",
        "第一次催办",
    )
    .unwrap();
    assert_eq!(terminal_replay.status, "completed");
    assert_eq!(followup_count(&fixture, &request.id, "reminder"), 3);
    assert!(follow_up(
        &fixture,
        &request.id,
        DATA_REQUEST_FOLLOWUP_ACTION_REMINDER,
        "terminal-new",
        "",
    )
    .unwrap_err()
    .to_string()
    .contains("待处理或处理中"));
}

#[test]
fn escalation_requires_target_and_reminder_and_is_idempotent() {
    let fixture = fixture();
    let overdue = initial_request(&fixture);
    set_request_times(&fixture, &overdue.id, Utc::now() - Duration::days(7));
    assert!(follow_up(
        &fixture,
        &overdue.id,
        DATA_REQUEST_FOLLOWUP_ACTION_ESCALATE,
        "escalate-without-reminder",
        "",
    )
    .is_err());
    follow_up(
        &fixture,
        &overdue.id,
        DATA_REQUEST_FOLLOWUP_ACTION_REMINDER,
        "overdue-reminder",
        "",
    )
    .unwrap();
    let escalated = follow_up(
        &fixture,
        &overdue.id,
        DATA_REQUEST_FOLLOWUP_ACTION_ESCALATE,
        "escalation",
        "请优先关注",
    )
    .unwrap();
    assert!(escalated.consumer_escalated_at.is_some());
    assert!(!escalated.can_escalate_attention);
    let replay = follow_up(
        &fixture,
        &overdue.id,
        DATA_REQUEST_FOLLOWUP_ACTION_ESCALATE,
        "escalation",
        "请优先关注",
    )
    .unwrap();
    assert_eq!(
        replay.consumer_escalated_at,
        escalated.consumer_escalated_at
    );
    assert_eq!(
        followup_count(&fixture, &overdue.id, "escalate_attention"),
        1
    );
    assert!(follow_up(
        &fixture,
        &overdue.id,
        DATA_REQUEST_FOLLOWUP_ACTION_ESCALATE,
        "second-escalation",
        "",
    )
    .is_err());

    let early = additional_request(&fixture, "early");
    set_request_times(&fixture, &early.id, Utc::now() - Duration::days(2));
    follow_up(
        &fixture,
        &early.id,
        DATA_REQUEST_FOLLOWUP_ACTION_REMINDER,
        "early-reminder",
        "",
    )
    .unwrap();
    assert!(follow_up(
        &fixture,
        &early.id,
        DATA_REQUEST_FOLLOWUP_ACTION_ESCALATE,
        "early-escalation",
        "",
    )
    .unwrap_err()
    .to_string()
    .contains("超过七天"));
}

fn initial_request(fixture: &Fixture) -> OpenCommerceConsumerDataRequest {
    create_request(fixture, &fixture.relationship_id)
}

fn additional_request(fixture: &Fixture, suffix: &str) -> OpenCommerceConsumerDataRequest {
    let relationship = open_commerce_relationship_service::create_relationship(
        &fixture.store,
        &fixture.consumer_project_id,
        &fixture.consumer_actor("owner"),
        CreateConsumerRelationshipRequest {
            merchant_id: fixture.merchant_id.clone(),
            source_app_id: "pc-web".to_string(),
            scopes: vec![RELATIONSHIP_SCOPE_PREFERENCE_REMEMBER.to_string()],
            purpose: format!("跟进测试关系 {suffix}"),
            expires_at: (Utc::now() + Duration::days(90)).to_rfc3339(),
        },
    )
    .unwrap();
    create_request(fixture, &relationship.id)
}

fn create_request(fixture: &Fixture, relationship_id: &str) -> OpenCommerceConsumerDataRequest {
    open_commerce_data_request_service::create_erasure_request(
        &fixture.store,
        &fixture.consumer_project_id,
        &fixture.consumer_actor("owner"),
        CreateConsumerDataErasureRequest {
            relationship_id: relationship_id.to_string(),
        },
    )
    .unwrap()
}

fn follow_up(
    fixture: &Fixture,
    request_id: &str,
    action: &str,
    idempotency_key: &str,
    note: &str,
) -> anyhow::Result<OpenCommerceConsumerDataRequest> {
    open_commerce_data_request_service::follow_up_request(
        &fixture.store,
        &fixture.consumer_project_id,
        request_id,
        &fixture.consumer_actor("owner"),
        FollowUpConsumerDataRequest {
            action: action.to_string(),
            idempotency_key: idempotency_key.to_string(),
            note: note.to_string(),
        },
    )
}

fn followup_request(idempotency_key: &str) -> FollowUpConsumerDataRequest {
    FollowUpConsumerDataRequest {
        action: DATA_REQUEST_FOLLOWUP_ACTION_REMINDER.to_string(),
        idempotency_key: idempotency_key.to_string(),
        note: String::new(),
    }
}

fn consumer_requests(fixture: &Fixture) -> anyhow::Result<Vec<OpenCommerceConsumerDataRequest>> {
    open_commerce_data_request_service::list_consumer_requests(
        &fixture.store,
        &fixture.consumer_project_id,
        &fixture.consumer_actor("owner"),
        100,
    )
}

fn consumer_request(fixture: &Fixture, request_id: &str) -> OpenCommerceConsumerDataRequest {
    consumer_requests(fixture)
        .unwrap()
        .into_iter()
        .find(|request| request.id == request_id)
        .unwrap()
}

fn set_request_times(fixture: &Fixture, request_id: &str, timestamp: chrono::DateTime<Utc>) {
    let timestamp = timestamp.to_rfc3339();
    set_request_time_text(fixture, request_id, &timestamp, &timestamp);
}

fn set_request_time_text(
    fixture: &Fixture,
    request_id: &str,
    requested_at: &str,
    updated_at: &str,
) {
    fixture
        .store
        .conn()
        .unwrap()
        .execute(
            "UPDATE open_commerce_consumer_data_requests
                SET requested_at=?1, updated_at=?2 WHERE id=?3",
            params![requested_at, updated_at, request_id],
        )
        .unwrap();
}

fn age_followup(fixture: &Fixture, request_id: &str, key: &str, age: Duration) {
    set_followup_time(fixture, request_id, key, &(Utc::now() - age).to_rfc3339());
}

fn set_followup_time(fixture: &Fixture, request_id: &str, key: &str, timestamp: &str) {
    fixture
        .store
        .conn()
        .unwrap()
        .execute(
            "UPDATE open_commerce_data_request_followups SET created_at=?1
              WHERE data_request_id=?2 AND idempotency_key=?3",
            params![timestamp, request_id, key],
        )
        .unwrap();
}

fn followup_count(fixture: &Fixture, request_id: &str, action: &str) -> i64 {
    fixture
        .store
        .conn()
        .unwrap()
        .query_row(
            "SELECT COUNT(*) FROM open_commerce_data_request_followups
              WHERE data_request_id=?1 AND action_kind=?2",
            params![request_id, action],
            |row| row.get(0),
        )
        .unwrap()
}

fn audit_count(fixture: &Fixture, request_id: &str, action: &str) -> i64 {
    fixture
        .store
        .conn()
        .unwrap()
        .query_row(
            "SELECT COUNT(*) FROM open_commerce_audit_events
              WHERE subject_id=?1 AND action=?2",
            params![request_id, action],
            |row| row.get(0),
        )
        .unwrap()
}

#[path = "open_commerce_data_request_followup_priority_tests.rs"]
mod priority_tests;

#[path = "open_commerce_data_request_followup_validation_tests.rs"]
mod validation_tests;
