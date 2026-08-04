use anyhow::{bail, Context, Result};
use chrono::{DateTime, Duration, Utc};
use serde::Serialize;
use sha2::{Digest, Sha256};

use super::AbortComputeAttemptRequest;

const ABORT_REQUEST_SCHEMA: &str = "compute_federation.attempt_abort_request.v1";

#[derive(Debug, Clone)]
pub(super) struct NormalizedAttemptAbort {
    pub lease_id: String,
    pub provider_id: String,
    pub expected_lease_revision: i64,
    pub expected_lease_digest: String,
    pub expected_fencing_generation: i64,
    pub expected_job_revision: i64,
    pub expected_job_digest: String,
    pub expected_reservation_revision: i64,
    pub expected_reservation_digest: String,
    pub expected_claim_revision: i64,
    pub expected_claim_digest: String,
    pub executor_abort_ref: String,
    pub reason_code: String,
    pub idempotency_scope: String,
    pub idempotency_key: String,
    pub aborted_by_user_id: String,
    pub request_digest: String,
}

#[derive(Serialize)]
struct CanonicalAbortRequest<'a> {
    schema: &'static str,
    lease_id: &'a str,
    provider_id: &'a str,
    expected_lease_revision: i64,
    expected_lease_digest: &'a str,
    expected_fencing_generation: i64,
    expected_job_revision: i64,
    expected_job_digest: &'a str,
    expected_reservation_revision: i64,
    expected_reservation_digest: &'a str,
    expected_claim_revision: i64,
    expected_claim_digest: &'a str,
    executor_abort_ref: &'a str,
    reason_code: &'a str,
    idempotency_key: &'a str,
    aborted_by_user_id: &'a str,
}

pub(super) fn normalize_abort_request(
    request: &AbortComputeAttemptRequest,
) -> Result<NormalizedAttemptAbort> {
    let lease_id = required("Attempt Lease ID", &request.lease_id, 200)?;
    let provider_id = required("Provider ID", &request.provider_id, 200)?;
    let expected_lease_digest = digest("Attempt Lease 摘要", &request.expected_lease_digest)?;
    let expected_job_digest = digest("Job 摘要", &request.expected_job_digest)?;
    let expected_reservation_digest =
        digest("Reservation 摘要", &request.expected_reservation_digest)?;
    let expected_claim_digest = digest("Capacity Claim 摘要", &request.expected_claim_digest)?;
    let executor_abort_ref = required("外部执行器中止凭据引用", &request.executor_abort_ref, 500)?;
    let reason_code = required("Attempt 中止原因码", &request.reason_code, 160)?;
    let idempotency_key = required("Attempt 中止幂等键", &request.idempotency_key, 200)?;
    let aborted_by_user_id = required("Attempt 中止执行人", &request.aborted_by_user_id, 200)?;
    for (label, value) in [
        ("expected_lease_revision", request.expected_lease_revision),
        (
            "expected_fencing_generation",
            request.expected_fencing_generation,
        ),
        ("expected_job_revision", request.expected_job_revision),
        (
            "expected_reservation_revision",
            request.expected_reservation_revision,
        ),
        ("expected_claim_revision", request.expected_claim_revision),
    ] {
        if value <= 0 {
            bail!("{label} 必须为正整数");
        }
    }
    let canonical = CanonicalAbortRequest {
        schema: ABORT_REQUEST_SCHEMA,
        lease_id: &lease_id,
        provider_id: &provider_id,
        expected_lease_revision: request.expected_lease_revision,
        expected_lease_digest: &expected_lease_digest,
        expected_fencing_generation: request.expected_fencing_generation,
        expected_job_revision: request.expected_job_revision,
        expected_job_digest: &expected_job_digest,
        expected_reservation_revision: request.expected_reservation_revision,
        expected_reservation_digest: &expected_reservation_digest,
        expected_claim_revision: request.expected_claim_revision,
        expected_claim_digest: &expected_claim_digest,
        executor_abort_ref: &executor_abort_ref,
        reason_code: &reason_code,
        idempotency_key: &idempotency_key,
        aborted_by_user_id: &aborted_by_user_id,
    };
    let request_digest = hex::encode(Sha256::digest(serde_json::to_vec(&canonical)?));
    Ok(NormalizedAttemptAbort {
        lease_id,
        provider_id: provider_id.clone(),
        expected_lease_revision: request.expected_lease_revision,
        expected_lease_digest,
        expected_fencing_generation: request.expected_fencing_generation,
        expected_job_revision: request.expected_job_revision,
        expected_job_digest,
        expected_reservation_revision: request.expected_reservation_revision,
        expected_reservation_digest,
        expected_claim_revision: request.expected_claim_revision,
        expected_claim_digest,
        executor_abort_ref,
        reason_code,
        idempotency_scope: format!("compute_attempt_abort:{provider_id}"),
        idempotency_key,
        aborted_by_user_id,
        request_digest,
    })
}

pub(super) fn abort_timestamp(
    job_updated_at: &str,
    reservation_updated_at: &str,
    lease_updated_at: &str,
    job_deadline_at: &str,
) -> Result<String> {
    let floor = [job_updated_at, reservation_updated_at, lease_updated_at]
        .into_iter()
        .map(parse_utc)
        .collect::<Result<Vec<_>>>()?
        .into_iter()
        .max()
        .context("Attempt 中止缺少时间基线")?
        .checked_add_signed(Duration::nanoseconds(1))
        .context("Attempt 中止时间溢出")?;
    let aborted_at = std::cmp::max(Utc::now(), floor);
    if aborted_at >= parse_utc(job_deadline_at)? {
        bail!("Job 已到达截止时间，不能再走 staging 无用量中止路径");
    }
    Ok(aborted_at.to_rfc3339())
}

fn parse_utc(value: &str) -> Result<DateTime<Utc>> {
    let parsed =
        DateTime::parse_from_rfc3339(value).context("Attempt 中止引用的时间不是 RFC3339")?;
    Ok(parsed.with_timezone(&Utc))
}

fn required(label: &str, value: &str, max_len: usize) -> Result<String> {
    let normalized = value.trim();
    if normalized.is_empty()
        || normalized.len() > max_len
        || normalized.chars().any(char::is_control)
    {
        bail!("{label} 无效");
    }
    Ok(normalized.to_string())
}

fn digest(label: &str, value: &str) -> Result<String> {
    let normalized = required(label, value, 64)?;
    if normalized.len() != 64 || !normalized.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        bail!("{label} 必须为 64 位十六进制摘要");
    }
    Ok(normalized.to_ascii_lowercase())
}
