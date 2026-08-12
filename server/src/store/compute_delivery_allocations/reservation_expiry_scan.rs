use anyhow::{bail, Result};
use rusqlite::{params, OptionalExtension, TransactionBehavior};

use super::{
    reservation_expiry_recovery::{due_reservations_on, DueReservationCandidate},
    ComputeDeliveryAllocationReservationExpiryReport,
};
use crate::store::{new_id, now, Store};

const CHECKPOINT_KEY: &str = "delivery_allocation_reservation_expiry_v1";
const CHECKPOINT_EFFECT_ADVANCED: &str = "advanced";
const CHECKPOINT_EFFECT_CLEARED: &str = "cleared";
const CHECKPOINT_EFFECT_SUPERSEDED: &str = "superseded";

#[derive(Debug, Clone)]
struct ExpiryScanCheckpoint {
    sweep_id: String,
    sweep_cutoff: String,
    last_expires_at: Option<String>,
    last_reservation_id: Option<String>,
    revision: i64,
}

#[derive(Debug, Clone)]
pub(crate) struct ComputeDeliveryAllocationReservationExpiryWorkerPageReport {
    pub sweep_cutoff: String,
    pub selected_count: usize,
    pub expired_count: usize,
    pub replayed_count: usize,
    pub blocked_count: usize,
    pub failed_count: usize,
    pub sweep_completed: bool,
    pub checkpoint_effect: &'static str,
}

impl Store {
    pub(crate) fn expire_due_compute_delivery_allocation_reservations_worker_page(
        &self,
        limit: usize,
    ) -> Result<ComputeDeliveryAllocationReservationExpiryWorkerPageReport> {
        if !(1..=100).contains(&limit) {
            bail!("DeliveryAllocation Reservation 到期恢复 worker limit 必须在 1..=100");
        }
        let (checkpoint, candidates) = self.load_expiry_scan_page(limit)?;
        let expiry_report = self
            .expire_due_reservation_candidates(checkpoint.sweep_cutoff.clone(), candidates.clone());
        let (sweep_completed, checkpoint_effect) = if let Some(last) = candidates.last() {
            let advanced = self.advance_expiry_scan_checkpoint(&checkpoint, last)?;
            (
                false,
                if advanced {
                    CHECKPOINT_EFFECT_ADVANCED
                } else {
                    CHECKPOINT_EFFECT_SUPERSEDED
                },
            )
        } else {
            let cleared = self.clear_expiry_scan_checkpoint(&checkpoint)?;
            (
                cleared,
                if cleared {
                    CHECKPOINT_EFFECT_CLEARED
                } else {
                    CHECKPOINT_EFFECT_SUPERSEDED
                },
            )
        };
        Ok(worker_report(
            checkpoint.sweep_cutoff,
            expiry_report,
            sweep_completed,
            checkpoint_effect,
        ))
    }

    fn load_expiry_scan_page(
        &self,
        limit: usize,
    ) -> Result<(ExpiryScanCheckpoint, Vec<DueReservationCandidate>)> {
        let mut connection = self.conn()?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let checkpoint = load_checkpoint_on(&transaction)?.unwrap_or_else(new_checkpoint);
        transaction.execute(
            "INSERT OR IGNORE INTO compute_delivery_allocation_expiry_worker_checkpoint (
                 checkpoint_key, sweep_id, sweep_cutoff, last_expires_at,
                 last_reservation_id, revision, updated_at
             ) VALUES (?1, ?2, ?3, NULL, NULL, 1, ?3)",
            params![CHECKPOINT_KEY, checkpoint.sweep_id, checkpoint.sweep_cutoff],
        )?;
        let checkpoint = load_checkpoint_on(&transaction)?.ok_or_else(|| {
            anyhow::anyhow!("DeliveryAllocation 到期恢复 worker checkpoint 初始化失败")
        })?;
        validate_checkpoint(&checkpoint)?;
        let candidates = due_reservations_on(
            &transaction,
            &checkpoint.sweep_cutoff,
            checkpoint.last_expires_at.as_deref(),
            checkpoint.last_reservation_id.as_deref(),
            limit,
        )?;
        transaction.commit()?;
        Ok((checkpoint, candidates))
    }

    fn advance_expiry_scan_checkpoint(
        &self,
        expected: &ExpiryScanCheckpoint,
        last: &DueReservationCandidate,
    ) -> Result<bool> {
        let connection = self.conn()?;
        let changed = connection.execute(
            "UPDATE compute_delivery_allocation_expiry_worker_checkpoint
                SET last_expires_at=?1, last_reservation_id=?2,
                    revision=revision+1,
                    updated_at=CASE
                        WHEN julianday(?3)>=julianday(updated_at) THEN ?3
                        ELSE updated_at
                    END
              WHERE checkpoint_key=?4 AND sweep_id=?5 AND sweep_cutoff=?6
                AND revision=?7
                AND ((last_expires_at IS NULL AND ?8 IS NULL)
                     OR last_expires_at=?8)
                AND ((last_reservation_id IS NULL AND ?9 IS NULL)
                     OR last_reservation_id=?9)",
            params![
                last.expires_at,
                last.reservation_id,
                now(),
                CHECKPOINT_KEY,
                expected.sweep_id,
                expected.sweep_cutoff,
                expected.revision,
                expected.last_expires_at,
                expected.last_reservation_id,
            ],
        )?;
        Ok(changed == 1)
    }

    fn clear_expiry_scan_checkpoint(&self, expected: &ExpiryScanCheckpoint) -> Result<bool> {
        let connection = self.conn()?;
        let changed = connection.execute(
            "DELETE FROM compute_delivery_allocation_expiry_worker_checkpoint
              WHERE checkpoint_key=?1 AND sweep_id=?2 AND sweep_cutoff=?3
                AND revision=?4
                AND ((last_expires_at IS NULL AND ?5 IS NULL)
                     OR last_expires_at=?5)
                AND ((last_reservation_id IS NULL AND ?6 IS NULL)
                     OR last_reservation_id=?6)",
            params![
                CHECKPOINT_KEY,
                expected.sweep_id,
                expected.sweep_cutoff,
                expected.revision,
                expected.last_expires_at,
                expected.last_reservation_id,
            ],
        )?;
        Ok(changed == 1)
    }
}

fn load_checkpoint_on(connection: &rusqlite::Connection) -> Result<Option<ExpiryScanCheckpoint>> {
    connection
        .query_row(
            "SELECT sweep_id, sweep_cutoff, last_expires_at,
                    last_reservation_id, revision
               FROM compute_delivery_allocation_expiry_worker_checkpoint
              WHERE checkpoint_key=?1",
            [CHECKPOINT_KEY],
            |row| {
                Ok(ExpiryScanCheckpoint {
                    sweep_id: row.get(0)?,
                    sweep_cutoff: row.get(1)?,
                    last_expires_at: row.get(2)?,
                    last_reservation_id: row.get(3)?,
                    revision: row.get(4)?,
                })
            },
        )
        .optional()
        .map_err(Into::into)
}

fn new_checkpoint() -> ExpiryScanCheckpoint {
    ExpiryScanCheckpoint {
        sweep_id: new_id("cdax"),
        sweep_cutoff: now(),
        last_expires_at: None,
        last_reservation_id: None,
        revision: 1,
    }
}

fn validate_checkpoint(checkpoint: &ExpiryScanCheckpoint) -> Result<()> {
    if checkpoint.sweep_id.trim().is_empty()
        || checkpoint.sweep_cutoff.trim().is_empty()
        || checkpoint.revision <= 0
        || checkpoint.last_expires_at.is_some() != checkpoint.last_reservation_id.is_some()
    {
        bail!("DeliveryAllocation 到期恢复 worker checkpoint 无效");
    }
    Ok(())
}

fn worker_report(
    sweep_cutoff: String,
    report: ComputeDeliveryAllocationReservationExpiryReport,
    sweep_completed: bool,
    checkpoint_effect: &'static str,
) -> ComputeDeliveryAllocationReservationExpiryWorkerPageReport {
    ComputeDeliveryAllocationReservationExpiryWorkerPageReport {
        sweep_cutoff,
        selected_count: report.selected_count,
        expired_count: report.expired_count,
        replayed_count: report.replayed_count,
        blocked_count: report.blocked_count,
        failed_count: report.failed_count,
        sweep_completed,
        checkpoint_effect,
    }
}
