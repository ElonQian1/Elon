use std::collections::{BTreeMap, BTreeSet};

use anyhow::{bail, Result};
use rusqlite::{params, OptionalExtension};
use serde::Serialize;
use sha2::{Digest, Sha256};

use crate::compute_federation::capacity::{ComputeCapacityBucketBalance, ComputeCapacityMeterMode};

use super::{compute_capacity_rows::stored_buckets_for_pool_epoch_on, now, Store};

#[derive(Debug, Clone, Serialize)]
pub(crate) struct ComputeCapacityDerivedBalance {
    pub issued_units: i128,
    pub available_units: i128,
    pub held_units: i128,
    pub active_units: i128,
    pub consumed_units: i128,
    pub retired_units: i128,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct ComputeCapacityBucketAudit {
    pub bucket_id: String,
    pub meter: String,
    pub stored: ComputeCapacityBucketBalance,
    pub derived: ComputeCapacityDerivedBalance,
    pub ledger_transaction_count: i64,
    pub derived_through_ledger_sequence: Option<i64>,
    pub issues: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct ComputeCapacityPoolAuditReport {
    pub pool_id: String,
    pub capacity_epoch: i64,
    pub pool_status: String,
    pub current_capacity_epoch: i64,
    pub checked_at: String,
    pub healthy: bool,
    pub transaction_count: i64,
    pub ledger_leg_count: i64,
    pub buckets: Vec<ComputeCapacityBucketAudit>,
    pub issues: Vec<String>,
}

impl Store {
    pub(crate) fn audit_compute_capacity_pool_epoch(
        &self,
        pool_id: &str,
        capacity_epoch: i64,
    ) -> Result<ComputeCapacityPoolAuditReport> {
        if pool_id.trim().is_empty() {
            bail!("容量池 ID 不能为空");
        }
        if capacity_epoch <= 0 {
            bail!("容量池 epoch 必须为正整数");
        }
        let conn = self.conn()?;
        Self::audit_compute_capacity_pool_epoch_on(&conn, pool_id.trim(), capacity_epoch)
    }

    pub(super) fn audit_compute_capacity_pool_epoch_on(
        conn: &rusqlite::Connection,
        pool_id: &str,
        capacity_epoch: i64,
    ) -> Result<ComputeCapacityPoolAuditReport> {
        let pool = conn
            .query_row(
                "SELECT status, current_capacity_epoch
                   FROM compute_capacity_pools WHERE pool_id=?1",
                params![pool_id.trim()],
                |row| Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?)),
            )
            .optional()?
            .ok_or_else(|| anyhow::anyhow!("容量池不存在"))?;
        let stored_buckets =
            stored_buckets_for_pool_epoch_on(conn, pool_id.trim(), capacity_epoch)?;
        let transactions = read_transactions(conn, pool_id.trim(), capacity_epoch)?;
        let legs = read_ledger_legs(conn, pool_id.trim(), capacity_epoch)?;

        let mut issues = Vec::new();
        if stored_buckets.is_empty() {
            issues.push("目标容量池 epoch 没有 bucket".to_string());
        }
        validate_transaction_sequence(&transactions, &mut issues);
        let known_transactions = transactions
            .iter()
            .map(|transaction| {
                (
                    transaction.transaction_id.as_str(),
                    transaction.ledger_sequence,
                )
            })
            .collect::<BTreeMap<_, _>>();
        let mut transaction_ids_with_legs = BTreeSet::new();
        let mut line_shapes = BTreeMap::<(String, i64), LedgerLineShape>::new();
        let mut derived = stored_buckets
            .iter()
            .map(|stored| {
                (
                    stored.balance.binding.bucket_id.clone(),
                    DerivedBalanceWork::default(),
                )
            })
            .collect::<BTreeMap<_, _>>();

        for leg in &legs {
            transaction_ids_with_legs.insert(leg.transaction_id.as_str());
            match known_transactions.get(leg.transaction_id.as_str()) {
                Some(sequence) if *sequence == leg.ledger_sequence => {}
                Some(_) => issues.push(format!(
                    "事务 {} 的 leg 使用了不一致的 ledger_sequence",
                    leg.transaction_id
                )),
                None => issues.push(format!(
                    "发现不属于目标池 epoch 的事务 leg：{}",
                    leg.transaction_id
                )),
            }

            let shape = line_shapes
                .entry((leg.transaction_id.clone(), leg.line_no))
                .or_default();
            shape.bucket_ids.insert(leg.bucket_id.clone());
            shape.meters.insert(leg.meter.clone());
            shape.total_delta = shape
                .total_delta
                .saturating_add(i128::from(leg.delta_units));
            match leg.leg_role.as_str() {
                "from" => {
                    shape.from_count += 1;
                    if leg.delta_units >= 0 {
                        shape.invalid_sign = true;
                    }
                }
                "to" => {
                    shape.to_count += 1;
                    if leg.delta_units <= 0 {
                        shape.invalid_sign = true;
                    }
                }
                _ => shape.invalid_role = true,
            }

            let Some(work) = derived.get_mut(&leg.bucket_id) else {
                issues.push(format!("账本 leg 引用了未知 bucket：{}", leg.bucket_id));
                continue;
            };
            let Some(stored) = stored_buckets
                .iter()
                .find(|stored| stored.balance.binding.bucket_id == leg.bucket_id)
            else {
                issues.push(format!("审计无法读取 bucket {} 的绑定", leg.bucket_id));
                continue;
            };
            if stored.balance.binding.meter != leg.meter {
                issues.push(format!("bucket {} 的账本 meter 不一致", leg.bucket_id));
            }
            if !work.apply_leg(leg) {
                issues.push(format!(
                    "bucket {} 的账本包含未知账户 {}",
                    leg.bucket_id, leg.account
                ));
            }
        }

        for transaction in &transactions {
            if !transaction_ids_with_legs.contains(transaction.transaction_id.as_str()) {
                issues.push(format!(
                    "事务 {} 没有任何 ledger leg",
                    transaction.transaction_id
                ));
            }
        }
        validate_line_shapes(&line_shapes, &mut issues);

        let mut bucket_reports = Vec::with_capacity(stored_buckets.len());
        for stored in stored_buckets {
            let bucket_id = stored.balance.binding.bucket_id.clone();
            let work = derived.remove(&bucket_id).unwrap_or_default();
            bucket_reports.push(build_bucket_report(stored.balance, work));
        }
        let healthy =
            issues.is_empty() && bucket_reports.iter().all(|bucket| bucket.issues.is_empty());
        Ok(ComputeCapacityPoolAuditReport {
            pool_id: pool_id.trim().to_string(),
            capacity_epoch,
            pool_status: pool.0,
            current_capacity_epoch: pool.1,
            checked_at: now(),
            healthy,
            transaction_count: i64::try_from(transactions.len())?,
            ledger_leg_count: i64::try_from(legs.len())?,
            buckets: bucket_reports,
            issues,
        })
    }
}

pub(crate) fn stable_compute_capacity_pool_audit_digest(
    report: &ComputeCapacityPoolAuditReport,
) -> Result<String> {
    let mut value = serde_json::to_value(report)?;
    let object = value
        .as_object_mut()
        .ok_or_else(|| anyhow::anyhow!("容量池账本审计结果不是对象"))?;
    object.remove("checked_at");
    Ok(hex::encode(Sha256::digest(serde_json::to_vec(&value)?)))
}

struct TransactionAuditRow {
    transaction_id: String,
    ledger_sequence: i64,
}

struct LedgerLegAuditRow {
    transaction_id: String,
    ledger_sequence: i64,
    line_no: i64,
    leg_role: String,
    bucket_id: String,
    meter: String,
    account: String,
    delta_units: i64,
}

#[derive(Default)]
struct LedgerLineShape {
    bucket_ids: BTreeSet<String>,
    meters: BTreeSet<String>,
    from_count: i64,
    to_count: i64,
    total_delta: i128,
    invalid_sign: bool,
    invalid_role: bool,
}

#[derive(Default)]
struct DerivedBalanceWork {
    issuance_delta: i128,
    available_units: i128,
    held_units: i128,
    active_units: i128,
    consumed_units: i128,
    retired_units: i128,
    transaction_ids: BTreeSet<String>,
    through_ledger_sequence: Option<i64>,
}

impl DerivedBalanceWork {
    fn apply_leg(&mut self, leg: &LedgerLegAuditRow) -> bool {
        let target = match leg.account.as_str() {
            "issuance" => &mut self.issuance_delta,
            "available" => &mut self.available_units,
            "held" => &mut self.held_units,
            "active" => &mut self.active_units,
            "consumed" => &mut self.consumed_units,
            "retired" => &mut self.retired_units,
            _ => return false,
        };
        *target = target.saturating_add(i128::from(leg.delta_units));
        self.transaction_ids.insert(leg.transaction_id.clone());
        self.through_ledger_sequence = Some(
            self.through_ledger_sequence
                .map_or(leg.ledger_sequence, |current| {
                    current.max(leg.ledger_sequence)
                }),
        );
        true
    }

    fn balance(&self) -> ComputeCapacityDerivedBalance {
        ComputeCapacityDerivedBalance {
            issued_units: -self.issuance_delta,
            available_units: self.available_units,
            held_units: self.held_units,
            active_units: self.active_units,
            consumed_units: self.consumed_units,
            retired_units: self.retired_units,
        }
    }
}

fn read_transactions(
    conn: &rusqlite::Connection,
    pool_id: &str,
    capacity_epoch: i64,
) -> Result<Vec<TransactionAuditRow>> {
    let mut statement = conn.prepare(
        "SELECT transaction_id, ledger_sequence
           FROM compute_capacity_ledger_transactions
          WHERE pool_id=?1 AND capacity_epoch=?2
          ORDER BY ledger_sequence",
    )?;
    let rows = statement
        .query_map(params![pool_id, capacity_epoch], |row| {
            Ok(TransactionAuditRow {
                transaction_id: row.get(0)?,
                ledger_sequence: row.get(1)?,
            })
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(rows)
}

fn read_ledger_legs(
    conn: &rusqlite::Connection,
    pool_id: &str,
    capacity_epoch: i64,
) -> Result<Vec<LedgerLegAuditRow>> {
    let mut statement = conn.prepare(
        "SELECT l.transaction_id, t.ledger_sequence, l.line_no, l.leg_role,
                l.bucket_id, l.meter, l.account, l.delta_units
           FROM compute_capacity_ledger_legs l
           JOIN compute_capacity_ledger_transactions t
             ON t.transaction_id=l.transaction_id
          WHERE t.pool_id=?1 AND t.capacity_epoch=?2
          ORDER BY t.ledger_sequence, l.line_no, l.leg_role",
    )?;
    let rows = statement
        .query_map(params![pool_id, capacity_epoch], |row| {
            Ok(LedgerLegAuditRow {
                transaction_id: row.get(0)?,
                ledger_sequence: row.get(1)?,
                line_no: row.get(2)?,
                leg_role: row.get(3)?,
                bucket_id: row.get(4)?,
                meter: row.get(5)?,
                account: row.get(6)?,
                delta_units: row.get(7)?,
            })
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(rows)
}

fn validate_transaction_sequence(transactions: &[TransactionAuditRow], issues: &mut Vec<String>) {
    for (index, transaction) in transactions.iter().enumerate() {
        let expected = i64::try_from(index).unwrap_or(i64::MAX).saturating_add(1);
        if transaction.ledger_sequence != expected {
            issues.push(format!(
                "事务 {} 的 ledger_sequence 应为 {}，实际为 {}",
                transaction.transaction_id, expected, transaction.ledger_sequence
            ));
        }
    }
}

fn validate_line_shapes(
    line_shapes: &BTreeMap<(String, i64), LedgerLineShape>,
    issues: &mut Vec<String>,
) {
    for ((transaction_id, line_no), shape) in line_shapes {
        if shape.from_count != 1
            || shape.to_count != 1
            || shape.total_delta != 0
            || shape.bucket_ids.len() != 1
            || shape.meters.len() != 1
            || shape.invalid_sign
            || shape.invalid_role
        {
            issues.push(format!(
                "事务 {transaction_id} 的 line {line_no} 不是同 bucket、同 meter、等额双腿零和分录"
            ));
        }
    }
}

fn build_bucket_report(
    stored: ComputeCapacityBucketBalance,
    work: DerivedBalanceWork,
) -> ComputeCapacityBucketAudit {
    let derived = work.balance();
    let mut issues = Vec::new();
    compare_account(
        "issued",
        derived.issued_units,
        stored.issued_units,
        &mut issues,
    );
    compare_account(
        "available",
        derived.available_units,
        stored.available_units,
        &mut issues,
    );
    compare_account("held", derived.held_units, stored.held_units, &mut issues);
    compare_account(
        "active",
        derived.active_units,
        stored.active_units,
        &mut issues,
    );
    compare_account(
        "consumed",
        derived.consumed_units,
        stored.consumed_units,
        &mut issues,
    );
    compare_account(
        "retired",
        derived.retired_units,
        stored.retired_units,
        &mut issues,
    );
    let transaction_count = i64::try_from(work.transaction_ids.len()).unwrap_or(i64::MAX);
    if stored.balance_revision != transaction_count {
        issues.push(format!(
            "balance_revision 应为 {transaction_count}，实际为 {}",
            stored.balance_revision
        ));
    }
    if stored.through_ledger_sequence != work.through_ledger_sequence {
        issues.push(format!(
            "through_ledger_sequence 应为 {:?}，实际为 {:?}",
            work.through_ledger_sequence, stored.through_ledger_sequence
        ));
    }
    validate_derived_conservation(&stored, &derived, &mut issues);
    ComputeCapacityBucketAudit {
        bucket_id: stored.binding.bucket_id.clone(),
        meter: stored.binding.meter.clone(),
        stored,
        derived,
        ledger_transaction_count: transaction_count,
        derived_through_ledger_sequence: work.through_ledger_sequence,
        issues,
    }
}

fn compare_account(label: &str, derived: i128, stored: i64, issues: &mut Vec<String>) {
    if derived != i128::from(stored) {
        issues.push(format!(
            "{label} 从账本推导为 {derived}，物化余额为 {stored}"
        ));
    }
}

fn validate_derived_conservation(
    stored: &ComputeCapacityBucketBalance,
    derived: &ComputeCapacityDerivedBalance,
    issues: &mut Vec<String>,
) {
    for (label, value) in [
        ("issued", derived.issued_units),
        ("available", derived.available_units),
        ("held", derived.held_units),
        ("active", derived.active_units),
        ("consumed", derived.consumed_units),
        ("retired", derived.retired_units),
    ] {
        if value < 0 {
            issues.push(format!("账本推导的 {label} 余额为负数 {value}"));
        }
    }
    let projected = derived
        .available_units
        .saturating_add(derived.held_units)
        .saturating_add(derived.active_units)
        .saturating_add(derived.retired_units)
        .saturating_add(match stored.binding.meter_mode {
            ComputeCapacityMeterMode::Consumable => derived.consumed_units,
            ComputeCapacityMeterMode::Reusable => 0,
        });
    if stored.binding.meter_mode == ComputeCapacityMeterMode::Reusable
        && derived.consumed_units != 0
    {
        issues.push("可复用 meter 不应出现 consumed 余额".to_string());
    }
    if derived.issued_units != projected {
        issues.push(format!(
            "账本推导余额不守恒：issued={}，账户合计={projected}",
            derived.issued_units
        ));
    }
}
