use anyhow::{anyhow, bail, Result};
use chrono::{DateTime, Utc};
use serde::Serialize;
use sha2::{Digest, Sha256};

use crate::compute_federation::{
    capacity::{
        ComputeCapacityCausalBinding, ComputeCapacityClaimKind, ComputeCapacityOfferBinding,
        ComputeCapacityPoolBinding,
    },
    market::ComputeDeliveryWindowBinding,
};

use super::{
    compute_capacity_claim_transitions::{
        ComputeCapacityClaimTerminalAction, FinishComputeCapacityClaim,
    },
    compute_capacity_claims::HoldComputeCapacityClaim,
    compute_capacity_ledger::AddComputeCapacitySupply,
    compute_capacity_supply_withdrawal::WithdrawComputeCapacitySupply,
};

const ADD_SUPPLY_REQUEST_SCHEMA: &str = "compute_federation.capacity_add_supply_request.v1";
const WITHDRAW_SUPPLY_REQUEST_SCHEMA: &str =
    "compute_federation.capacity_withdraw_supply_request.v1";
const HOLD_CLAIM_REQUEST_SCHEMA: &str = "compute_federation.capacity_hold_claim_request.v2";
const FINISH_CLAIM_REQUEST_SCHEMA: &str = "compute_federation.capacity_finish_claim_request.v1";

#[derive(Serialize)]
struct CanonicalPoolBinding<'a> {
    pool_id: &'a str,
    capacity_epoch: i64,
    pool_revision: i64,
    pool_digest: &'a str,
}

#[derive(Serialize)]
struct CanonicalDeliveryWindowBinding<'a> {
    window_id: &'a str,
    window_digest: &'a str,
}

#[derive(Serialize)]
struct CanonicalCapacityLine {
    bucket_id: String,
    quantity_units: i64,
}

#[derive(Serialize)]
struct CanonicalSupplyRequest<'a> {
    schema: &'static str,
    pool: CanonicalPoolBinding<'a>,
    delivery_window: CanonicalDeliveryWindowBinding<'a>,
    subject_kind: &'a str,
    subject_id: &'a str,
    lines: Vec<CanonicalCapacityLine>,
    occurred_at: String,
}

#[derive(Serialize)]
struct CanonicalHoldRequest<'a> {
    schema: &'static str,
    pool: CanonicalPoolBinding<'a>,
    delivery_window: CanonicalDeliveryWindowBinding<'a>,
    claim_kind: ComputeCapacityClaimKind,
    subject_kind: &'a str,
    subject_id: &'a str,
    lines: Vec<CanonicalCapacityLine>,
    causal_binding: CanonicalCausalBinding<'a>,
    expires_at: String,
    occurred_at: String,
}

#[derive(Serialize)]
struct CanonicalOfferBinding<'a> {
    offer_id: &'a str,
    offer_version: i64,
    offer_digest: &'a str,
}

#[derive(Serialize)]
struct CanonicalCausalBinding<'a> {
    offer: Option<CanonicalOfferBinding<'a>>,
    job_id: Option<&'a str>,
    reservation_id: Option<&'a str>,
    attempt_lease_id: Option<&'a str>,
    fencing_generation: Option<i64>,
}

#[derive(Serialize)]
struct CanonicalFinishRequest<'a> {
    schema: &'static str,
    claim_id: &'a str,
    expected_revision: i64,
    action: &'static str,
    occurred_at: String,
}

pub(super) fn add_supply_request_digest(input: &AddComputeCapacitySupply) -> Result<String> {
    digest(&CanonicalSupplyRequest {
        schema: ADD_SUPPLY_REQUEST_SCHEMA,
        pool: canonical_pool(&input.pool),
        delivery_window: canonical_window(&input.delivery_window),
        subject_kind: input.subject_kind.trim(),
        subject_id: input.subject_id.trim(),
        lines: canonical_lines(
            input
                .lines
                .iter()
                .map(|line| (line.bucket_id.as_str(), line.quantity_units)),
        ),
        occurred_at: canonical_utc("容量发行发生时间", &input.occurred_at)?,
    })
}

pub(super) fn withdraw_supply_request_digest(
    input: &WithdrawComputeCapacitySupply,
) -> Result<String> {
    digest(&CanonicalSupplyRequest {
        schema: WITHDRAW_SUPPLY_REQUEST_SCHEMA,
        pool: canonical_pool(&input.pool),
        delivery_window: canonical_window(&input.delivery_window),
        subject_kind: input.subject_kind.trim(),
        subject_id: input.subject_id.trim(),
        lines: canonical_lines(
            input
                .lines
                .iter()
                .map(|line| (line.bucket_id.as_str(), line.quantity_units)),
        ),
        occurred_at: canonical_utc("容量撤出发生时间", &input.occurred_at)?,
    })
}

pub(super) fn hold_claim_request_digest(input: &HoldComputeCapacityClaim) -> Result<String> {
    let expires_at = input
        .expires_at
        .as_deref()
        .ok_or_else(|| anyhow!("容量 Claim 必须设置到期时间"))?;
    digest(&CanonicalHoldRequest {
        schema: HOLD_CLAIM_REQUEST_SCHEMA,
        pool: canonical_pool(&input.pool),
        delivery_window: canonical_window(&input.delivery_window),
        claim_kind: input.claim_kind,
        subject_kind: input.subject_kind.trim(),
        subject_id: input.subject_id.trim(),
        lines: canonical_lines(
            input
                .lines
                .iter()
                .map(|line| (line.bucket_id.as_str(), line.quantity_units)),
        ),
        causal_binding: canonical_causal_binding(&input.causal_binding),
        expires_at: canonical_utc("容量 Claim 到期时间", expires_at)?,
        occurred_at: canonical_utc("容量 Claim 发生时间", &input.occurred_at)?,
    })
}

fn canonical_causal_binding(binding: &ComputeCapacityCausalBinding) -> CanonicalCausalBinding<'_> {
    CanonicalCausalBinding {
        offer: binding.offer.as_ref().map(canonical_offer_binding),
        job_id: binding.job_id.as_deref().map(str::trim),
        reservation_id: binding.reservation_id.as_deref().map(str::trim),
        attempt_lease_id: binding.attempt_lease_id.as_deref().map(str::trim),
        fencing_generation: binding.fencing_generation,
    }
}

fn canonical_offer_binding(binding: &ComputeCapacityOfferBinding) -> CanonicalOfferBinding<'_> {
    CanonicalOfferBinding {
        offer_id: binding.offer_id.trim(),
        offer_version: binding.offer_version,
        offer_digest: binding.offer_digest.trim(),
    }
}

pub(super) fn finish_claim_request_digest(input: &FinishComputeCapacityClaim) -> Result<String> {
    digest(&CanonicalFinishRequest {
        schema: FINISH_CLAIM_REQUEST_SCHEMA,
        claim_id: input.claim_id.trim(),
        expected_revision: input.expected_revision,
        action: match input.action {
            ComputeCapacityClaimTerminalAction::Release => "release",
            ComputeCapacityClaimTerminalAction::Expire => "expire",
        },
        occurred_at: canonical_utc("容量 Claim 终态发生时间", &input.occurred_at)?,
    })
}

fn canonical_pool(pool: &ComputeCapacityPoolBinding) -> CanonicalPoolBinding<'_> {
    CanonicalPoolBinding {
        pool_id: pool.pool_id.trim(),
        capacity_epoch: pool.capacity_epoch,
        pool_revision: pool.pool_revision,
        pool_digest: pool.pool_digest.trim(),
    }
}

fn canonical_window(window: &ComputeDeliveryWindowBinding) -> CanonicalDeliveryWindowBinding<'_> {
    CanonicalDeliveryWindowBinding {
        window_id: window.window_id.trim(),
        window_digest: window.window_digest.trim(),
    }
}

fn canonical_lines<'a>(lines: impl Iterator<Item = (&'a str, i64)>) -> Vec<CanonicalCapacityLine> {
    let mut lines = lines
        .map(|(bucket_id, quantity_units)| CanonicalCapacityLine {
            bucket_id: bucket_id.trim().to_string(),
            quantity_units,
        })
        .collect::<Vec<_>>();
    lines.sort_by(|left, right| {
        left.bucket_id
            .cmp(&right.bucket_id)
            .then(left.quantity_units.cmp(&right.quantity_units))
    });
    lines
}

pub(super) fn canonical_utc(label: &str, value: &str) -> Result<String> {
    let parsed =
        DateTime::parse_from_rfc3339(value.trim()).map_err(|_| anyhow!("{label}不是 RFC3339"))?;
    if parsed.offset().local_minus_utc() != 0 {
        bail!("{label}必须使用 UTC 时区");
    }
    Ok(parsed.with_timezone(&Utc).to_rfc3339())
}

fn digest(payload: &impl Serialize) -> Result<String> {
    Ok(hex::encode(Sha256::digest(serde_json::to_vec(payload)?)))
}
