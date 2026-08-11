use std::sync::{Arc, Barrier};

use rusqlite::Connection;

use crate::store::Store;

use super::{
    sui_preflight_job_model::{
        ClaimSuiPreflightJobRequest, CompleteSuiPreflightJobRequest, QueueSuiPreflightJobRequest,
        ReleaseSuiPreflightJobRequest, RenewSuiPreflightJobRequest,
    },
    sui_preflight_job_service as service,
    sui_preflight_job_test_support::{fixture, RuntimeFlagGuard},
    sui_preflight_model::CreateSuiPreflightAdapterRequest,
    sui_preflight_service, sui_projection_service,
};

#[test]
fn lease_lifecycle_releases_reclaims_and_completes_idempotently() {
    let _runtime = RuntimeFlagGuard::enabled();
    let fixture = fixture();
    let queued = queue(&fixture);
    let replay = queue(&fixture);
    assert_eq!(queued.id, replay.id);

    let first = service::claim_next(
        &fixture.store,
        &fixture.adapter,
        &ClaimSuiPreflightJobRequest { lease_seconds: 60 },
    )
    .unwrap();
    let first_issue = first.issue.unwrap();
    assert!(first.claimed);
    assert!(first_issue.lease_token_visible_once);
    assert_eq!(first_issue.job.status, "leased");
    assert_eq!(first_issue.job.attempt_no, 1);
    assert_eq!(
        first_issue.handoff.payload.constraints.signature_present,
        false
    );
    assert_eq!(
        first_issue
            .handoff
            .payload
            .constraints
            .transaction_broadcast,
        false
    );
    assert_eq!(first_issue.handoff.payload.constraints.funds_moved, false);

    let empty = service::claim_next(
        &fixture.store,
        &fixture.adapter,
        &ClaimSuiPreflightJobRequest { lease_seconds: 60 },
    )
    .unwrap();
    assert!(!empty.claimed);
    assert!(empty.issue.is_none());

    let renewed = service::renew(
        &fixture.store,
        &fixture.adapter,
        &queued.id,
        &RenewSuiPreflightJobRequest {
            lease_token: first_issue.lease_token.clone(),
            extend_seconds: 300,
        },
    )
    .unwrap();
    assert!(renewed.renewed);
    assert_eq!(renewed.job.status, "leased");

    let released = service::release(
        &fixture.store,
        &fixture.adapter,
        &queued.id,
        &ReleaseSuiPreflightJobRequest {
            lease_token: first_issue.lease_token,
            reason: "worker capacity changed".to_string(),
        },
    )
    .unwrap();
    assert!(released.released);
    assert_eq!(released.job.status, "pending");
    assert_eq!(
        released.job.last_error.as_deref(),
        Some("worker capacity changed")
    );

    let second = service::claim_next(
        &fixture.store,
        &fixture.adapter,
        &ClaimSuiPreflightJobRequest { lease_seconds: 300 },
    )
    .unwrap()
    .issue
    .unwrap();
    assert_eq!(second.job.id, queued.id);
    assert_eq!(second.job.attempt_no, 2);

    let completion = CompleteSuiPreflightJobRequest {
        lease_token: second.lease_token,
        outcome: "passed".to_string(),
        summary: "offline package verified".to_string(),
        tool_version: "local-test-v1".to_string(),
        idempotency_key: "sui-preflight-job-complete-1".to_string(),
    };
    let completed =
        service::complete(&fixture.store, &fixture.adapter, &queued.id, &completion).unwrap();
    let replayed =
        service::complete(&fixture.store, &fixture.adapter, &queued.id, &completion).unwrap();
    assert!(completed.completed);
    assert_eq!(completed.job.status, "completed");
    assert_eq!(
        completed.job.report_id.as_deref(),
        Some(completed.report.id.as_str())
    );
    assert_eq!(completed.report.id, replayed.report.id);
    assert_eq!(
        completed.report.report_digest,
        replayed.report.report_digest
    );
}

#[test]
fn concurrent_workers_issue_exactly_one_lease() {
    let _runtime = RuntimeFlagGuard::enabled();
    let fixture = fixture();
    let queued = queue(&fixture);
    let barrier = Arc::new(Barrier::new(2));

    let handles = (0..2)
        .map(|_| {
            let store = Store::open(&fixture.path).unwrap();
            let adapter = fixture.adapter.clone();
            let barrier = Arc::clone(&barrier);
            std::thread::spawn(move || {
                barrier.wait();
                service::claim_next(
                    &store,
                    &adapter,
                    &ClaimSuiPreflightJobRequest { lease_seconds: 300 },
                )
                .unwrap()
            })
        })
        .collect::<Vec<_>>();
    let polls = handles
        .into_iter()
        .map(|handle| handle.join().unwrap())
        .collect::<Vec<_>>();

    assert_eq!(polls.iter().filter(|poll| poll.claimed).count(), 1);
    assert_eq!(polls.iter().filter(|poll| poll.issue.is_some()).count(), 1);
    let leased = fixture
        .store
        .task_sui_preflight_job(&fixture.project_id, &queued.id)
        .unwrap();
    assert_eq!(leased.status, "leased");
    assert_eq!(leased.attempt_no, 1);
}

#[test]
fn expired_lease_is_reclaimed_and_stale_token_is_rejected() {
    let _runtime = RuntimeFlagGuard::enabled();
    let fixture = fixture();
    let queued = queue(&fixture);
    let first = claim(&fixture, &fixture.adapter, 60);
    let first_token = first.lease_token;

    let conn = Connection::open(&fixture.path).unwrap();
    conn.execute(
        "UPDATE task_sui_preflight_jobs
            SET lease_expires_at='2000-01-01T00:00:00Z'
          WHERE id=?1",
        [&queued.id],
    )
    .unwrap();
    drop(conn);

    let second = claim(&fixture, &fixture.adapter, 300);
    assert_eq!(second.job.id, queued.id);
    assert_eq!(second.job.attempt_no, 2);
    assert_ne!(second.lease_token, first_token);
    assert!(service::renew(
        &fixture.store,
        &fixture.adapter,
        &queued.id,
        &RenewSuiPreflightJobRequest {
            lease_token: first_token,
            extend_seconds: 300,
        },
    )
    .is_err());
}

#[test]
fn adapter_scope_rotation_and_disable_fail_closed() {
    let _runtime = RuntimeFlagGuard::enabled();
    let fixture = fixture();
    let queued = queue(&fixture);
    let wrong_network = create_adapter(&fixture, &["devnet"], &["standard"]);
    let wrong_kind = create_adapter(&fixture, &["testnet"], &["correction"]);
    assert!(!poll(&fixture, &wrong_network).claimed);
    assert!(!poll(&fixture, &wrong_kind).claimed);

    let first = claim(&fixture, &fixture.adapter, 60);
    let rotated = sui_preflight_service::rotate_adapter(
        &fixture.store,
        &fixture.project_id,
        &fixture.adapter.id,
        30,
        "owner",
    )
    .unwrap();
    assert!(service::renew(
        &fixture.store,
        &fixture.adapter,
        &queued.id,
        &RenewSuiPreflightJobRequest {
            lease_token: first.lease_token,
            extend_seconds: 300,
        },
    )
    .is_err());
    assert!(fixture
        .store
        .authenticate_task_sui_preflight_adapter(&fixture.adapter_token)
        .is_err());

    let conn = Connection::open(&fixture.path).unwrap();
    conn.execute(
        "UPDATE task_sui_preflight_jobs
            SET lease_expires_at='2000-01-01T00:00:00Z'
          WHERE id=?1",
        [&queued.id],
    )
    .unwrap();
    drop(conn);
    let second = claim(&fixture, &rotated.adapter, 300);
    assert_eq!(second.job.attempt_no, 2);

    sui_preflight_service::disable_adapter(
        &fixture.store,
        &fixture.project_id,
        &rotated.adapter.id,
        "owner",
    )
    .unwrap();
    assert!(fixture
        .store
        .authenticate_task_sui_preflight_adapter(&rotated.adapter_token)
        .is_err());
    assert!(service::complete(
        &fixture.store,
        &rotated.adapter,
        &queued.id,
        &completion(second.lease_token, "disabled-adapter-complete"),
    )
    .is_err());
    assert_eq!(
        fixture
            .store
            .task_sui_preflight_job(&fixture.project_id, &queued.id)
            .unwrap()
            .status,
        "leased"
    );
}

#[test]
fn handoff_digest_drift_blocks_old_job_and_allows_fresh_requeue() {
    let _runtime = RuntimeFlagGuard::enabled();
    let fixture = fixture();
    let queued = queue(&fixture);
    let conn = Connection::open(&fixture.path).unwrap();
    conn.execute(
        "UPDATE task_sui_preflight_jobs SET handoff_digest=?1 WHERE id=?2",
        ["0".repeat(64), queued.id.clone()],
    )
    .unwrap();
    drop(conn);

    assert!(!poll(&fixture, &fixture.adapter).claimed);
    let blocked = fixture
        .store
        .task_sui_preflight_job(&fixture.project_id, &queued.id)
        .unwrap();
    assert_eq!(blocked.status, "blocked");
    assert_eq!(blocked.last_error.as_deref(), Some("handoff_digest_drift"));

    let requeued = queue(&fixture);
    assert_ne!(requeued.id, queued.id);
    assert_eq!(requeued.status, "pending");
    assert_ne!(requeued.handoff_digest, blocked.handoff_digest);
}

#[test]
fn conflicting_completion_preserves_original_report_and_projection_state() {
    let _runtime = RuntimeFlagGuard::enabled();
    let fixture = fixture();
    let projection_before =
        sui_projection_service::detail(&fixture.store, &fixture.project_id, &fixture.projection_id)
            .unwrap();
    let queued = queue(&fixture);
    let issue = claim(&fixture, &fixture.adapter, 300);
    let original = completion(issue.lease_token.clone(), "stable-completion-key");
    let completed =
        service::complete(&fixture.store, &fixture.adapter, &queued.id, &original).unwrap();
    let conflicting = CompleteSuiPreflightJobRequest {
        lease_token: issue.lease_token,
        outcome: "rejected".to_string(),
        summary: "conflicting result must not overwrite".to_string(),
        tool_version: "local-test-v2".to_string(),
        idempotency_key: original.idempotency_key,
    };
    assert!(
        service::complete(&fixture.store, &fixture.adapter, &queued.id, &conflicting,).is_err()
    );

    let persisted = fixture
        .store
        .task_sui_preflight_job(&fixture.project_id, &queued.id)
        .unwrap();
    let reports = sui_preflight_service::list_reports(&fixture.store, &fixture.project_id).unwrap();
    let projection_after =
        sui_projection_service::detail(&fixture.store, &fixture.project_id, &fixture.projection_id)
            .unwrap();
    assert_eq!(persisted.report_id, Some(completed.report.id));
    assert_eq!(reports.reports.len(), 1);
    assert_eq!(reports.reports[0].outcome, "passed");
    assert_eq!(
        projection_after.projection_digest,
        projection_before.projection_digest
    );
    assert_eq!(
        projection_after.integrity_status,
        projection_before.integrity_status
    );
    assert_eq!(
        projection_after.submission_readiness,
        projection_before.submission_readiness
    );
    assert_eq!(
        projection_after.network_submission,
        projection_before.network_submission
    );
    assert_eq!(
        projection_after.submission_attempts,
        projection_before.submission_attempts
    );
}

fn poll(
    fixture: &super::sui_preflight_job_test_support::SuiPreflightJobFixture,
    adapter: &super::sui_preflight_model::SuiPreflightAdapter,
) -> super::sui_preflight_job_model::SuiPreflightJobPoll {
    service::claim_next(
        &fixture.store,
        adapter,
        &ClaimSuiPreflightJobRequest { lease_seconds: 300 },
    )
    .unwrap()
}

fn claim(
    fixture: &super::sui_preflight_job_test_support::SuiPreflightJobFixture,
    adapter: &super::sui_preflight_model::SuiPreflightAdapter,
    lease_seconds: i64,
) -> super::sui_preflight_job_model::SuiPreflightJobIssue {
    service::claim_next(
        &fixture.store,
        adapter,
        &ClaimSuiPreflightJobRequest { lease_seconds },
    )
    .unwrap()
    .issue
    .unwrap()
}

fn create_adapter(
    fixture: &super::sui_preflight_job_test_support::SuiPreflightJobFixture,
    networks: &[&str],
    package_kinds: &[&str],
) -> super::sui_preflight_model::SuiPreflightAdapter {
    sui_preflight_service::create_adapter(
        &fixture.store,
        &fixture.project_id,
        &fixture.owner_user_id,
        "owner",
        &CreateSuiPreflightAdapterRequest {
            display_name: "Scoped local worker".to_string(),
            allowed_networks: networks.iter().map(|value| value.to_string()).collect(),
            allowed_package_kinds: package_kinds
                .iter()
                .map(|value| value.to_string())
                .collect(),
            expires_in_days: 30,
            confirmed_by_user: true,
        },
    )
    .unwrap()
    .adapter
}

fn completion(lease_token: String, idempotency_key: &str) -> CompleteSuiPreflightJobRequest {
    CompleteSuiPreflightJobRequest {
        lease_token,
        outcome: "passed".to_string(),
        summary: "offline package verified".to_string(),
        tool_version: "local-test-v1".to_string(),
        idempotency_key: idempotency_key.to_string(),
    }
}

fn queue(
    fixture: &super::sui_preflight_job_test_support::SuiPreflightJobFixture,
) -> super::sui_preflight_job_model::SuiPreflightJob {
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
