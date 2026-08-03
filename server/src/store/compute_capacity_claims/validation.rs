use anyhow::{anyhow, bail, Result};
use chrono::DateTime;

use crate::compute_federation::capacity::ComputeCapacityClaimKind;

use super::HoldComputeCapacityClaim;

pub(super) fn validate_hold_input(input: &HoldComputeCapacityClaim) -> Result<()> {
    for (label, value) in [
        ("容量池 ID", input.pool.pool_id.as_str()),
        ("容量池摘要", input.pool.pool_digest.as_str()),
        ("交付窗口 ID", input.delivery_window.window_id.as_str()),
        ("交付窗口摘要", input.delivery_window.window_digest.as_str()),
        ("主体类型", input.subject_kind.as_str()),
        ("主体 ID", input.subject_id.as_str()),
        ("幂等范围", input.idempotency_scope.as_str()),
        ("幂等键", input.idempotency_key.as_str()),
        ("发生时间", input.occurred_at.as_str()),
    ] {
        if value.trim().is_empty() {
            bail!("{label}不能为空");
        }
    }
    if input.pool.capacity_epoch <= 0 || input.pool.pool_revision <= 0 {
        bail!("容量池 epoch 和版本必须为正整数");
    }
    if !matches!(
        input.claim_kind,
        ComputeCapacityClaimKind::QuoteHold
            | ComputeCapacityClaimKind::Reservation
            | ComputeCapacityClaimKind::CapacityCommitment
    ) {
        bail!("只有报价、预约或容量承诺 Claim 可以直接进入 held 状态");
    }
    if input.lines.is_empty() || input.lines.len() > 64 {
        bail!("容量 Claim 必须包含 1 到 64 个 bucket");
    }
    if input
        .lines
        .iter()
        .any(|line| line.bucket_id.trim().is_empty() || line.quantity_units <= 0)
    {
        bail!("容量 Claim bucket 和数量必须有效");
    }
    let occurred_at = parse_utc("容量 Claim 发生时间", input.occurred_at.trim())?;
    let expires_at = input
        .expires_at
        .as_deref()
        .ok_or_else(|| anyhow!("容量 Claim 必须设置到期时间"))?;
    if expires_at.trim().is_empty() {
        bail!("容量 Claim 到期时间不能为空字符串");
    }
    if parse_utc("容量 Claim 到期时间", expires_at.trim())? <= occurred_at {
        bail!("容量 Claim 到期时间必须晚于发生时间");
    }
    validate_hold_causal_binding(input)
}

fn validate_hold_causal_binding(input: &HoldComputeCapacityClaim) -> Result<()> {
    let binding = &input.causal_binding;
    if input.subject_kind.trim() == "compute_reservation"
        && (input.subject_kind != "compute_reservation"
            || input.claim_kind != ComputeCapacityClaimKind::Reservation)
    {
        bail!("compute_reservation 主体只能用于精确的 Reservation Claim");
    }
    if binding.attempt_lease_id.is_some() || binding.fencing_generation.is_some() {
        bail!("容量 Hold 不能提前绑定 Attempt 或 fencing generation");
    }
    if let Some(offer) = binding.offer.as_ref() {
        validate_exact_causal_value("Offer ID", &offer.offer_id)?;
        validate_exact_causal_value("Offer 摘要", &offer.offer_digest)?;
        if offer.offer_version <= 0 {
            bail!("容量 Hold 的 Offer 版本必须为正整数");
        }
    }
    if let Some(job_id) = binding.job_id.as_deref() {
        validate_exact_causal_value("Job ID", job_id)?;
    }
    if let Some(reservation_id) = binding.reservation_id.as_deref() {
        validate_exact_causal_value("Reservation ID", reservation_id)?;
        if binding.job_id.is_none() {
            bail!("容量 Hold 绑定 Reservation 时必须同时绑定 Job");
        }
        if input.claim_kind != ComputeCapacityClaimKind::Reservation {
            bail!("只有 Reservation Claim 可以绑定 Reservation ID");
        }
    }
    if input.claim_kind == ComputeCapacityClaimKind::Reservation {
        let reservation_id = binding
            .reservation_id
            .as_deref()
            .ok_or_else(|| anyhow!("Reservation Claim 缺少 Reservation 因果绑定"))?;
        if binding.offer.is_none()
            || binding.job_id.is_none()
            || input.subject_kind != "compute_reservation"
            || input.subject_id != reservation_id
        {
            bail!("Reservation Claim 必须完整绑定 Offer、Job 和同主体 Reservation");
        }
    }
    Ok(())
}

fn validate_exact_causal_value(label: &str, value: &str) -> Result<()> {
    if value.trim().is_empty() || value != value.trim() {
        bail!("容量 Hold {label} 不能为空或包含首尾空白");
    }
    Ok(())
}

pub(super) fn parse_utc(label: &str, value: &str) -> Result<DateTime<chrono::FixedOffset>> {
    let parsed = DateTime::parse_from_rfc3339(value).map_err(|_| anyhow!("{label}不是 RFC3339"))?;
    if parsed.offset().local_minus_utc() != 0 {
        bail!("{label}必须使用 UTC 时区");
    }
    Ok(parsed)
}
