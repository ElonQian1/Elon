use anyhow::{bail, Context, Result};
use chrono::{DateTime, Utc};
use serde::Serialize;
use sha2::{Digest, Sha256};

use super::{ComputeBrokerFinishAction, FinishComputeBrokerRequest};

const BROKER_FINISH_REQUEST_SCHEMA: &str = "compute_federation.broker_finish_request.v1";

#[derive(Debug, Clone)]
pub(super) struct NormalizedBrokerFinishRequest {
    pub reservation_id: String,
    pub consumer_account_id: String,
    pub idempotency_key: String,
    pub expected_reservation_revision: i64,
    pub expected_reservation_digest: String,
    pub action: ComputeBrokerFinishAction,
    pub occurred_at: String,
    pub request_digest: String,
}

#[derive(Serialize)]
struct CanonicalBrokerFinishRequest<'a> {
    schema: &'static str,
    reservation_id: &'a str,
    consumer_account_id: &'a str,
    idempotency_key: &'a str,
    expected_reservation_revision: i64,
    expected_reservation_digest: &'a str,
    action: ComputeBrokerFinishAction,
    occurred_at: &'a str,
}

pub(super) fn normalize_broker_finish_request(
    request: &FinishComputeBrokerRequest,
) -> Result<NormalizedBrokerFinishRequest> {
    let reservation_id = required("Reservation ID", &request.reservation_id, 160)?;
    let consumer_account_id = required("消费者账户 ID", &request.consumer_account_id, 200)?;
    let idempotency_key = required("Broker 终态幂等键", &request.idempotency_key, 200)?;
    let expected_reservation_digest = required(
        "Reservation 摘要",
        &request.expected_reservation_digest,
        200,
    )?;
    if request.expected_reservation_revision <= 0 {
        bail!("Broker expected_reservation_revision 必须为正整数");
    }
    let occurred_at = canonical_utc(&request.occurred_at)?;
    let canonical = CanonicalBrokerFinishRequest {
        schema: BROKER_FINISH_REQUEST_SCHEMA,
        reservation_id: &reservation_id,
        consumer_account_id: &consumer_account_id,
        idempotency_key: &idempotency_key,
        expected_reservation_revision: request.expected_reservation_revision,
        expected_reservation_digest: &expected_reservation_digest,
        action: request.action,
        occurred_at: &occurred_at,
    };
    let request_digest = hex::encode(Sha256::digest(serde_json::to_vec(&canonical)?));
    Ok(NormalizedBrokerFinishRequest {
        reservation_id,
        consumer_account_id,
        idempotency_key,
        expected_reservation_revision: request.expected_reservation_revision,
        expected_reservation_digest,
        action: request.action,
        occurred_at,
        request_digest,
    })
}

pub(super) fn action_value(action: ComputeBrokerFinishAction) -> &'static str {
    match action {
        ComputeBrokerFinishAction::Release => "release",
        ComputeBrokerFinishAction::Expire => "expire",
    }
}

pub(super) fn billing_terminal_status(action: ComputeBrokerFinishAction) -> &'static str {
    match action {
        ComputeBrokerFinishAction::Release => "released_no_usage",
        ComputeBrokerFinishAction::Expire => "expired_released",
    }
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
    let parsed =
        DateTime::parse_from_rfc3339(value.trim()).context("Broker 终态发生时间不是 RFC3339")?;
    if parsed.offset().local_minus_utc() != 0 || parsed > Utc::now() {
        bail!("Broker 终态发生时间必须使用 UTC 且不能晚于当前时间");
    }
    Ok(parsed.with_timezone(&Utc).to_rfc3339())
}
