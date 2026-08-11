use super::{
    sui_preflight_job_model::{
        ClaimSuiPreflightJobRequest, CompleteSuiPreflightJobRequest, QueueSuiPreflightJobRequest,
        SuiPreflightJobIssue,
    },
    sui_preflight_job_service as service,
    sui_preflight_job_test_support::{fixture, RuntimeFlagGuard, SuiPreflightJobFixture},
    sui_preflight_service,
};

#[test]
fn report_insert_failure_rolls_back_completion_and_can_recover() {
    let _runtime = RuntimeFlagGuard::enabled();
    let fixture = fixture();
    let issue = queue_and_claim(&fixture);
    fixture
        .store
        .conn()
        .unwrap()
        .execute_batch(
            "CREATE TRIGGER reject_sui_preflight_report_insert
             BEFORE INSERT ON task_sui_preflight_reports
             BEGIN SELECT RAISE(ABORT, 'injected preflight report insert failure'); END;",
        )
        .unwrap();

    let error = service::complete(
        &fixture.store,
        &fixture.adapter,
        &issue.job.id,
        &completion(issue.lease_token.clone(), "report-insert-failure-recovery"),
    )
    .unwrap_err();
    assert!(format!("{error:#}").contains("injected preflight report insert failure"));
    assert_lease_and_report_rolled_back(&fixture, &issue.job.id);

    fixture
        .store
        .conn()
        .unwrap()
        .execute_batch("DROP TRIGGER reject_sui_preflight_report_insert;")
        .unwrap();
    let completed = service::complete(
        &fixture.store,
        &fixture.adapter,
        &issue.job.id,
        &completion(issue.lease_token, "report-insert-failure-recovery"),
    )
    .unwrap();
    assert_eq!(completed.job.status, "completed");
    assert_eq!(
        sui_preflight_service::list_reports(&fixture.store, &fixture.project_id)
            .unwrap()
            .reports
            .len(),
        1
    );
}

#[test]
fn task_update_failure_rolls_back_inserted_report_and_can_recover() {
    let _runtime = RuntimeFlagGuard::enabled();
    let fixture = fixture();
    let issue = queue_and_claim(&fixture);
    fixture
        .store
        .conn()
        .unwrap()
        .execute_batch(
            "CREATE TRIGGER reject_sui_preflight_task_completion
             BEFORE UPDATE OF status ON task_sui_preflight_jobs
             WHEN NEW.status = 'completed'
             BEGIN SELECT RAISE(ABORT, 'injected preflight task update failure'); END;",
        )
        .unwrap();

    let error = service::complete(
        &fixture.store,
        &fixture.adapter,
        &issue.job.id,
        &completion(issue.lease_token.clone(), "task-update-failure-recovery"),
    )
    .unwrap_err();
    assert!(format!("{error:#}").contains("injected preflight task update failure"));
    assert_lease_and_report_rolled_back(&fixture, &issue.job.id);

    fixture
        .store
        .conn()
        .unwrap()
        .execute_batch("DROP TRIGGER reject_sui_preflight_task_completion;")
        .unwrap();
    let completed = service::complete(
        &fixture.store,
        &fixture.adapter,
        &issue.job.id,
        &completion(issue.lease_token, "task-update-failure-recovery"),
    )
    .unwrap();
    assert_eq!(completed.job.status, "completed");
    assert_eq!(completed.report.outcome, "passed");
}

fn queue_and_claim(fixture: &SuiPreflightJobFixture) -> SuiPreflightJobIssue {
    service::queue(
        &fixture.store,
        &fixture.project_id,
        &fixture.owner_user_id,
        "owner",
        &QueueSuiPreflightJobRequest {
            package_kind: "standard".to_string(),
            projection_package_id: fixture.projection_id.clone(),
            confirmed_by_user: true,
        },
    )
    .unwrap();
    service::claim_next(
        &fixture.store,
        &fixture.adapter,
        &ClaimSuiPreflightJobRequest { lease_seconds: 300 },
    )
    .unwrap()
    .issue
    .unwrap()
}

fn assert_lease_and_report_rolled_back(fixture: &SuiPreflightJobFixture, job_id: &str) {
    let job = service::list(&fixture.store, &fixture.project_id)
        .unwrap()
        .jobs
        .into_iter()
        .find(|job| job.id == job_id)
        .unwrap();
    assert_eq!(job.status, "leased");
    assert!(job.report_id.is_none());
    assert!(job.completed_at.is_none());
    assert!(
        sui_preflight_service::list_reports(&fixture.store, &fixture.project_id)
            .unwrap()
            .reports
            .is_empty()
    );
}

fn completion(lease_token: String, idempotency_key: &str) -> CompleteSuiPreflightJobRequest {
    CompleteSuiPreflightJobRequest {
        lease_token,
        outcome: "passed".to_string(),
        summary: "offline package verified after injected write failure".to_string(),
        tool_version: "transaction-failure-test-v1".to_string(),
        idempotency_key: idempotency_key.to_string(),
    }
}
