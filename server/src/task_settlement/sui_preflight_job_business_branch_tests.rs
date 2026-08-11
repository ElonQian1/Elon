use crate::store::Store;

use super::{
    dispute_service,
    model::OpenSettlementDisputeRequest,
    sui_correction_projection_service,
    sui_correction_projection_test_support::fixture as correction_fixture,
    sui_preflight_job_model::{
        ClaimSuiPreflightJobRequest, CompleteSuiPreflightJobRequest, QueueSuiPreflightJobRequest,
        SuiPreflightJob,
    },
    sui_preflight_job_service as service,
    sui_preflight_job_test_support::{fixture as standard_fixture, RuntimeFlagGuard},
    sui_preflight_model::{CreateSuiPreflightAdapterRequest, SuiPreflightAdapter},
    sui_preflight_service,
};

#[test]
fn open_dispute_blocks_existing_and_future_standard_preflight_job() {
    let _runtime = RuntimeFlagGuard::enabled();
    let fixture = standard_fixture();
    let queued = queue_job(
        &fixture.store,
        &fixture.project_id,
        &fixture.owner_user_id,
        "standard",
        &fixture.projection_id,
    );

    dispute_service::open(
        &fixture.store,
        &fixture.project_id,
        &fixture.receipt_id,
        &fixture.owner_user_id,
        &dispute_request("标准投影已经入队后出现争议"),
    )
    .unwrap();

    let poll = claim_next(&fixture.store, &fixture.adapter);
    assert!(!poll.claimed);
    assert!(poll.issue.is_none());

    let jobs = service::list(&fixture.store, &fixture.project_id).unwrap();
    assert_eq!(jobs.jobs.len(), 1);
    assert_eq!(jobs.jobs[0].id, queued.id);
    assert_eq!(jobs.jobs[0].status, "blocked");
    assert!(jobs.jobs[0]
        .last_error
        .as_deref()
        .unwrap()
        .contains("handoff_unavailable"));

    assert!(service::queue(
        &fixture.store,
        &fixture.project_id,
        &fixture.owner_user_id,
        "owner",
        &QueueSuiPreflightJobRequest {
            package_kind: "standard".to_string(),
            projection_package_id: fixture.projection_id,
            confirmed_by_user: true,
        },
    )
    .is_err());
}

#[test]
fn posted_correction_package_can_be_claimed_and_completed_atomically() {
    let _runtime = RuntimeFlagGuard::enabled();
    let fixture = correction_fixture(true);
    let package = sui_correction_projection_service::prepare(
        &fixture.store,
        &fixture.project_id,
        &fixture.correction.correction.id,
        &fixture.user_id,
        "testnet",
    )
    .unwrap();
    let standard_only = create_adapter(
        &fixture.store,
        &fixture.project_id,
        &fixture.user_id,
        &["testnet"],
        &["standard"],
    );
    let correction_adapter = create_adapter(
        &fixture.store,
        &fixture.project_id,
        &fixture.user_id,
        &["testnet"],
        &["correction"],
    );
    let queued = queue_job(
        &fixture.store,
        &fixture.project_id,
        &fixture.user_id,
        "correction",
        &package.id,
    );

    let wrong_scope = claim_next(&fixture.store, &standard_only);
    assert!(!wrong_scope.claimed);

    let issue = claim_next(&fixture.store, &correction_adapter)
        .issue
        .unwrap();
    assert_eq!(issue.job.id, queued.id);
    assert_eq!(issue.handoff.payload.package_kind, "correction");
    assert!(issue.handoff.payload.atomic_bundle);
    assert!(!issue.handoff.payload.constraints.signature_present);
    assert!(!issue.handoff.payload.constraints.transaction_broadcast);
    assert!(!issue.handoff.payload.constraints.funds_moved);
    assert!(issue.handoff.payload.envelope.get("reversal").is_some());
    assert!(issue.handoff.payload.envelope.get("replacement").is_some());

    let completed = service::complete(
        &fixture.store,
        &correction_adapter,
        &queued.id,
        &completion(issue.lease_token, "correction-preflight-complete"),
    )
    .unwrap();
    assert_eq!(completed.job.status, "completed");
    assert_eq!(completed.report.package_kind, "correction");
    assert_eq!(completed.report.projection_package_id, package.id);

    let unchanged = sui_correction_projection_service::detail(
        &fixture.store,
        &fixture.project_id,
        &completed.report.projection_package_id,
    )
    .unwrap();
    assert_eq!(unchanged.network_submission, "not_submitted");
    assert_eq!(unchanged.submission_attempts, 0);
}

#[test]
fn replacement_dispute_blocks_queued_correction_job_and_future_queue() {
    let _runtime = RuntimeFlagGuard::enabled();
    let fixture = correction_fixture(true);
    let package = sui_correction_projection_service::prepare(
        &fixture.store,
        &fixture.project_id,
        &fixture.correction.correction.id,
        &fixture.user_id,
        "testnet",
    )
    .unwrap();
    let adapter = create_adapter(
        &fixture.store,
        &fixture.project_id,
        &fixture.user_id,
        &["testnet"],
        &["correction"],
    );
    let queued = queue_job(
        &fixture.store,
        &fixture.project_id,
        &fixture.user_id,
        "correction",
        &package.id,
    );

    dispute_service::open(
        &fixture.store,
        &fixture.project_id,
        &package.replacement_receipt_id,
        &fixture.user_id,
        &dispute_request("替换凭证在预检领取前出现新争议"),
    )
    .unwrap();

    let poll = claim_next(&fixture.store, &adapter);
    assert!(!poll.claimed);
    let blocked = service::list(&fixture.store, &fixture.project_id)
        .unwrap()
        .jobs
        .into_iter()
        .find(|job| job.id == queued.id)
        .unwrap();
    assert_eq!(blocked.status, "blocked");
    assert!(blocked
        .last_error
        .as_deref()
        .unwrap()
        .contains("handoff_unavailable"));

    assert!(service::queue(
        &fixture.store,
        &fixture.project_id,
        &fixture.user_id,
        "owner",
        &QueueSuiPreflightJobRequest {
            package_kind: "correction".to_string(),
            projection_package_id: package.id,
            confirmed_by_user: true,
        },
    )
    .is_err());
}

fn dispute_request(summary: &str) -> OpenSettlementDisputeRequest {
    OpenSettlementDisputeRequest {
        reason_code: "amount".to_string(),
        summary: summary.to_string(),
        evidence_ref: Some("artifact:sui-preflight-business-branch".to_string()),
    }
}

fn create_adapter(
    store: &Store,
    project_id: &str,
    user_id: &str,
    networks: &[&str],
    package_kinds: &[&str],
) -> SuiPreflightAdapter {
    sui_preflight_service::create_adapter(
        store,
        project_id,
        user_id,
        "owner",
        &CreateSuiPreflightAdapterRequest {
            display_name: "Business branch worker".to_string(),
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

fn queue_job(
    store: &Store,
    project_id: &str,
    user_id: &str,
    package_kind: &str,
    projection_package_id: &str,
) -> SuiPreflightJob {
    service::queue(
        store,
        project_id,
        user_id,
        "owner",
        &QueueSuiPreflightJobRequest {
            package_kind: package_kind.to_string(),
            projection_package_id: projection_package_id.to_string(),
            confirmed_by_user: true,
        },
    )
    .unwrap()
}

fn claim_next(
    store: &Store,
    adapter: &SuiPreflightAdapter,
) -> super::sui_preflight_job_model::SuiPreflightJobPoll {
    service::claim_next(
        store,
        adapter,
        &ClaimSuiPreflightJobRequest { lease_seconds: 300 },
    )
    .unwrap()
}

fn completion(lease_token: String, idempotency_key: &str) -> CompleteSuiPreflightJobRequest {
    CompleteSuiPreflightJobRequest {
        lease_token,
        outcome: "passed".to_string(),
        summary: "offline correction bundle verified".to_string(),
        tool_version: "business-branch-test-v1".to_string(),
        idempotency_key: idempotency_key.to_string(),
    }
}
