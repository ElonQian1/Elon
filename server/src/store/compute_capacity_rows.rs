use anyhow::{bail, Result};
use rusqlite::{params, Connection, OptionalExtension, Row};

use crate::compute_federation::capacity::{
    ComputeCapacityBucketBalance, ComputeCapacityBucketBinding, ComputeCapacityBucketStatus,
    ComputeCapacityMeterMode, ComputeCapacityPoolBinding,
};
use crate::compute_federation::market::ComputeDeliveryWindowBinding;

pub(super) struct StoredComputeCapacityBucket {
    pub balance: ComputeCapacityBucketBalance,
    pub starts_at: String,
    pub ends_at: String,
}

pub(super) fn stored_bucket_on(
    conn: &Connection,
    bucket_id: &str,
) -> Result<Option<StoredComputeCapacityBucket>> {
    let row = conn
        .query_row(
            &format!("{BUCKET_SELECT} WHERE b.bucket_id=?1"),
            params![bucket_id.trim()],
            bucket_row,
        )
        .optional()?;
    row.map(stored_bucket_from_row).transpose()
}

struct BucketRow {
    bucket_id: String,
    bucket_digest: String,
    pool_id: String,
    capacity_epoch: i64,
    pool_revision: i64,
    pool_digest: String,
    window_id: String,
    window_digest: String,
    starts_at: String,
    ends_at: String,
    meter: String,
    meter_mode: String,
    quantum_units: i64,
    meter_policy_digest: String,
    status: String,
    issued_units: i64,
    available_units: i64,
    held_units: i64,
    active_units: i64,
    consumed_units: i64,
    retired_units: i64,
    balance_revision: i64,
    through_ledger_sequence: Option<i64>,
}

fn bucket_row(row: &Row<'_>) -> rusqlite::Result<BucketRow> {
    Ok(BucketRow {
        bucket_id: row.get(0)?,
        bucket_digest: row.get(1)?,
        pool_id: row.get(2)?,
        capacity_epoch: row.get(3)?,
        pool_revision: row.get(4)?,
        pool_digest: row.get(5)?,
        window_id: row.get(6)?,
        window_digest: row.get(7)?,
        starts_at: row.get(8)?,
        ends_at: row.get(9)?,
        meter: row.get(10)?,
        meter_mode: row.get(11)?,
        quantum_units: row.get(12)?,
        meter_policy_digest: row.get(13)?,
        status: row.get(14)?,
        issued_units: row.get(15)?,
        available_units: row.get(16)?,
        held_units: row.get(17)?,
        active_units: row.get(18)?,
        consumed_units: row.get(19)?,
        retired_units: row.get(20)?,
        balance_revision: row.get(21)?,
        through_ledger_sequence: row.get(22)?,
    })
}

fn stored_bucket_from_row(row: BucketRow) -> Result<StoredComputeCapacityBucket> {
    let meter_mode = match row.meter_mode.as_str() {
        "consumable" => ComputeCapacityMeterMode::Consumable,
        "reusable" => ComputeCapacityMeterMode::Reusable,
        _ => bail!("容量 bucket meter_mode 无效"),
    };
    let status = match row.status.as_str() {
        "open" => ComputeCapacityBucketStatus::Open,
        "closed" => ComputeCapacityBucketStatus::Closed,
        "retired" => ComputeCapacityBucketStatus::Retired,
        _ => bail!("容量 bucket status 无效"),
    };
    Ok(StoredComputeCapacityBucket {
        starts_at: row.starts_at,
        ends_at: row.ends_at,
        balance: ComputeCapacityBucketBalance {
            binding: ComputeCapacityBucketBinding {
                bucket_id: row.bucket_id,
                bucket_digest: row.bucket_digest,
                pool: ComputeCapacityPoolBinding {
                    pool_id: row.pool_id,
                    capacity_epoch: row.capacity_epoch,
                    pool_revision: row.pool_revision,
                    pool_digest: row.pool_digest,
                },
                delivery_window: ComputeDeliveryWindowBinding {
                    window_id: row.window_id,
                    window_digest: row.window_digest,
                },
                meter: row.meter,
                meter_mode,
                quantum_units: row.quantum_units,
                meter_policy_digest: row.meter_policy_digest,
            },
            status,
            issued_units: row.issued_units,
            available_units: row.available_units,
            held_units: row.held_units,
            active_units: row.active_units,
            consumed_units: row.consumed_units,
            retired_units: row.retired_units,
            balance_revision: row.balance_revision,
            through_ledger_sequence: row.through_ledger_sequence,
        },
    })
}

pub(super) fn meter_mode_value(mode: ComputeCapacityMeterMode) -> &'static str {
    match mode {
        ComputeCapacityMeterMode::Consumable => "consumable",
        ComputeCapacityMeterMode::Reusable => "reusable",
    }
}

const BUCKET_SELECT: &str = "SELECT b.bucket_id, b.bucket_digest, b.pool_id, b.capacity_epoch,
            b.pool_revision, pv.pool_digest, b.delivery_window_id,
            b.delivery_window_digest, b.delivery_window_starts_at,
            b.delivery_window_ends_at, b.meter, b.meter_mode, b.quantum_units,
            b.meter_policy_digest, b.status, b.issued_units, b.available_units,
            b.held_units, b.active_units, b.consumed_units, b.retired_units,
            b.balance_revision, b.through_ledger_sequence
       FROM compute_capacity_buckets b
       JOIN compute_capacity_pool_versions pv
         ON pv.pool_id=b.pool_id
        AND pv.capacity_epoch=b.capacity_epoch
        AND pv.pool_revision=b.pool_revision";
