use anyhow::{anyhow, bail, Result};
use chrono::{DateTime, Utc};
use sha2::{Digest, Sha256};

use super::ActivateComputeAttemptRequest;

pub(super) struct NormalizedAttemptActivation {
    pub lease_id: String,
    pub reservation_id: String,
    pub provider_id: String,
    pub executor_id: String,
    pub shard_id: Option<String>,
    pub attempt_no: i64,
    pub fencing_generation: i64,
    pub executor_acceptance_ref: String,
    pub lease_credential_ref: String,
    pub lease_credential_hint: String,
    pub expected_job_revision: i64,
    pub expected_job_digest: String,
    pub expected_reservation_revision: i64,
    pub expected_reservation_digest: String,
    pub expected_claim_revision: i64,
    pub expected_claim_digest: String,
    pub expires_at: String,
    pub hard_deadline_at: String,
    pub idempotency_key: String,
    pub activated_by_user_id: String,
    pub request_digest: String,
}

pub(super) fn normalize_activation(
    request: &ActivateComputeAttemptRequest,
) -> Result<NormalizedAttemptActivation> {
    let lease_id = exact("Attempt Lease ID", &request.lease_id, 160)?;
    let reservation_id = exact("Reservation ID", &request.reservation_id, 160)?;
    let provider_id = exact("Provider ID", &request.provider_id, 160)?;
    let executor_id = exact("Executor ID", &request.executor_id, 160)?;
    let shard_id = optional_exact("Shard ID", request.shard_id.as_deref(), 160)?;
    let executor_acceptance_ref =
        exact("执行器接受证明引用", &request.executor_acceptance_ref, 512)?;
    let lease_credential_ref = exact("Lease 凭据引用", &request.lease_credential_ref, 512)?;
    let lease_credential_hint = exact("Lease 凭据提示", &request.lease_credential_hint, 160)?;
    let expected_job_digest = digest("Job digest", &request.expected_job_digest)?;
    let expected_reservation_digest =
        digest("Reservation digest", &request.expected_reservation_digest)?;
    let expected_claim_digest = digest("Capacity Claim digest", &request.expected_claim_digest)?;
    let expires_at = utc("Lease expires_at", &request.expires_at)?;
    let hard_deadline_at = utc("Lease hard_deadline_at", &request.hard_deadline_at)?;
    let idempotency_key = exact("Attempt 激活幂等键", &request.idempotency_key, 160)?;
    let activated_by_user_id = exact("Attempt 激活操作者", &request.activated_by_user_id, 160)?;
    if request.attempt_no != 1 || request.fencing_generation != 1 {
        bail!("当前入口只支持首次 Attempt，attempt_no 与 fencing_generation 必须为 1");
    }
    if request.expected_job_revision <= 0
        || request.expected_reservation_revision <= 0
        || request.expected_claim_revision <= 0
    {
        bail!("Attempt 激活的 expected revision 必须为正整数");
    }
    if parse_utc(&expires_at)? >= parse_utc(&hard_deadline_at)? {
        bail!("Lease hard_deadline_at 必须晚于 expires_at");
    }
    let request_digest = hex::encode(Sha256::digest(serde_json::to_vec(&serde_json::json!({
        "schema":"compute_federation.attempt_activation_request.v1",
        "lease_id":lease_id,
        "reservation_id":reservation_id,
        "provider_id":provider_id,
        "executor_id":executor_id,
        "shard_id":shard_id,
        "attempt_no":request.attempt_no,
        "fencing_generation":request.fencing_generation,
        "executor_acceptance_ref":executor_acceptance_ref,
        "lease_credential_ref":lease_credential_ref,
        "lease_credential_hint":lease_credential_hint,
        "expected_job_revision":request.expected_job_revision,
        "expected_job_digest":expected_job_digest,
        "expected_reservation_revision":request.expected_reservation_revision,
        "expected_reservation_digest":expected_reservation_digest,
        "expected_claim_revision":request.expected_claim_revision,
        "expected_claim_digest":expected_claim_digest,
        "expires_at":expires_at,
        "hard_deadline_at":hard_deadline_at,
        "idempotency_key":idempotency_key,
        "activated_by_user_id":activated_by_user_id,
    }))?));
    Ok(NormalizedAttemptActivation {
        lease_id,
        reservation_id,
        provider_id,
        executor_id,
        shard_id,
        attempt_no: request.attempt_no,
        fencing_generation: request.fencing_generation,
        executor_acceptance_ref,
        lease_credential_ref,
        lease_credential_hint,
        expected_job_revision: request.expected_job_revision,
        expected_job_digest,
        expected_reservation_revision: request.expected_reservation_revision,
        expected_reservation_digest,
        expected_claim_revision: request.expected_claim_revision,
        expected_claim_digest,
        expires_at,
        hard_deadline_at,
        idempotency_key,
        activated_by_user_id,
        request_digest,
    })
}

pub(super) fn parse_utc(value: &str) -> Result<DateTime<Utc>> {
    let value = DateTime::parse_from_rfc3339(value).map_err(|_| anyhow!("时间必须是 RFC3339"))?;
    if value.offset().local_minus_utc() != 0 {
        bail!("时间必须使用 UTC 时区");
    }
    Ok(value.with_timezone(&Utc))
}

fn utc(label: &str, value: &str) -> Result<String> {
    let value = exact(label, value, 64)?;
    parse_utc(&value)?;
    Ok(value)
}

fn digest(label: &str, value: &str) -> Result<String> {
    let value = exact(label, value, 64)?;
    if value.len() != 64 || !value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        bail!("{label} 必须是 64 位十六进制摘要");
    }
    Ok(value.to_ascii_lowercase())
}

fn optional_exact(label: &str, value: Option<&str>, max: usize) -> Result<Option<String>> {
    value.map(|value| exact(label, value, max)).transpose()
}

fn exact(label: &str, value: &str, max: usize) -> Result<String> {
    if value.is_empty() || value.len() > max || value.trim() != value {
        bail!("{label} 不能为空、不能包含首尾空白且长度不能超过 {max}");
    }
    Ok(value.to_string())
}
