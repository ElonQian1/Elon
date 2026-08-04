use std::collections::BTreeSet;

use anyhow::{anyhow, bail, Context, Result};
use chrono::{DateTime, Duration, Utc};
use serde::Serialize;
use sha2::{Digest, Sha256};

use crate::compute_federation::execution::ComputeReservedCapacity;

use super::ReserveComputeBrokerRequest;

const BROKER_RESERVE_REQUEST_SCHEMA: &str = "compute_federation.broker_reserve_request.v1";
const MICROS_PER_CNY_FEN: i64 = 10_000;

#[derive(Debug, Clone)]
pub(super) struct NormalizedBrokerReserveRequest {
    pub reservation_id: String,
    pub consumer_account_id: String,
    pub idempotency_key: String,
    pub job_id: String,
    pub expected_job_revision: i64,
    pub expected_job_digest: String,
    pub reserved_capacity: Vec<ComputeReservedCapacity>,
    pub expires_at: String,
    pub request_digest: String,
}

#[derive(Serialize)]
struct CanonicalBrokerReserveRequest<'a> {
    schema: &'static str,
    reservation_id: &'a str,
    consumer_account_id: &'a str,
    idempotency_key: &'a str,
    job_id: &'a str,
    expected_job_revision: i64,
    expected_job_digest: &'a str,
    reserved_capacity: &'a [ComputeReservedCapacity],
    expires_at: &'a str,
    budget_adapter: &'static str,
}

pub(super) fn normalize_broker_reserve_request(
    request: &ReserveComputeBrokerRequest,
) -> Result<NormalizedBrokerReserveRequest> {
    let reservation_id = required("Reservation ID", &request.reservation_id, 160)?;
    let consumer_account_id = required("消费者账户 ID", &request.consumer_account_id, 200)?;
    let idempotency_key = required("Broker 幂等键", &request.idempotency_key, 200)?;
    let job_id = required("Job ID", &request.job_id, 200)?;
    let expected_job_digest = required("Job 摘要", &request.expected_job_digest, 200)?;
    if request.expected_job_revision <= 0 {
        bail!("Broker expected_job_revision 必须为正整数");
    }
    if request.reserved_capacity.is_empty() || request.reserved_capacity.len() > 64 {
        bail!("Broker 预留容量必须包含 1 到 64 个 meter");
    }
    let mut meters = BTreeSet::new();
    let mut reserved_capacity = request.reserved_capacity.clone();
    for item in &mut reserved_capacity {
        item.meter = required("Broker 容量 meter", &item.meter, 120)?;
        if item.quantity <= 0 || !meters.insert(item.meter.clone()) {
            bail!("Broker 容量数量必须为正整数且 meter 不能重复");
        }
    }
    reserved_capacity.sort_by(|left, right| left.meter.cmp(&right.meter));
    let expires_at = canonical_utc(&request.expires_at)?;
    let canonical = CanonicalBrokerReserveRequest {
        schema: BROKER_RESERVE_REQUEST_SCHEMA,
        reservation_id: &reservation_id,
        consumer_account_id: &consumer_account_id,
        idempotency_key: &idempotency_key,
        job_id: &job_id,
        expected_job_revision: request.expected_job_revision,
        expected_job_digest: &expected_job_digest,
        reserved_capacity: &reserved_capacity,
        expires_at: &expires_at,
        budget_adapter: super::BROKER_BUDGET_ADAPTER,
    };
    let request_digest = hex::encode(Sha256::digest(serde_json::to_vec(&canonical)?));
    Ok(NormalizedBrokerReserveRequest {
        reservation_id,
        consumer_account_id,
        idempotency_key,
        job_id,
        expected_job_revision: request.expected_job_revision,
        expected_job_digest,
        reserved_capacity,
        expires_at,
        request_digest,
    })
}

pub(super) fn cny_micros_to_fen(amount_micros: i64) -> Result<i64> {
    if amount_micros < 0 {
        bail!("人民币预算微单位不能为负数");
    }
    amount_micros
        .checked_add(MICROS_PER_CNY_FEN - 1)
        .map(|value| value / MICROS_PER_CNY_FEN)
        .ok_or_else(|| anyhow!("人民币预算换算为分时溢出"))
}

pub(super) fn timestamp_after(value: &str) -> Result<String> {
    let previous = DateTime::parse_from_rfc3339(value)
        .context("Broker 前序时间不是 RFC3339")?
        .with_timezone(&Utc);
    let minimum = previous
        .checked_add_signed(Duration::nanoseconds(1))
        .ok_or_else(|| anyhow!("Broker 时间推进溢出"))?;
    Ok(std::cmp::max(Utc::now(), minimum).to_rfc3339())
}

pub(in crate::store) fn broker_compute_call_id(reservation_id: &str) -> String {
    format!("compute_broker:{reservation_id}")
}

pub(super) fn ensure_future_expiry(value: &str) -> Result<()> {
    let parsed =
        DateTime::parse_from_rfc3339(value).context("Broker Reservation 到期时间不是 RFC3339")?;
    if parsed <= Utc::now() {
        bail!("Broker Reservation 到期时间必须晚于当前时间");
    }
    Ok(())
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

fn canonical_utc(value: &str) -> Result<String> {
    let parsed = DateTime::parse_from_rfc3339(value.trim())
        .context("Broker Reservation 到期时间不是 RFC3339")?;
    if parsed.offset().local_minus_utc() != 0 {
        bail!("Broker Reservation 到期时间必须使用 UTC");
    }
    Ok(parsed.with_timezone(&Utc).to_rfc3339())
}
