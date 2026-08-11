use std::path::{Path, PathBuf};

use uuid::Uuid;

use crate::{
    compute_federation::external_pool_adapter_release::{
        canonical_external_pool_adapter_release_capability_set_digest,
        ComputeExternalPoolAdapterReleaseCapability, ComputeExternalPoolAdapterReleaseIntent,
        ComputeExternalPoolAdapterReleaseVerifierIntent,
        COMPUTE_EXTERNAL_POOL_ADAPTER_RELEASE_CONFIRMATION,
        COMPUTE_EXTERNAL_POOL_ADAPTER_RELEASE_PROVIDER_KIND,
        COMPUTE_EXTERNAL_POOL_ADAPTER_RELEASE_ROUTE_KIND,
    },
    store::Store,
};

use super::types::{
    ApplyExternalPoolAdapterRelease, ReviewExternalPoolAdapterReleaseRequest,
    SubmitExternalPoolAdapterReleaseRequest, EXTERNAL_POOL_ADAPTER_RELEASE_APPLY_CONFIRMATION,
    EXTERNAL_POOL_ADAPTER_RELEASE_REVIEW_CONFIRMATION, REVIEW_DECISION_APPROVED,
    REVIEW_DECISION_CHANGES_REQUESTED,
};

const SUBMITTER: &str = "admin-release-submitter";
const REVIEWER: &str = "admin-release-reviewer";
const APPLIER: &str = "admin-release-applier";

fn temporary_store() -> (Store, PathBuf) {
    let path = std::env::temp_dir().join(format!(
        "elon_external_pool_adapter_release_{}.db",
        Uuid::new_v4().simple()
    ));
    (Store::open(&path).expect("store opens"), path)
}

fn remove_store_files(path: &Path) {
    for suffix in ["", "-wal", "-shm"] {
        let candidate = PathBuf::from(format!("{}{}", path.display(), suffix));
        let _ = std::fs::remove_file(candidate);
    }
}

fn release(version: &str) -> ComputeExternalPoolAdapterReleaseIntent {
    let capabilities = [
        "authenticated_ack",
        "authenticated_events",
        "cancel_no_start",
        "idempotent_commit",
        "prepare",
        "reconcile",
    ]
    .into_iter()
    .map(
        |capability_id| ComputeExternalPoolAdapterReleaseCapability {
            capability_id: capability_id.to_string(),
            capability_revision: 1,
        },
    )
    .collect::<Vec<_>>();
    let capability_set_digest =
        canonical_external_pool_adapter_release_capability_set_digest(&capabilities)
            .expect("capability digest");
    ComputeExternalPoolAdapterReleaseIntent {
        adapter_id: "community-external-pool".to_string(),
        release_version: version.to_string(),
        route_kind: COMPUTE_EXTERNAL_POOL_ADAPTER_RELEASE_ROUTE_KIND.to_string(),
        supported_provider_kinds: vec![
            COMPUTE_EXTERNAL_POOL_ADAPTER_RELEASE_PROVIDER_KIND.to_string()
        ],
        candidate_artifact_ref: format!("artifact-ref:community-pool-{version}"),
        declared_implementation_sha256: "1".repeat(64),
        supported_capabilities: capabilities,
        capability_set_digest,
        expected_credential_verifier: ComputeExternalPoolAdapterReleaseVerifierIntent {
            verification_kind: "signed_challenge".to_string(),
            verifier_id: "community-pool-verifier".to_string(),
            verifier_revision: 1,
            verifier_digest: "2".repeat(64),
        },
    }
}

fn submit_input(version: &str) -> SubmitExternalPoolAdapterReleaseRequest {
    SubmitExternalPoolAdapterReleaseRequest {
        submitted_by_admin_user_id: SUBMITTER.to_string(),
        release: release(version),
        idempotency_key: format!("submit-{version}"),
        confirmation: COMPUTE_EXTERNAL_POOL_ADAPTER_RELEASE_CONFIRMATION.to_string(),
        submission_note: "stage a reviewed external-pool Adapter candidate".to_string(),
        idempotency_scope: "external-pool-adapter-release-submit".to_string(),
    }
}

fn review_input(
    request_id: &str,
    request_digest: &str,
    material_digest: &str,
    decision: &str,
) -> ReviewExternalPoolAdapterReleaseRequest {
    ReviewExternalPoolAdapterReleaseRequest {
        request_id: request_id.to_string(),
        expected_request_digest: request_digest.to_string(),
        expected_request_material_digest: material_digest.to_string(),
        decision: decision.to_string(),
        review_confirmation: EXTERNAL_POOL_ADAPTER_RELEASE_REVIEW_CONFIRMATION.to_string(),
        review_note: (decision != REVIEW_DECISION_APPROVED)
            .then(|| "candidate must be revised before staging".to_string()),
        reviewed_by_admin_user_id: REVIEWER.to_string(),
        idempotency_scope: "external-pool-adapter-release-review".to_string(),
        idempotency_key: format!("review-{request_id}"),
    }
}

fn apply_input(
    request_id: &str,
    request_digest: &str,
    material_digest: &str,
    review_digest: &str,
) -> ApplyExternalPoolAdapterRelease {
    ApplyExternalPoolAdapterRelease {
        request_id: request_id.to_string(),
        expected_request_digest: request_digest.to_string(),
        expected_request_material_digest: material_digest.to_string(),
        expected_review_digest: review_digest.to_string(),
        applied_by_admin_user_id: APPLIER.to_string(),
        apply_confirmation: EXTERNAL_POOL_ADAPTER_RELEASE_APPLY_CONFIRMATION.to_string(),
        apply_note: "stage metadata only; execution remains disabled".to_string(),
        idempotency_scope: "external-pool-adapter-release-apply".to_string(),
        idempotency_key: format!("apply-{request_id}"),
    }
}

#[test]
fn approved_release_replays_exactly_after_database_reopen() {
    let (store, path) = temporary_store();
    let request = store
        .submit_external_pool_adapter_release_request(submit_input("1.0.0"))
        .expect("release request submits");
    assert_eq!(request.status, "submitted");
    assert!(!request.replayed);

    let request_replay = store
        .submit_external_pool_adapter_release_request(submit_input("1.0.0"))
        .expect("release request replays");
    assert_eq!(request_replay.request_id, request.request_id);
    assert!(request_replay.replayed);

    let review = store
        .review_external_pool_adapter_release_request(review_input(
            &request.request_id,
            &request.request_digest,
            &request.request_material_digest,
            REVIEW_DECISION_APPROVED,
        ))
        .expect("independent review approves");
    assert_eq!(review.reviewed_by_admin_user_id, REVIEWER);
    assert!(!review.replayed);

    let review_replay = store
        .review_external_pool_adapter_release_request(review_input(
            &request.request_id,
            &request.request_digest,
            &request.request_material_digest,
            REVIEW_DECISION_APPROVED,
        ))
        .expect("approval replays");
    assert_eq!(review_replay.review_id, review.review_id);
    assert!(review_replay.replayed);

    let admission = store
        .apply_external_pool_adapter_release(apply_input(
            &request.request_id,
            &request.request_digest,
            &request.request_material_digest,
            &review.review_digest,
        ))
        .expect("approved release stages");
    assert_eq!(admission.status, "staged");
    assert_eq!(admission.reviewed_by_admin_user_id, REVIEWER);
    assert_eq!(admission.release_effect, "staged_admission_only");
    assert!(!admission.replayed);

    let admission_replay = store
        .apply_external_pool_adapter_release(apply_input(
            &request.request_id,
            &request.request_digest,
            &request.request_material_digest,
            &review.review_digest,
        ))
        .expect("staging replays");
    assert_eq!(admission_replay.admission_id, admission.admission_id);
    assert!(admission_replay.replayed);
    drop(store);

    let reopened = Store::open(&path).expect("store reopens");
    let reopened_request = reopened
        .submit_external_pool_adapter_release_request(submit_input("1.0.0"))
        .expect("request history survives reopen");
    assert_eq!(reopened_request.status, "staged");
    assert!(reopened_request.replayed);
    let reopened_review = reopened
        .review_external_pool_adapter_release_request(review_input(
            &request.request_id,
            &request.request_digest,
            &request.request_material_digest,
            REVIEW_DECISION_APPROVED,
        ))
        .expect("review history survives reopen");
    assert_eq!(reopened_review.review_id, review.review_id);
    let reopened_admission = reopened
        .apply_external_pool_adapter_release(apply_input(
            &request.request_id,
            &request.request_digest,
            &request.request_material_digest,
            &review.review_digest,
        ))
        .expect("admission history survives reopen");
    assert_eq!(reopened_admission.admission_id, admission.admission_id);

    let duplicate_error = reopened
        .submit_external_pool_adapter_release_request(SubmitExternalPoolAdapterReleaseRequest {
            idempotency_key: "submit-duplicate-release".to_string(),
            ..submit_input("1.0.0")
        })
        .err()
        .expect("a second request cannot replace a staged release");
    assert!(duplicate_error.to_string().contains("already staged"));
    drop(reopened);
    remove_store_files(&path);
}

#[test]
fn four_eyes_and_non_approval_close_staging_paths() {
    let (store, path) = temporary_store();
    let request = store
        .submit_external_pool_adapter_release_request(submit_input("2.0.0"))
        .expect("release request submits");
    let mut same_actor = review_input(
        &request.request_id,
        &request.request_digest,
        &request.request_material_digest,
        REVIEW_DECISION_APPROVED,
    );
    same_actor.reviewed_by_admin_user_id = SUBMITTER.to_string();
    assert!(store
        .review_external_pool_adapter_release_request(same_actor)
        .err()
        .expect("submitter review must fail")
        .to_string()
        .contains("cannot review"));

    let mut wrong_confirmation = review_input(
        &request.request_id,
        &request.request_digest,
        &request.request_material_digest,
        REVIEW_DECISION_APPROVED,
    );
    wrong_confirmation.review_confirmation = "confirm-something-else".to_string();
    assert!(store
        .review_external_pool_adapter_release_request(wrong_confirmation)
        .err()
        .expect("wrong confirmation must fail")
        .to_string()
        .contains("confirmation is not exact"));

    let review = store
        .review_external_pool_adapter_release_request(review_input(
            &request.request_id,
            &request.request_digest,
            &request.request_material_digest,
            REVIEW_DECISION_CHANGES_REQUESTED,
        ))
        .expect("review closes with changes requested");
    assert_eq!(review.decision, REVIEW_DECISION_CHANGES_REQUESTED);
    assert!(store
        .apply_external_pool_adapter_release(apply_input(
            &request.request_id,
            &request.request_digest,
            &request.request_material_digest,
            &review.review_digest,
        ))
        .err()
        .expect("non-approved review cannot stage")
        .to_string()
        .contains("only the exact approved"));

    let mut conflicting_replay = review_input(
        &request.request_id,
        &request.request_digest,
        &request.request_material_digest,
        REVIEW_DECISION_CHANGES_REQUESTED,
    );
    conflicting_replay.review_note = Some("a different immutable note".to_string());
    assert!(store
        .review_external_pool_adapter_release_request(conflicting_replay)
        .err()
        .expect("changed replay must fail")
        .to_string()
        .contains("conflicts with immutable history"));

    drop(store);
    remove_store_files(&path);
}
