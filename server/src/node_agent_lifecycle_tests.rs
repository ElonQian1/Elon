use super::*;

#[test]
fn restart_after_unmarked_session_reports_unexpected_exit() {
    let base = std::env::temp_dir().join(format!(
        "elon-lifecycle-test-{}-{}.json",
        process::id(),
        uuid::Uuid::new_v4().simple()
    ));
    let first = NodeLifecycleTracker::start_at(base.clone(), "1.0.0", 1_000);
    drop(first);
    let second = NodeLifecycleTracker::start_at(base.clone(), "1.0.1", 80_000);
    let report = second.report_at(
        LifecycleInputs {
            connected: true,
            logged_in: true,
            last_event: "已连接",
            active_task_count: 0,
            sidecar_session_count: 0,
        },
        80_010,
    );

    assert_eq!(
        report.previous_exit_kind.as_deref(),
        Some("unexpected_exit")
    );
    assert_eq!(report.state, "recovered_after_unexpected_exit");
    assert_eq!(report.recommended_action, "review_previous_session");

    let _ = fs::remove_file(base);
}

#[test]
fn planned_shutdown_is_not_reported_as_crash() {
    let base = std::env::temp_dir().join(format!(
        "elon-lifecycle-planned-{}-{}.json",
        process::id(),
        uuid::Uuid::new_v4().simple()
    ));
    let first = NodeLifecycleTracker::start_at(base.clone(), "1.0.0", 1_000);
    first.mark_planned_shutdown("update");
    let second = NodeLifecycleTracker::start_at(base.clone(), "1.0.1", 2_000);
    let report = second.report_at(
        LifecycleInputs {
            connected: true,
            logged_in: true,
            last_event: "已连接",
            active_task_count: 0,
            sidecar_session_count: 0,
        },
        2_010,
    );

    assert_eq!(report.previous_exit_kind.as_deref(), Some("planned_update"));
    assert_eq!(report.state, "healthy");
    assert_eq!(report.recommended_action, "none");

    let _ = fs::remove_file(base);
}

#[test]
fn active_recovery_tasks_take_priority() {
    let base = std::env::temp_dir().join(format!(
        "elon-lifecycle-active-{}-{}.json",
        process::id(),
        uuid::Uuid::new_v4().simple()
    ));
    let tracker = NodeLifecycleTracker::start_at(base.clone(), "1.0.0", 1_000);
    let report = tracker.report_at(
        LifecycleInputs {
            connected: true,
            logged_in: true,
            last_event: "已连接",
            active_task_count: 1,
            sidecar_session_count: 2,
        },
        1_010,
    );

    assert!(report.restart_recovery);
    assert_eq!(report.recommended_action, "review_task_recovery");

    let _ = fs::remove_file(base);
}
