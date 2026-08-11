//! Immutable Provider Capacity Commitment root and single terminal receipt authority.

use anyhow::{bail, Result};
use rusqlite::{params, Connection, OptionalExtension};

use crate::compute_federation::{
    capacity::ComputeCapacityClaimState,
    capacity_commitment::{
        ComputeCapacityCommitment, ComputeCapacityCommitmentLedgerBinding,
        CAPACITY_COMMITMENT_STATUS_CANCELED, CAPACITY_COMMITMENT_STATUS_COMMITTED,
        CAPACITY_COMMITMENT_STATUS_EXPIRED,
    },
    market::{PRICE_SOURCE_FALLBACK_CURVE, PRICING_MODE_CAPACITY_FUTURE},
};

mod canonical;
mod create;
mod read;
mod terminal;
mod types;
mod validation;

pub(crate) use types::{
    CancelComputeCapacityCommitment, ComputeCapacityCommitmentCreateReceipt,
    ComputeCapacityCommitmentDetail, ComputeCapacityCommitmentExpiryItem,
    ComputeCapacityCommitmentExpiryReport, ComputeCapacityCommitmentTerminalWriteReceipt,
    CreateComputeCapacityCommitment, ExpireDueComputeCapacityCommitments,
    COMPUTE_CAPACITY_COMMITMENT_CANCEL_CONFIRMATION,
    COMPUTE_CAPACITY_COMMITMENT_CREATE_CONFIRMATION,
    COMPUTE_CAPACITY_COMMITMENT_EXPIRE_DUE_CONFIRMATION,
};

use super::Store;
use super::{
    compute_offer_registry::registered_offer_version_on,
    compute_platform_reference_price_curve::audited_platform_reference_snapshot_binding_on,
    compute_price_snapshot_registry::registered_price_snapshot_on,
    compute_provider_registry::registered_provider_version_on,
};

impl Store {
    pub(crate) fn compute_capacity_commitment_for_owner(
        &self,
        owner_account_id: &str,
        provider_id: &str,
        pool_id: &str,
        commitment_id: &str,
    ) -> Result<Option<ComputeCapacityCommitmentDetail>> {
        for (label, value, max) in [
            ("owner account ID", owner_account_id, 200),
            ("Provider ID", provider_id, 160),
            ("Pool ID", pool_id, 200),
            ("Commitment ID", commitment_id, 200),
        ] {
            validation::validate_exact(label, value, max)?;
        }
        let connection = self.conn()?;
        let Some(commitment) = read::commitment_by_id_on(&connection, commitment_id)? else {
            return Ok(None);
        };
        if commitment.owner_account_id != owner_account_id
            || commitment.provider.provider_id != provider_id
            || commitment.pool.pool_id != pool_id
        {
            return Ok(None);
        }
        read::detail_on(&connection, commitment).map(Some)
    }

    pub(crate) fn list_compute_capacity_commitments_for_owner(
        &self,
        owner_account_id: &str,
        provider_id: &str,
        pool_id: &str,
        status: Option<&str>,
        limit: usize,
    ) -> Result<Vec<ComputeCapacityCommitmentDetail>> {
        for (label, value, max) in [
            ("owner account ID", owner_account_id, 200),
            ("Provider ID", provider_id, 160),
            ("Pool ID", pool_id, 200),
        ] {
            validation::validate_exact(label, value, max)?;
        }
        if !(1..=100).contains(&limit) {
            bail!("容量承诺 list limit 必须在 1 到 100 之间");
        }
        if status.is_some_and(|value| {
            !matches!(
                value,
                CAPACITY_COMMITMENT_STATUS_COMMITTED
                    | CAPACITY_COMMITMENT_STATUS_CANCELED
                    | CAPACITY_COMMITMENT_STATUS_EXPIRED
            )
        }) {
            bail!("容量承诺 list status 不受支持");
        }
        let connection = self.conn()?;
        let mut statement = connection.prepare(
            "SELECT commitments.commitment_id
               FROM compute_capacity_commitments commitments
               LEFT JOIN compute_capacity_commitment_terminal_receipts terminal
                 ON terminal.commitment_id=commitments.commitment_id
              WHERE commitments.owner_account_id=?1
                AND commitments.provider_id=?2 AND commitments.pool_id=?3
                AND (?4 IS NULL OR COALESCE(terminal.terminal_status,'committed')=?4)
              ORDER BY commitments.created_at DESC, commitments.commitment_id
              LIMIT ?5",
        )?;
        let ids = statement
            .query_map(
                params![owner_account_id, provider_id, pool_id, status, limit as i64],
                |row| row.get::<_, String>(0),
            )?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        drop(statement);
        ids.into_iter()
            .map(|id| {
                let commitment = read::commitment_by_id_on(&connection, &id)?
                    .ok_or_else(|| anyhow::anyhow!("容量承诺列表项在审计期间消失"))?;
                read::detail_on(&connection, commitment)
            })
            .collect()
    }
}

pub(super) fn audit_ledger_binding_on(
    conn: &Connection,
    binding: &ComputeCapacityCommitmentLedgerBinding,
    claim_id: &str,
) -> Result<()> {
    let found = conn
        .query_row(
            "SELECT 1 FROM compute_capacity_ledger_transactions
              WHERE transaction_id=?1 AND transaction_digest=?2 AND ledger_sequence=?3
                AND event_kind=?4 AND claim_id=?5 AND causal_transaction_id IS ?6",
            params![
                binding.transaction_id,
                binding.transaction_digest,
                binding.ledger_sequence,
                binding.event_kind,
                claim_id,
                binding.causal_transaction_id,
            ],
            |_| Ok(()),
        )
        .optional()?;
    if found.is_none() {
        bail!("容量承诺 ledger exact binding 审计失败");
    }
    Ok(())
}

pub(super) fn audit_immutable_dependencies_on(
    conn: &Connection,
    value: &ComputeCapacityCommitment,
) -> Result<()> {
    let provider = registered_provider_version_on(
        conn,
        &value.provider.provider_id,
        value.provider.policy_revision,
    )?
    .ok_or_else(|| anyhow::anyhow!("容量承诺 Provider 历史版本缺失"))?;
    if provider.provider.provider_id != value.provider.provider_id
        || provider.provider.policy_revision != value.provider.policy_revision
        || provider.provider_digest != value.provider.provider_digest
    {
        bail!("容量承诺 Provider 历史摘要不一致");
    }
    let offer =
        registered_offer_version_on(conn, &value.offer.offer_id, value.offer.offer_version)?
            .ok_or_else(|| anyhow::anyhow!("容量承诺 Offer 历史版本缺失"))?;
    if offer.offer.offer_digest != value.offer.offer_digest
        || offer.offer.provider_id != value.provider.provider_id
        || offer.provider_policy_revision != value.provider.policy_revision
        || offer.provider_digest != value.provider.provider_digest
        || offer.offer.capacity_pool != value.pool
    {
        bail!("容量承诺 Offer 历史版本绑定不一致");
    }
    let snapshot = registered_price_snapshot_on(conn, &value.price_snapshot_id)?
        .ok_or_else(|| anyhow::anyhow!("容量承诺 Price Snapshot 缺失"))?;
    if snapshot.snapshot_digest != value.price_snapshot_digest
        || snapshot.offer_id != value.offer.offer_id
        || snapshot.offer_version != value.offer.offer_version
        || snapshot.offer_digest != value.offer.offer_digest
        || snapshot.provider_id != value.provider.provider_id
        || snapshot.delivery_window != value.delivery_window
        || snapshot.pricing_mode != PRICING_MODE_CAPACITY_FUTURE
        || snapshot.instrument_id.as_deref() != Some(value.instrument_id.as_str())
        || snapshot.currency != "CNY"
        || snapshot.trade_id.is_some()
        || snapshot.price_source.source_kind != PRICE_SOURCE_FALLBACK_CURVE
    {
        bail!("容量承诺 Price Snapshot 历史绑定不一致");
    }
    let reference = audited_platform_reference_snapshot_binding_on(
        conn,
        &value.price_snapshot_id,
        &value.reference_binding.binding_id,
        &value.reference_binding.binding_digest,
    )?
    .ok_or_else(|| anyhow::anyhow!("容量承诺 v223 reference binding 缺失"))?;
    if reference.snapshot_id != value.price_snapshot_id
        || reference.snapshot_digest != value.price_snapshot_digest
        || reference.quote_id != snapshot.quote_id
        || reference.source_kind != PRICE_SOURCE_FALLBACK_CURVE
        || reference.source_id != snapshot.price_source.source_id
        || reference.source_version != snapshot.price_source.source_version
        || reference.source_digest != snapshot.price_source.source_digest
        || reference.quoted_at != snapshot.quoted_at
        || reference.expires_at != snapshot.expires_at
        || reference.status != "snapshot_registered"
    {
        bail!("容量承诺 v223 reference binding 摘要不一致");
    }
    Ok(())
}

pub(super) fn claim_state_name(state: ComputeCapacityClaimState) -> &'static str {
    match state {
        ComputeCapacityClaimState::Pending => "pending",
        ComputeCapacityClaimState::Held => "held",
        ComputeCapacityClaimState::Active => "active",
        ComputeCapacityClaimState::Consumed => "consumed",
        ComputeCapacityClaimState::Released => "released",
        ComputeCapacityClaimState::Expired => "expired",
        ComputeCapacityClaimState::Canceled => "canceled",
    }
}
