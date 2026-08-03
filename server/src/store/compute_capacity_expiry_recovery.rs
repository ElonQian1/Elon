use anyhow::{anyhow, bail, Result};
use chrono::{DateTime, Utc};
use rusqlite::params;
use serde::Serialize;

use super::{now, ComputeCapacityClaimTerminalAction, FinishComputeCapacityClaim, Store};

const EXPIRY_RECOVERY_SCOPE: &str = "capacity_expiry_recovery_v1";
const MAX_EXPIRY_RECOVERY_BATCH: i64 = 500;

#[derive(Debug, Clone, Serialize)]
pub(crate) struct ComputeCapacityExpiryRecoveryItem {
    pub claim_id: String,
    pub expected_revision: i64,
    pub expires_at: String,
    pub status: String,
    pub replayed: bool,
    pub transaction_id: Option<String>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct ComputeCapacityExpiryRecoveryReport {
    pub cutoff_at: String,
    pub selected_count: i64,
    pub expired_count: i64,
    pub replayed_count: i64,
    pub failed_count: i64,
    pub items: Vec<ComputeCapacityExpiryRecoveryItem>,
}

impl Store {
    pub(crate) fn recover_expired_compute_capacity_claims(
        &self,
        cutoff_at: &str,
        limit: i64,
    ) -> Result<ComputeCapacityExpiryRecoveryReport> {
        let recovery_started_at = now();
        let cutoff_at = validate_recovery_input(cutoff_at, limit, &recovery_started_at)?;
        let candidates = {
            let conn = self.conn()?;
            let mut statement = conn.prepare(
                "SELECT claims.claim_id, claims.revision, claims.expires_at
                   FROM compute_capacity_claims AS claims
                  WHERE claims.status='held'
                    AND NOT EXISTS (
                        SELECT 1
                          FROM compute_capacity_ledger_transactions AS held_transaction
                         WHERE held_transaction.claim_id=claims.claim_id
                           AND held_transaction.claim_effect='held'
                           AND held_transaction.reservation_id IS NOT NULL
                    )
                    AND claims.expires_at IS NOT NULL AND claims.expires_at<=?1
                  ORDER BY claims.expires_at, claims.claim_id LIMIT ?2",
            )?;
            let rows = statement
                .query_map(params![cutoff_at, limit], |row| {
                    Ok(ExpiryCandidate {
                        claim_id: row.get(0)?,
                        revision: row.get(1)?,
                        expires_at: row.get(2)?,
                    })
                })?
                .collect::<rusqlite::Result<Vec<_>>>()?;
            rows
        };

        let mut items = Vec::with_capacity(candidates.len());
        let mut expired_count = 0_i64;
        let mut replayed_count = 0_i64;
        let mut failed_count = 0_i64;
        for candidate in candidates {
            let idempotency_key = format!(
                "{}:r{}:{}",
                candidate.claim_id, candidate.revision, candidate.expires_at
            );
            match self.finish_compute_capacity_claim(FinishComputeCapacityClaim {
                claim_id: candidate.claim_id.clone(),
                expected_revision: candidate.revision,
                action: ComputeCapacityClaimTerminalAction::Expire,
                idempotency_scope: EXPIRY_RECOVERY_SCOPE.to_string(),
                idempotency_key,
                occurred_at: candidate.expires_at.clone(),
            }) {
                Ok(receipt) => {
                    expired_count = expired_count.saturating_add(1);
                    if receipt.replayed {
                        replayed_count = replayed_count.saturating_add(1);
                    }
                    items.push(ComputeCapacityExpiryRecoveryItem {
                        claim_id: candidate.claim_id,
                        expected_revision: candidate.revision,
                        expires_at: candidate.expires_at,
                        status: receipt.state,
                        replayed: receipt.replayed,
                        transaction_id: Some(receipt.ledger.transaction_id),
                        error: None,
                    });
                }
                Err(error) => {
                    failed_count = failed_count.saturating_add(1);
                    items.push(ComputeCapacityExpiryRecoveryItem {
                        claim_id: candidate.claim_id,
                        expected_revision: candidate.revision,
                        expires_at: candidate.expires_at,
                        status: "failed".to_string(),
                        replayed: false,
                        transaction_id: None,
                        error: Some(error.to_string()),
                    });
                }
            }
        }
        Ok(ComputeCapacityExpiryRecoveryReport {
            cutoff_at,
            selected_count: i64::try_from(items.len())?,
            expired_count,
            replayed_count,
            failed_count,
            items,
        })
    }
}

struct ExpiryCandidate {
    claim_id: String,
    revision: i64,
    expires_at: String,
}

fn validate_recovery_input(
    cutoff_at: &str,
    limit: i64,
    recovery_started_at: &str,
) -> Result<String> {
    if cutoff_at.trim().is_empty() {
        bail!("容量 Claim 到期恢复截止时间不能为空");
    }
    if !(1..=MAX_EXPIRY_RECOVERY_BATCH).contains(&limit) {
        bail!("容量 Claim 到期恢复批量必须在 1 到 {MAX_EXPIRY_RECOVERY_BATCH} 之间");
    }
    let parsed = DateTime::parse_from_rfc3339(cutoff_at.trim())
        .map_err(|_| anyhow!("容量 Claim 到期恢复截止时间不是 RFC3339"))?;
    if parsed.offset().local_minus_utc() != 0 {
        bail!("容量 Claim 到期恢复截止时间必须使用 UTC 时区");
    }
    let recovery_started_at = DateTime::parse_from_rfc3339(recovery_started_at)
        .map_err(|_| anyhow!("容量 Claim 到期恢复记录时间不是 RFC3339"))?;
    if recovery_started_at.offset().local_minus_utc() != 0 {
        bail!("容量 Claim 到期恢复记录时间必须使用 UTC 时区");
    }
    if parsed > recovery_started_at {
        bail!("容量 Claim 到期恢复截止时间不能晚于当前记录时间");
    }
    Ok(parsed.with_timezone(&Utc).to_rfc3339())
}
