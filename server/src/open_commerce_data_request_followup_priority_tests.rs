use std::sync::{Arc, Barrier};

use super::*;

#[test]
fn concurrent_reminder_double_click_persists_one_followup_and_audit() {
    let fixture = fixture();
    let request = initial_request(&fixture);
    set_request_times(&fixture, &request.id, Utc::now() - Duration::days(2));
    let barrier = Arc::new(Barrier::new(2));

    std::thread::scope(|scope| {
        let run = |barrier: Arc<Barrier>| {
            barrier.wait();
            follow_up(
                &fixture,
                &request.id,
                DATA_REQUEST_FOLLOWUP_ACTION_REMINDER,
                "double-click",
                "",
            )
        };
        let left = scope.spawn({
            let barrier = Arc::clone(&barrier);
            move || run(barrier)
        });
        let right = scope.spawn({
            let barrier = Arc::clone(&barrier);
            move || run(barrier)
        });
        assert!(left.join().unwrap().is_ok());
        assert!(right.join().unwrap().is_ok());
    });

    assert_eq!(followup_count(&fixture, &request.id, "reminder"), 1);
    assert_eq!(
        audit_count(&fixture, &request.id, "consumer_data_erasure.reminder"),
        1
    );
}

#[test]
fn concurrent_escalation_double_click_persists_one_followup_and_audit() {
    let fixture = fixture();
    let request = initial_request(&fixture);
    set_request_times(&fixture, &request.id, Utc::now() - Duration::days(8));
    follow_up(
        &fixture,
        &request.id,
        DATA_REQUEST_FOLLOWUP_ACTION_REMINDER,
        "before-concurrent-escalation",
        "",
    )
    .unwrap();
    let barrier = Arc::new(Barrier::new(2));

    std::thread::scope(|scope| {
        let run = |barrier: Arc<Barrier>| {
            barrier.wait();
            follow_up(
                &fixture,
                &request.id,
                DATA_REQUEST_FOLLOWUP_ACTION_ESCALATE,
                "concurrent-escalation",
                "",
            )
        };
        let left = scope.spawn({
            let barrier = Arc::clone(&barrier);
            move || run(barrier)
        });
        let right = scope.spawn({
            let barrier = Arc::clone(&barrier);
            move || run(barrier)
        });
        assert!(left.join().unwrap().is_ok());
        assert!(right.join().unwrap().is_ok());
    });

    assert_eq!(
        followup_count(&fixture, &request.id, "escalate_attention"),
        1
    );
    assert_eq!(
        audit_count(
            &fixture,
            &request.id,
            "consumer_data_erasure.escalate_attention"
        ),
        1
    );
}

#[test]
fn merchant_inbox_prioritizes_attention_before_limit() {
    let fixture = fixture();
    let escalated = initial_request(&fixture);
    set_request_times(&fixture, &escalated.id, Utc::now() - Duration::days(9));
    follow_up(
        &fixture,
        &escalated.id,
        DATA_REQUEST_FOLLOWUP_ACTION_REMINDER,
        "priority-reminder",
        "",
    )
    .unwrap();
    follow_up(
        &fixture,
        &escalated.id,
        DATA_REQUEST_FOLLOWUP_ACTION_ESCALATE,
        "priority-escalation",
        "",
    )
    .unwrap();

    let overdue = additional_request(&fixture, "overdue");
    set_request_times(&fixture, &overdue.id, Utc::now() - Duration::days(8));
    let _normal = additional_request(&fixture, "normal");
    let terminal = additional_request(&fixture, "terminal");
    let merchant = fixture.merchant_actor("owner");
    open_commerce_data_request_service::decide_request(
        &fixture.store,
        &fixture.merchant_project_id,
        &fixture.merchant_id,
        &terminal.id,
        &merchant,
        decision("accept", "开始处理"),
    )
    .unwrap();
    open_commerce_data_request_service::decide_request(
        &fixture.store,
        &fixture.merchant_project_id,
        &fixture.merchant_id,
        &terminal.id,
        &merchant,
        decision("complete", "已处理"),
    )
    .unwrap();

    let inbox = open_commerce_data_request_service::list_merchant_requests(
        &fixture.store,
        &fixture.merchant_project_id,
        &fixture.merchant_id,
        &merchant,
        2,
    )
    .unwrap();
    assert_eq!(inbox.len(), 2);
    assert_eq!(inbox[0].id, escalated.id);
    assert_eq!(inbox[1].id, overdue.id);
    assert!(inbox[0].consumer_escalated_at.is_some());
    assert!(inbox[1].is_operationally_overdue);
}
