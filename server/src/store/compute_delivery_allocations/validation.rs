use std::collections::BTreeMap;

use anyhow::{anyhow, bail, Result};
use chrono::{DateTime, FixedOffset};
use rusqlite::Connection;

use crate::compute_federation::{
    capacity::{
        ComputeCapacityClaim, ComputeCapacityClaimKind, ComputeCapacityClaimState,
        ComputeCapacityPoolStatus,
    },
    delivery_allocation::ComputeDeliveryAllocationGrant,
    execution::JOB_STATUS_QUOTED,
    offer::{OFFER_STATUS_ACTIVE, OFFER_STATUS_DRAINING},
    provider::{PROVIDER_STATUS_ACTIVE, PROVIDER_STATUS_DRAINING},
};

use super::{
    super::{
        compute_capacity_claim_rows::stored_claim_on,
        compute_capacity_commitments::{
            audited_capacity_commitment_source_on, ComputeCapacityCommitmentCreateReceipt,
        },
        compute_capacity_instruments::{
            require_capacity_instrument_adoption_for_historical_offer_on,
            require_current_capacity_instrument_adoption_on,
        },
        compute_capacity_pool_queries::current_capacity_pool_on,
        compute_job_registry::{current_registered_job_on, ComputeJobRegistrationReceipt},
        compute_offer_registry::current_registered_offer_on,
        compute_price_snapshot_registry::registered_price_snapshot_on,
        compute_provider_registry::current_registered_provider_on,
    },
    types::{
        CreateComputeDeliveryAllocationGrant, DeclineComputeDeliveryAllocationGrant,
        ExerciseComputeDeliveryAllocationGrant, ExpireDueComputeDeliveryAllocationGrants,
        COMPUTE_DELIVERY_ALLOCATION_DECLINE_CONFIRMATION,
        COMPUTE_DELIVERY_ALLOCATION_EXERCISE_CONFIRMATION,
        COMPUTE_DELIVERY_ALLOCATION_EXPIRE_DUE_CONFIRMATION,
        COMPUTE_DELIVERY_ALLOCATION_GRANT_CONFIRMATION,
    },
};

pub(super) struct ValidatedAllocationSource {
    pub commitment: ComputeCapacityCommitmentCreateReceipt,
    pub parent_claim: ComputeCapacityClaim,
    pub source_job: ComputeJobRegistrationReceipt,
}

pub(super) fn validate_create_input(input: &CreateComputeDeliveryAllocationGrant) -> Result<()> {
    for (label, value, max) in [
        (
            "Provider owner account ID",
            input.provider_owner_account_id.as_str(),
            200,
        ),
        ("Provider ID", input.provider_id.as_str(), 160),
        ("Pool ID", input.pool_id.as_str(), 200),
        ("Commitment ID", input.commitment_id.as_str(), 200),
        (
            "consumer account ID",
            input.consumer_account_id.as_str(),
            200,
        ),
        ("Job ID", input.job_id.as_str(), 200),
        ("idempotency scope", input.idempotency_scope.as_str(), 240),
        ("idempotency key", input.idempotency_key.as_str(), 200),
    ] {
        validate_exact(label, value, max)?;
    }
    if input.provider_owner_account_id == input.consumer_account_id {
        bail!("DeliveryAllocation 必须绑定不同的 Provider owner 与 consumer");
    }
    if input.expected_commitment_revision != 1 || input.expected_job_revision <= 0 {
        bail!("DeliveryAllocation Grant expected revision 无效");
    }
    validate_digest(
        "expected Commitment digest",
        &input.expected_commitment_digest,
    )?;
    validate_digest("expected Job digest", &input.expected_job_digest)?;
    if input.confirmation != COMPUTE_DELIVERY_ALLOCATION_GRANT_CONFIRMATION {
        bail!("DeliveryAllocation Grant 缺少固定确认短语");
    }
    Ok(())
}

pub(super) fn validate_exercise_input(
    input: &ExerciseComputeDeliveryAllocationGrant,
) -> Result<()> {
    for (label, value, max) in [
        (
            "consumer account ID",
            input.consumer_account_id.as_str(),
            200,
        ),
        ("Grant ID", input.grant_id.as_str(), 200),
        ("Reservation ID", input.reservation_id.as_str(), 200),
        ("idempotency scope", input.idempotency_scope.as_str(), 240),
        ("idempotency key", input.idempotency_key.as_str(), 200),
    ] {
        validate_exact(label, value, max)?;
    }
    if input.expected_grant_revision != 1 {
        bail!("DeliveryAllocation Exercise 只接受 expected Grant revision 1");
    }
    validate_digest("expected Grant digest", &input.expected_grant_digest)?;
    if input.confirmation != COMPUTE_DELIVERY_ALLOCATION_EXERCISE_CONFIRMATION {
        bail!("DeliveryAllocation Exercise 缺少固定财务确认短语");
    }
    Ok(())
}

pub(super) fn validate_decline_input(input: &DeclineComputeDeliveryAllocationGrant) -> Result<()> {
    for (label, value, max) in [
        (
            "consumer account ID",
            input.consumer_account_id.as_str(),
            200,
        ),
        ("Grant ID", input.grant_id.as_str(), 200),
        ("idempotency scope", input.idempotency_scope.as_str(), 240),
        ("idempotency key", input.idempotency_key.as_str(), 200),
    ] {
        validate_exact(label, value, max)?;
    }
    if input.expected_grant_revision != 1 {
        bail!("DeliveryAllocation Decline 只接受 expected Grant revision 1");
    }
    validate_digest("expected Grant digest", &input.expected_grant_digest)?;
    if input.confirmation != COMPUTE_DELIVERY_ALLOCATION_DECLINE_CONFIRMATION {
        bail!("DeliveryAllocation Decline 缺少固定确认短语");
    }
    Ok(())
}

pub(super) fn validate_expire_input(
    input: &ExpireDueComputeDeliveryAllocationGrants,
) -> Result<()> {
    validate_exact("platform admin user ID", &input.admin_user_id, 200)?;
    if !(1..=100).contains(&input.limit) {
        bail!("DeliveryAllocation Expire Due limit 必须在 1 到 100 之间");
    }
    if input.confirmation != COMPUTE_DELIVERY_ALLOCATION_EXPIRE_DUE_CONFIRMATION {
        bail!("DeliveryAllocation Expire Due 缺少固定确认短语");
    }
    Ok(())
}

pub(super) fn validate_grant_source_on(
    conn: &Connection,
    input: &CreateComputeDeliveryAllocationGrant,
    recorded_at: &str,
) -> Result<ValidatedAllocationSource> {
    let (commitment, terminal) = audited_capacity_commitment_source_on(conn, &input.commitment_id)?
        .ok_or_else(|| anyhow!("DeliveryAllocation 来源 Commitment 不存在"))?;
    if terminal.is_some() {
        bail!("已有 v225 terminal 的 Commitment 不能创建 DeliveryAllocation Grant");
    }
    let root = &commitment.commitment;
    if root.commitment_revision != input.expected_commitment_revision
        || root.commitment_digest != input.expected_commitment_digest
        || root.owner_account_id != input.provider_owner_account_id
        || root.provider.provider_id != input.provider_id
        || root.pool.pool_id != input.pool_id
    {
        bail!("DeliveryAllocation Grant 的 Commitment owner/path/revision/digest 不匹配");
    }
    let now = parse_utc("DeliveryAllocation Grant Store time", recorded_at)?;
    if now
        >= parse_utc(
            "DeliveryAllocation window start",
            &root.delivery_window.starts_at_utc,
        )?
    {
        bail!("DeliveryAllocation Grant 只能在交付窗口开始前创建");
    }
    let snapshot = registered_price_snapshot_on(conn, &root.price_snapshot_id)?
        .ok_or_else(|| anyhow!("DeliveryAllocation Grant 的 Price Snapshot 缺失"))?;
    if snapshot.snapshot_digest != root.price_snapshot_digest
        || now
            >= parse_utc(
                "DeliveryAllocation Snapshot expires_at",
                &snapshot.expires_at,
            )?
    {
        bail!("DeliveryAllocation Grant 必须在 exact Snapshot 仍有效时创建");
    }
    require_commitment_instrument_authority_on(conn, root, &snapshot, false)?;
    ensure_current_supply_on(conn, root, false)?;
    let source_job = current_registered_job_on(conn, &input.job_id)?
        .ok_or_else(|| anyhow!("DeliveryAllocation Grant 的 current Job 不存在"))?;
    validate_source_job(root, &source_job, input)?;
    let parent_claim = current_parent_claim_on(conn, root)?;
    ensure_whole_only_job_limits(&parent_claim, &source_job)?;
    Ok(ValidatedAllocationSource {
        commitment,
        parent_claim,
        source_job,
    })
}

pub(super) fn validate_exercise_source_on(
    conn: &Connection,
    grant: &ComputeDeliveryAllocationGrant,
    recorded_at: &str,
) -> Result<ValidatedAllocationSource> {
    let (commitment, terminal) =
        audited_capacity_commitment_source_on(conn, &grant.commitment.commitment_id)?
            .ok_or_else(|| anyhow!("DeliveryAllocation Exercise 来源 Commitment 缺失"))?;
    if terminal.is_some() {
        bail!("已有 v225 terminal 的 Commitment 不能行权");
    }
    let root = &commitment.commitment;
    if root.commitment_revision != grant.commitment.commitment_revision
        || root.commitment_digest != grant.commitment.commitment_digest
        || root.owner_account_id != grant.provider_owner_account_id
    {
        bail!("DeliveryAllocation Exercise 的 Commitment root 已漂移");
    }
    let now = parse_utc("DeliveryAllocation Exercise Store time", recorded_at)?;
    if now
        >= parse_utc(
            "DeliveryAllocation exercise expiry",
            &grant.exercise_expires_at,
        )?
        || grant.exercise_expires_at != root.delivery_window.starts_at_utc
    {
        bail!("DeliveryAllocation Grant 已到行权截止时间");
    }
    let snapshot = registered_price_snapshot_on(conn, &root.price_snapshot_id)?
        .ok_or_else(|| anyhow!("DeliveryAllocation Exercise 的 Price Snapshot 缺失"))?;
    require_commitment_instrument_authority_on(conn, root, &snapshot, true)?;
    ensure_current_supply_on(conn, root, true)?;
    let source_job = current_registered_job_on(conn, &grant.job.job_id)?
        .ok_or_else(|| anyhow!("DeliveryAllocation Exercise 的 current Job 不存在"))?;
    if source_job.job.consumer_account_id != grant.consumer_account_id
        || source_job.job.project_id != grant.project_id
        || source_job.job.status != JOB_STATUS_QUOTED
        || source_job.revision != grant.job.job_revision
        || source_job.job_digest != grant.job.job_digest
    {
        bail!("DeliveryAllocation Exercise 只接受 Grant 绑定的 current exact quoted Job");
    }
    let selected = source_job
        .job
        .selected_offer
        .as_ref()
        .ok_or_else(|| anyhow!("DeliveryAllocation Job 缺少 Offer binding"))?;
    if selected.offer_id != root.offer.offer_id
        || selected.offer_version != root.offer.offer_version
        || selected.offer_digest != root.offer.offer_digest
        || source_job.job.price_snapshot_id.as_deref() != Some(root.price_snapshot_id.as_str())
    {
        bail!("DeliveryAllocation Exercise 的 Job/Offer/Snapshot 与 Commitment 不一致");
    }
    let parent_claim = current_parent_claim_on(conn, root)?;
    ensure_whole_only_job_limits(&parent_claim, &source_job)?;
    Ok(ValidatedAllocationSource {
        commitment,
        parent_claim,
        source_job,
    })
}

fn require_commitment_instrument_authority_on(
    conn: &Connection,
    commitment: &crate::compute_federation::capacity_commitment::ComputeCapacityCommitment,
    snapshot: &crate::compute_federation::market::ComputePriceSnapshot,
    allow_historical_offer: bool,
) -> Result<()> {
    if snapshot.snapshot_digest != commitment.price_snapshot_digest
        || snapshot.instrument_id.as_deref() != Some(commitment.instrument_id.as_str())
    {
        bail!("DeliveryAllocation Commitment/Snapshot instrument binding 不一致");
    }
    let offer = if allow_historical_offer {
        super::super::compute_offer_registry::registered_offer_version_on(
            conn,
            &commitment.offer.offer_id,
            commitment.offer.offer_version,
        )?
    } else {
        current_registered_offer_on(conn, &commitment.offer.offer_id)?
    }
    .ok_or_else(|| anyhow!("DeliveryAllocation CapacityInstrument exact Offer 缺失"))?;
    if offer.offer.offer_version != commitment.offer.offer_version
        || offer.offer.offer_digest != commitment.offer.offer_digest
    {
        bail!("DeliveryAllocation CapacityInstrument Offer exact version/digest 已漂移");
    }
    let authority = if allow_historical_offer {
        require_capacity_instrument_adoption_for_historical_offer_on(
            conn,
            &offer.offer,
            snapshot.instrument_id.as_deref(),
        )?
    } else {
        require_current_capacity_instrument_adoption_on(
            conn,
            &offer.offer,
            snapshot.instrument_id.as_deref(),
        )?
    };
    authority
        .ok_or_else(|| anyhow!("DeliveryAllocation 缺少 CapacityInstrument adoption authority"))?;
    Ok(())
}

pub(super) fn validate_nonexercise_source_on(
    conn: &Connection,
    grant: &ComputeDeliveryAllocationGrant,
) -> Result<()> {
    let (commitment, terminal) =
        audited_capacity_commitment_source_on(conn, &grant.commitment.commitment_id)?
            .ok_or_else(|| anyhow!("DeliveryAllocation terminal 来源 Commitment 缺失"))?;
    if terminal.is_some()
        || commitment.commitment.commitment_revision != grant.commitment.commitment_revision
        || commitment.commitment.commitment_digest != grant.commitment.commitment_digest
    {
        bail!("DeliveryAllocation terminal 与 v225 terminal/source 冲突");
    }
    let _ = current_parent_claim_on(conn, &commitment.commitment)?;
    Ok(())
}

fn validate_source_job(
    commitment: &crate::compute_federation::capacity_commitment::ComputeCapacityCommitment,
    source_job: &ComputeJobRegistrationReceipt,
    input: &CreateComputeDeliveryAllocationGrant,
) -> Result<()> {
    let selected = source_job
        .job
        .selected_offer
        .as_ref()
        .ok_or_else(|| anyhow!("DeliveryAllocation quoted Job 缺少 Offer binding"))?;
    if source_job.job.consumer_account_id != input.consumer_account_id
        || source_job.job.status != JOB_STATUS_QUOTED
        || source_job.revision != input.expected_job_revision
        || source_job.job_digest != input.expected_job_digest
        || selected.offer_id != commitment.offer.offer_id
        || selected.offer_version != commitment.offer.offer_version
        || selected.offer_digest != commitment.offer.offer_digest
        || source_job.job.price_snapshot_id.as_deref()
            != Some(commitment.price_snapshot_id.as_str())
    {
        bail!("DeliveryAllocation Grant 只能绑定 consumer 的 current exact quoted Job");
    }
    Ok(())
}

fn current_parent_claim_on(
    conn: &Connection,
    commitment: &crate::compute_federation::capacity_commitment::ComputeCapacityCommitment,
) -> Result<ComputeCapacityClaim> {
    let claim = stored_claim_on(conn, &commitment.claim.claim_id)?
        .ok_or_else(|| anyhow!("DeliveryAllocation parent Commitment Claim 缺失"))?;
    if claim.revision != commitment.claim.claim_revision
        || claim.claim_digest != commitment.claim.claim_digest
        || claim.claim_kind != ComputeCapacityClaimKind::CapacityCommitment
        || claim.state != ComputeCapacityClaimState::Held
        || claim.subject_kind != "compute_capacity_commitment"
        || claim.subject_id != commitment.commitment_id
        || claim.parent_claim_id.is_some()
        || claim.pool != commitment.pool
        || claim.delivery_window != commitment.delivery_window.binding
    {
        bail!("DeliveryAllocation parent Commitment Claim 不是 exact held root");
    }
    Ok(claim)
}

fn ensure_current_supply_on(
    conn: &Connection,
    commitment: &crate::compute_federation::capacity_commitment::ComputeCapacityCommitment,
    allow_draining: bool,
) -> Result<()> {
    let provider = current_registered_provider_on(conn, &commitment.provider.provider_id)?
        .ok_or_else(|| anyhow!("DeliveryAllocation current Provider 缺失"))?;
    let provider_allowed = provider.provider.status == PROVIDER_STATUS_ACTIVE
        || (allow_draining && provider.provider.status == PROVIDER_STATUS_DRAINING);
    if !provider_allowed || provider.provider.owner_account_id != commitment.owner_account_id {
        bail!("DeliveryAllocation current Provider 状态或 owner 不安全");
    }
    let offer = current_registered_offer_on(conn, &commitment.offer.offer_id)?
        .ok_or_else(|| anyhow!("DeliveryAllocation current Offer 缺失"))?;
    let offer_allowed = offer.offer.status == OFFER_STATUS_ACTIVE
        || (allow_draining && offer.offer.status == OFFER_STATUS_DRAINING);
    if !offer_allowed
        || offer.offer.provider_id != commitment.provider.provider_id
        || offer.offer.capacity_pool.pool_id != commitment.pool.pool_id
    {
        bail!("DeliveryAllocation current Offer 状态或稳定绑定不安全");
    }
    let pool = current_capacity_pool_on(conn, &commitment.pool.pool_id)?
        .ok_or_else(|| anyhow!("DeliveryAllocation current Pool 缺失"))?;
    if pool.provider_id != commitment.provider.provider_id
        || pool.binding.pool_id != commitment.pool.pool_id
        || pool.binding.capacity_epoch != commitment.pool.capacity_epoch
        || (!matches!(pool.status, ComputeCapacityPoolStatus::Active)
            && !(allow_draining && matches!(pool.status, ComputeCapacityPoolStatus::Draining)))
    {
        bail!("DeliveryAllocation current Pool provider/epoch/status 不安全");
    }
    Ok(())
}

fn ensure_whole_only_job_limits(
    claim: &ComputeCapacityClaim,
    job: &ComputeJobRegistrationReceipt,
) -> Result<()> {
    let claim_lines = claim
        .lines
        .iter()
        .map(|line| (line.bucket.meter.as_str(), line.quantity_units))
        .collect::<BTreeMap<_, _>>();
    let job_limits = job
        .job
        .workload
        .usage_limits
        .iter()
        .map(|limit| (limit.meter.as_str(), limit.max_quantity))
        .collect::<BTreeMap<_, _>>();
    if claim_lines.len() != claim.lines.len()
        || job_limits.len() != job.job.workload.usage_limits.len()
        || claim_lines != job_limits
    {
        bail!("DeliveryAllocation 必须 whole-only 匹配 parent Claim 与 Job usage limits");
    }
    Ok(())
}

pub(super) fn parse_utc(label: &str, value: &str) -> Result<DateTime<FixedOffset>> {
    let parsed =
        DateTime::parse_from_rfc3339(value).map_err(|_| anyhow!("{label} 不是 RFC3339"))?;
    if parsed.offset().local_minus_utc() != 0 {
        bail!("{label} 必须使用 UTC 时区");
    }
    Ok(parsed)
}

pub(super) fn validate_exact(label: &str, value: &str, max: usize) -> Result<()> {
    if value.trim().is_empty()
        || value != value.trim()
        || value.chars().count() > max
        || value.chars().any(char::is_control)
    {
        bail!("{label} 为空、过长、未规范化或包含控制字符");
    }
    Ok(())
}

pub(super) fn validate_digest(label: &str, value: &str) -> Result<()> {
    if value.len() != 64
        || value
            .bytes()
            .any(|byte| !byte.is_ascii_digit() && !(b'a'..=b'f').contains(&byte))
    {
        bail!("{label} 必须是 64 位小写十六进制 SHA-256");
    }
    Ok(())
}
