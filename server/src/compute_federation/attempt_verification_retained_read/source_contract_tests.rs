const ROOT: &str = include_str!("../attempt_verification_retained_read.rs");
const SERVICE: &str = include_str!("service.rs");
const API: &str = include_str!("api.rs");
const MCP: &str = include_str!("mcp.rs");
const STORE_FACADE: &str =
    include_str!("../../store/compute_federation_historical_causal_reference.rs");
const STORE_READ: &str =
    include_str!("../../store/compute_federation_historical_causal_reference/verification_read.rs");
const VERIFICATION_OWNER: &str = include_str!("../../store/compute_attempt_verifications.rs");
const RESERVATION_OWNER: &str = include_str!("../../store/compute_reservation_registry.rs");
const LEASE_OWNER: &str = include_str!("../../store/compute_attempt_leases.rs");
const ACTIVATION_ROWS: &str = include_str!("../../store/compute_attempt_activations/rows.rs");
const ATTEMPT_SERVICE: &str = include_str!("../../compute_federation_attempt_service.rs");
const ATTEMPT_API: &str = include_str!("../../compute_federation_attempt_api.rs");
const MCP_AGGREGATOR: &str = include_str!("../../compute_federation_mcp.rs");
const COMPUTE_MODULE: &str = include_str!("../mod.rs");

#[test]
fn read_reuses_the_native_exact_52_key_verification_receipt() {
    let receipt = declaration(
        VERIFICATION_OWNER,
        "pub(crate) struct ComputeAttemptVerificationDecisionReceipt",
    );
    let fields = receipt
        .lines()
        .filter(|line| line.trim_start().starts_with("pub ") && line.trim_end().ends_with(','))
        .map(|line| line.trim().split(':').next().unwrap())
        .collect::<Vec<_>>();
    assert_eq!(fields.len(), 52);
    assert_eq!(
        fields,
        [
            "pub schema",
            "pub verification_decision_id",
            "pub terminal_candidate_id",
            "pub terminal_candidate_event_digest",
            "pub consumer_review_id",
            "pub consumer_review_event_digest",
            "pub platform_observation_id",
            "pub platform_observation_event_digest",
            "pub lease_id",
            "pub provider_id",
            "pub consumer_account_id",
            "pub source_lease_revision",
            "pub source_lease_digest",
            "pub fencing_generation",
            "pub job_id",
            "pub job_revision",
            "pub job_digest",
            "pub reservation_id",
            "pub reservation_revision",
            "pub reservation_digest",
            "pub capacity_claim_id",
            "pub capacity_claim_revision",
            "pub capacity_claim_digest",
            "pub final_usage_snapshot_id",
            "pub final_usage_sequence_no",
            "pub final_provider_usage_digest",
            "pub platform_observed_usage_digest",
            "pub candidate_outcome",
            "pub consumer_decision",
            "pub observed_outcome",
            "pub policy_id",
            "pub policy_version",
            "pub decision",
            "pub reason_codes",
            "pub reason_codes_digest",
            "pub decision_ref",
            "pub verified_usage",
            "pub verified_usage_digest",
            "pub compensable_usage",
            "pub compensable_usage_digest",
            "pub request_digest",
            "pub event_digest",
            "pub decided_by_user_id",
            "pub decided_at",
            "pub verification_effect",
            "pub execution_receipt_effect",
            "pub lease_effect",
            "pub job_effect",
            "pub capacity_effect",
            "pub reservation_effect",
            "pub money_effect",
            "pub replayed",
        ]
    );
    assert!(SERVICE.contains("ComputeAttemptVerificationDecisionReceipt"));
    assert!(SERVICE.contains("Ok(retained.into_receipt())"));
    assert!(!ROOT.contains("struct ComputeAttemptVerificationDecisionReceipt"));
    assert!(!SERVICE.contains("#[derive(Serialize"));
    assert!(!ATTEMPT_SERVICE.contains("fn get_verification_for_participant("));
    assert!(!VERIFICATION_OWNER.contains("pub(crate) fn compute_attempt_verification_decision("));
}

#[test]
fn store_uses_one_deferred_historical_v192_read_and_private_scope() {
    let facade = between(
        STORE_FACADE,
        "pub(crate) fn resolve_compute_attempt_retained_verification(",
        "pub(crate) fn resolve_compute_execution_source_lineage(",
    );
    assert!(facade.contains("TransactionBehavior::Deferred"));
    assert_eq!(
        facade
            .matches("compute_attempt_historical_verification_decision_on")
            .count(),
        1
    );
    assert!(facade.contains("verification_read::validate_retained_verification_on"));
    assert!(!facade.contains("execution_receipt"));
    assert!(facade.contains("ComputeAttemptRetainedVerificationResolveError"));
    assert!(facade.contains(".conn()"));
    assert!(facade.contains("ComputeAttemptRetainedVerificationResolveError::operational"));
    assert!(facade.contains("classify_retained_verification_owner_error"));

    let job = STORE_READ
        .find("registered_historical_job_version_on")
        .unwrap();
    let offer = STORE_READ
        .find("registered_historical_offer_version_on")
        .unwrap();
    let provider = STORE_READ.find("registered_provider_version_on").unwrap();
    assert!(job < offer && offer < provider);
    for binding in [
        "job.job_digest != receipt.job_digest",
        "job.job.consumer_account_id != receipt.consumer_account_id",
        "offer.offer.offer_digest != selected_offer.offer_digest",
        "selected_offer.provider_id != receipt.provider_id",
        "provider.provider.provider_id != receipt.provider_id",
        "provider.provider_digest != offer.provider_digest",
    ] {
        assert!(
            STORE_READ.contains(binding),
            "missing audit binding {binding}"
        );
    }
    assert!(STORE_READ.contains("job.job.project_id.as_deref()"));
    assert!(STORE_READ.contains("provider.provider.owner_account_id"));
    for forbidden in ["v193", "execution_receipt", "execution_source_lineage"] {
        assert!(!STORE_READ.contains(forbidden));
    }

    for helper in [
        "compute_attempt_historical_terminal_candidate_on",
        "registered_historical_job_version_on",
        "registered_historical_offer_version_on",
        "registered_provider_version_on",
        "registered_historical_reservation_version_on",
        "stored_claim_version_on",
        "audited_compute_attempt_lease_version_on",
    ] {
        assert!(
            STORE_READ.contains(helper),
            "missing historical helper {helper}"
        );
    }
    for binding in [
        "receipt.terminal_candidate_id != candidate.terminal_candidate_id",
        "receipt.consumer_account_id != candidate.consumer_account_id",
        "receipt.source_lease_digest != candidate.source_lease_digest",
        "reservation_body.job.job_digest != receipt.job_digest",
        "reservation_body.offer.offer_digest != selected_offer.offer_digest",
        "reservation_body.capacity_claim.claim_digest != receipt.capacity_claim_digest",
        "claim.subject_id != reservation_body.reservation_id",
        "source_lease.lease_digest != receipt.source_lease_digest",
        "source_lease.lease.fencing_generation != receipt.fencing_generation",
        "source_lease.lease.job_id != receipt.job_id",
        "source_lease.lease.reservation_id != receipt.reservation_id",
        "source_lease.lease.provider_id != receipt.provider_id",
        "source_lease.lease.status != ATTEMPT_STATUS_RUNNING",
        "source_lease.lease.last_heartbeat_at.is_none()",
        "source_lease.consumer_account_id != receipt.consumer_account_id",
    ] {
        assert!(
            STORE_READ.contains(binding),
            "missing owner splice guard {binding}"
        );
    }
    assert!(RESERVATION_OWNER.contains("registered_historical_reservation_version_on"));
    assert!(LEASE_OWNER.contains("compute_attempt_historical_activation_sources_on"));
    assert!(LEASE_OWNER.contains("pub(in crate::store) consumer_account_id: String"));
    assert_eq!(
        LEASE_OWNER
            .matches("consumer_account_id: running_job.job.consumer_account_id.clone()")
            .count(),
        2
    );
    assert!(
        LEASE_OWNER.contains("renewal.consumer_account_id != running_job.job.consumer_account_id")
    );
    assert!(ACTIVATION_ROWS.contains("job.job.consumer_account_id != stored.consumer_account_id"));

    for marker in [
        "pub(crate) struct ValidatedComputeAttemptRetainedVerification",
        "struct ComputeAttemptRetainedVerificationAccessScope",
    ] {
        let start = STORE_FACADE
            .find(marker)
            .expect("private Store type must exist");
        let prefix = STORE_FACADE[..start]
            .lines()
            .rev()
            .take(4)
            .collect::<Vec<_>>()
            .join("\n");
        assert!(!prefix.contains("#[derive"), "Store type gained derives");
    }
    for forbidden in [
        "impl Clone for ValidatedComputeAttemptRetainedVerification",
        "Serialize for ValidatedComputeAttemptRetainedVerification",
        "Deserialize for ValidatedComputeAttemptRetainedVerification",
        "impl Clone for ComputeAttemptRetainedVerificationAccessScope",
        "Serialize for ComputeAttemptRetainedVerificationAccessScope",
        "Deserialize for ComputeAttemptRetainedVerificationAccessScope",
    ] {
        assert!(!STORE_FACADE.contains(forbidden));
    }
}

#[test]
fn typed_store_errors_keep_operational_failures_out_of_redacted_integrity_paths() {
    let error = between(
        STORE_FACADE,
        "pub(crate) enum ComputeAttemptRetainedVerificationResolveError",
        "struct ComputeAttemptRetainedVerificationAccessScope",
    );
    assert!(error.contains("Integrity { source: AnyhowError }"));
    assert!(error.contains("Operational { source: AnyhowError }"));
    assert!(!error.contains("#[derive(Clone"));
    assert!(!error.contains("Serialize"));
    assert!(!error.contains("Deserialize"));

    let classifier = STORE_FACADE
        .split("fn classify_retained_verification_owner_error(")
        .nth(1)
        .expect("typed owner classifier must exist");
    assert!(classifier.contains("downcast_ref::<rusqlite::Error>()"));
    assert!(classifier.contains("rusqlite::Error::QueryReturnedNoRows"));
    assert!(classifier.contains("rusqlite::Error::FromSqlConversionFailure"));
    assert!(classifier.contains("rusqlite::Error::IntegralValueOutOfRange"));
    assert!(classifier.contains("rusqlite::Error::Utf8Error"));
    assert!(classifier.contains("rusqlite::Error::InvalidColumnType"));
    assert!(classifier.contains("ComputeAttemptRetainedVerificationResolveError::operational"));
    assert!(classifier.contains("ComputeAttemptRetainedVerificationResolveError::integrity"));
    for forbidden in ["to_string()", "contains(\"", "format!("] {
        assert!(
            !classifier.contains(forbidden),
            "classifier uses error text"
        );
    }

    assert!(SERVICE.contains("fn participant_resolve_error("));
    assert!(SERVICE.contains("fn admin_resolve_error("));
    assert!(SERVICE.contains("ComputeAttemptRetainedVerificationResolveError::Operational"));
    assert!(SERVICE.contains("AttemptVerificationRetainedReadError::Unavailable"));
    assert!(SERVICE.contains("ATTEMPT_VERIFICATION_RETAINED_INTERNAL_ERROR"));
    assert!(API.contains("StatusCode::INTERNAL_SERVER_ERROR"));
    assert!(MCP.contains("redacted_service_error"));
}

#[test]
fn http_preserves_existing_paths_with_path_only_redacted_reads() {
    for route in [
        "/api/me/compute/attempt-leases/:lease_id/verification-decision",
        "/api/admin/compute/attempt-leases/:lease_id/verification-decision",
    ] {
        assert_eq!(
            ATTEMPT_API.matches(route).count(),
            1,
            "route drift: {route}"
        );
    }
    assert!(ATTEMPT_API.contains(
        "get(attempt_verification_retained_read::get_for_admin).post(decide_verification)"
    ));
    assert!(ATTEMPT_API.contains("get(attempt_verification_retained_read::get_for_participant)"));
    assert!(API.contains("uri.query().is_some()"));
    assert!(API.contains("to_bytes(body, 1)"));
    assert!(API.contains("Path::<String>::from_request_parts(parts, state)"));
    assert!(!API.contains("Path(lease_id): Path<String>"));

    let participant = between(
        SERVICE,
        "pub(super) fn read_for_participant(",
        "pub(super) fn read_for_admin(",
    );
    assert_eq!(
        participant
            .matches("AttemptVerificationRetainedReadError::NotVisible")
            .count(),
        2
    );
    assert!(participant.contains("retained.belongs_to_project(project_id)"));
    let admin = between(
        SERVICE,
        "pub(super) fn read_for_admin(",
        "fn validate_lease_id(",
    );
    assert!(admin.contains("map_err(admin_resolve_error)"));
    assert!(admin.contains("AttemptVerificationRetainedReadError::NotFound"));
    let response = between(API, "fn retained_response<", "fn coded_error(");
    assert!(response.contains("StatusCode::NOT_FOUND"));
    assert!(response.contains("StatusCode::CONFLICT"));
    assert!(response.contains("StatusCode::INTERNAL_SERVER_ERROR"));

    for code in [
        "ATTEMPT_VERIFICATION_RETAINED_INVALID_LEASE_ID",
        "ATTEMPT_VERIFICATION_RETAINED_INVALID_REQUEST_INPUT",
        "ATTEMPT_VERIFICATION_RETAINED_UNAUTHENTICATED",
        "ATTEMPT_VERIFICATION_RETAINED_NOT_VISIBLE",
        "ATTEMPT_VERIFICATION_RETAINED_PROJECT_FORBIDDEN",
        "ATTEMPT_VERIFICATION_RETAINED_NOT_FOUND",
        "ATTEMPT_VERIFICATION_RETAINED_INTEGRITY_CONFLICT",
        "ATTEMPT_VERIFICATION_RETAINED_ADMIN_FORBIDDEN",
        "ATTEMPT_VERIFICATION_RETAINED_INTERNAL_ERROR",
    ] {
        assert!(SERVICE.contains(code), "missing stable error code {code}");
    }
}

#[test]
fn mcp_is_exact_two_tools_and_enforces_project_isolation() {
    for tool in [
        "compute_get_my_attempt_verification_decision",
        "compute_admin_get_attempt_verification_decision",
    ] {
        assert_eq!(MCP.matches(tool).count(), 1, "tool ABI drift: {tool}");
    }
    assert_eq!(MCP.matches("support::tool(").count(), 2);
    assert!(MCP.contains("Some(project_id)"));
    assert!(MCP.contains("ensure_platform_admin(platform_role)?"));
    let schema = MCP.split("fn lease_schema()").nth(1).unwrap();
    assert!(schema.contains("required\":[\"lease_id\"]"));
    assert!(schema.contains("properties\":{\"lease_id\""));
    assert!(schema.contains("additionalProperties\":false"));
    for forbidden in ["user_id", "project_id", "receipt_id", "digest"] {
        assert!(!schema.contains(forbidden), "MCP input exposes {forbidden}");
    }
    for wiring in [
        "attempt_verification_retained_read::definitions()",
        "attempt_verification_retained_read::admin_definitions()",
        "attempt_verification_retained_read::call_if_handled(",
        "attempt_verification_retained_read::call_admin_if_handled(",
    ] {
        assert!(MCP_AGGREGATOR.contains(wiring), "missing aggregator wiring");
    }
    assert!(COMPUTE_MODULE.contains("pub(crate) mod attempt_verification_retained_read;"));
}

#[test]
fn adoption_has_no_writer_projection_or_clock_effect() {
    let adoption = [ROOT, SERVICE, API, MCP, STORE_READ].join("\n");
    let lower = adoption.to_ascii_lowercase();
    for forbidden in [
        "transactionbehavior::immediate",
        ".execute(",
        ".execute_batch(",
        "insert into",
        "update ",
        "delete from",
        "create table",
        "migration",
        "utc::now",
        "new_id",
        "current_",
        "latest",
    ] {
        assert!(
            !lower.contains(forbidden),
            "read adoption contains {forbidden}"
        );
    }
}

fn declaration<'a>(source: &'a str, marker: &str) -> &'a str {
    let start = source.find(marker).expect("declaration marker must exist");
    let tail = &source[start..];
    let end = tail.find("\n}").expect("declaration must be bounded");
    &tail[..end]
}

fn between<'a>(source: &'a str, start_marker: &str, end_marker: &str) -> &'a str {
    let start = source
        .find(start_marker)
        .expect("source block start marker must exist");
    let tail = &source[start..];
    let end = tail
        .find(end_marker)
        .expect("source block end marker must exist");
    &tail[..end]
}
