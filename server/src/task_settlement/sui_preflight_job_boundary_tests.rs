use chrono::DateTime;
use rusqlite::Connection;

use super::{
    sui_preflight_job_model::{
        ClaimSuiPreflightJobRequest, CompleteSuiPreflightJobRequest, QueueSuiPreflightJobRequest,
        SuiPreflightJob, SuiPreflightJobIssue,
    },
    sui_preflight_job_service as service,
    sui_preflight_job_test_support::{fixture, RuntimeFlagGuard, SuiPreflightJobFixture},
    sui_preflight_model::{
        CreateSuiPreflightAdapterRequest, RecordSuiPreflightReportRequest, SuiPreflightAdapter,
    },
    sui_preflight_service,
};

#[test]
fn lease_bounds_and_forced_hard_deadline_fail_closed_then_recover() {
    let _runtime = RuntimeFlagGuard::enabled();
    let fixture = fixture();
    let queued = queue(&fixture);

    assert!(service::claim_next(
        &fixture.store,
        &fixture.adapter,
        &ClaimSuiPreflightJobRequest { lease_seconds: 59 },
    )
    .is_err());
    assert!(service::claim_next(
        &fixture.store,
        &fixture.adapter,
        &ClaimSuiPreflightJobRequest { lease_seconds: 901 },
    )
    .is_err());

    let first = claim(&fixture, &fixture.adapter, 60);
    let conn = Connection::open(&fixture.path).unwrap();
    conn.execute(
        "UPDATE task_sui_preflight_jobs
            SET lease_expires_at='2000-01-01T00:00:00Z',
                lease_deadline_at='2000-01-01T00:00:00Z'
          WHERE id=?1",
        [&queued.id],
    )
    .unwrap();
    drop(conn);

    assert!(service::complete(
        &fixture.store,
        &fixture.adapter,
        &queued.id,
        &completion(first.lease_token, "expired-hard-deadline"),
    )
    .is_err());
    let second = claim(&fixture, &fixture.adapter, 900);
    assert_eq!(second.job.attempt_no, 2);
    let expires_at =
        DateTime::parse_from_rfc3339(second.job.lease_expires_at.as_deref().unwrap()).unwrap();
    let deadline_at =
        DateTime::parse_from_rfc3339(second.job.lease_deadline_at.as_deref().unwrap()).unwrap();
    assert!(expires_at <= deadline_at);
}

#[test]
fn machine_adapter_cannot_claim_jobs_from_another_project() {
    let _runtime = RuntimeFlagGuard::enabled();
    let fixture = fixture();
    let queued = queue(&fixture);
    let other_project = fixture
        .store
        .create_project(
            &fixture.owner_user_id,
            "Isolated Sui preflight worker project",
            None,
            None,
        )
        .unwrap()
        .project;
    let other_adapter = sui_preflight_service::create_adapter(
        &fixture.store,
        &other_project.id,
        &fixture.owner_user_id,
        "owner",
        &CreateSuiPreflightAdapterRequest {
            display_name: "Other project worker".to_string(),
            allowed_networks: vec!["testnet".to_string()],
            allowed_package_kinds: vec!["standard".to_string()],
            expires_in_days: 30,
            confirmed_by_user: true,
        },
    )
    .unwrap()
    .adapter;

    let isolated = service::claim_next(
        &fixture.store,
        &other_adapter,
        &ClaimSuiPreflightJobRequest { lease_seconds: 300 },
    )
    .unwrap();
    assert!(!isolated.claimed);
    assert!(isolated.issue.is_none());
    let claimed = claim(&fixture, &fixture.adapter, 300);
    assert_eq!(claimed.job.id, queued.id);
    assert_eq!(claimed.job.project_id, fixture.project_id);
}

#[test]
fn report_conflict_rolls_back_job_completion_until_new_key_is_used() {
    let _runtime = RuntimeFlagGuard::enabled();
    let fixture = fixture();
    let queued = queue(&fixture);
    let issue = claim(&fixture, &fixture.adapter, 300);
    let conflict_key = "preflight-transaction-conflict";
    let existing = sui_preflight_service::record_report(
        &fixture.store,
        &fixture.adapter,
        &RecordSuiPreflightReportRequest {
            package_kind: queued.package_kind.clone(),
            projection_package_id: queued.projection_package_id.clone(),
            handoff_digest: issue.handoff.handoff_digest.clone(),
            outcome: "rejected".to_string(),
            summary: "existing rejected report".to_string(),
            tool_version: "boundary-test-v1".to_string(),
            idempotency_key: conflict_key.to_string(),
        },
    )
    .unwrap();

    assert!(service::complete(
        &fixture.store,
        &fixture.adapter,
        &queued.id,
        &completion(issue.lease_token.clone(), conflict_key),
    )
    .is_err());
    let still_leased = fixture
        .store
        .task_sui_preflight_job(&fixture.project_id, &queued.id)
        .unwrap();
    assert_eq!(still_leased.status, "leased");
    assert!(still_leased.report_id.is_none());
    let reports = sui_preflight_service::list_reports(&fixture.store, &fixture.project_id).unwrap();
    assert_eq!(reports.reports.len(), 1);
    assert_eq!(reports.reports[0].id, existing.id);

    let completed = service::complete(
        &fixture.store,
        &fixture.adapter,
        &queued.id,
        &completion(issue.lease_token, "preflight-transaction-recovery"),
    )
    .unwrap();
    assert_eq!(completed.job.status, "completed");
    assert_ne!(completed.report.id, existing.id);
    assert_eq!(
        sui_preflight_service::list_reports(&fixture.store, &fixture.project_id)
            .unwrap()
            .reports
            .len(),
        2
    );
}

fn queue(fixture: &SuiPreflightJobFixture) -> SuiPreflightJob {
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
    .unwrap()
}

fn claim(
    fixture: &SuiPreflightJobFixture,
    adapter: &SuiPreflightAdapter,
    lease_seconds: i64,
) -> SuiPreflightJobIssue {
    service::claim_next(
        &fixture.store,
        adapter,
        &ClaimSuiPreflightJobRequest { lease_seconds },
    )
    .unwrap()
    .issue
    .unwrap()
}

fn completion(lease_token: String, idempotency_key: &str) -> CompleteSuiPreflightJobRequest {
    CompleteSuiPreflightJobRequest {
        lease_token,
        outcome: "passed".to_string(),
        summary: "offline package verified".to_string(),
        tool_version: "boundary-test-v1".to_string(),
        idempotency_key: idempotency_key.to_string(),
    }
}
