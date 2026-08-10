use std::sync::{Arc, Barrier};

use crate::store::Store;

use super::{
    sui_preflight_job_model::{
        ClaimSuiPreflightJobRequest, CompleteSuiPreflightJobRequest, QueueSuiPreflightJobRequest,
        ReleaseSuiPreflightJobRequest, RenewSuiPreflightJobRequest,
    },
    sui_preflight_job_service as service,
    sui_preflight_job_test_support::{fixture, RuntimeFlagGuard},
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
