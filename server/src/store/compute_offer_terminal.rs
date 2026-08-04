use anyhow::{anyhow, bail, Context, Result};
use chrono::{DateTime, Utc};
use rusqlite::{params, TransactionBehavior};

use crate::{
    compute_federation::offer::{
        OFFER_STATUS_DRAINING, OFFER_STATUS_EXPIRED, OFFER_STATUS_REVOKED,
    },
    compute_federation_offer_lifecycle_model::ComputeOfferLifecycleReceipt,
};

use super::{
    compute_offer_contract_validation::compute_offer_digest,
    compute_offer_lifecycle::{
        audit_lifecycle_on, lifecycle_by_idempotency_on, lifecycle_by_offer_target_on,
        lifecycle_digest, StoredLifecycleEvent,
    },
    compute_offer_registry::current_registered_offer_on,
    new_id, now, Store,
};

#[derive(Debug, Clone)]
pub(crate) struct TerminateComputeOffer {
    pub offer_id: String,
    pub target_status: String,
    pub expected_offer_version: i64,
    pub expected_offer_digest: String,
    pub reason: String,
    pub idempotency_scope: String,
    pub idempotency_key: String,
    pub changed_by_user_id: String,
}

impl Store {
    pub(crate) fn terminate_compute_offer(
        &self,
        input: TerminateComputeOffer,
    ) -> Result<ComputeOfferLifecycleReceipt> {
        validate_input(&input)?;
        let mut conn = self.conn()?;
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;

        if let Some(existing) = lifecycle_by_idempotency_on(
            &tx,
            input.idempotency_scope.trim(),
            input.idempotency_key.trim(),
        )? {
            validate_replay(&tx, &input, &existing)?;
            tx.commit()?;
            return Ok(existing.into_receipt(true));
        }
        if let Some(existing) =
            lifecycle_by_offer_target_on(&tx, input.offer_id.trim(), input.target_status.trim())?
        {
            validate_replay(&tx, &input, &existing)?;
            tx.commit()?;
            return Ok(existing.into_receipt(true));
        }

        let previous = current_registered_offer_on(&tx, input.offer_id.trim())?
            .ok_or_else(|| anyhow!("算力 Offer 不存在"))?;
        if previous.offer.status != OFFER_STATUS_DRAINING
            || previous.offer.offer_version != input.expected_offer_version
            || previous.offer.offer_digest != input.expected_offer_digest
        {
            bail!("只有当前版本和摘要精确匹配的 draining Offer 可以进入终态");
        }
        ensure_no_live_reservations(&tx, input.offer_id.trim())?;
        if input.target_status == OFFER_STATUS_EXPIRED {
            let valid_until = DateTime::parse_from_rfc3339(&previous.offer.valid_until)
                .context("Offer 有效期不是 RFC3339")?;
            if valid_until > Utc::now() {
                bail!("Offer 尚未超过有效期，提前退出必须使用 revoked");
            }
        }

        let mut terminal = previous.offer.clone();
        terminal.offer_version = terminal
            .offer_version
            .checked_add(1)
            .context("算力 Offer 版本溢出")?;
        terminal.status = input.target_status.clone();
        terminal.offer_digest.clear();
        terminal.offer_digest = compute_offer_digest(&terminal)?;
        let terminal_receipt = self.register_compute_offer_on(&tx, &terminal)?;
        if terminal_receipt.replayed || terminal_receipt.offer != terminal {
            bail!("Offer 终态版本未按请求创建");
        }

        let changed_at = now();
        let event_id = new_id("compute_offer_lifecycle");
        let event_digest = lifecycle_digest(
            &event_id,
            &terminal.offer_id,
            &terminal.provider_id,
            &terminal.capacity_pool.pool_id,
            OFFER_STATUS_DRAINING,
            &terminal.status,
            previous.offer.offer_version,
            &previous.offer.offer_digest,
            terminal.offer_version,
            &terminal.offer_digest,
            input.reason.trim(),
            input.changed_by_user_id.trim(),
            &changed_at,
        )?;
        tx.execute(
            "INSERT INTO compute_offer_lifecycle_events (
                event_id, offer_id, provider_id, pool_id,
                previous_status, target_status,
                previous_offer_version, previous_offer_digest,
                target_offer_version, target_offer_digest,
                reason, event_digest, idempotency_scope, idempotency_key,
                changed_by_user_id, changed_at, created_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10,
                       ?11, ?12, ?13, ?14, ?15, ?16, ?16)",
            params![
                event_id,
                terminal.offer_id,
                terminal.provider_id,
                terminal.capacity_pool.pool_id,
                OFFER_STATUS_DRAINING,
                terminal.status,
                previous.offer.offer_version,
                previous.offer.offer_digest,
                terminal.offer_version,
                terminal.offer_digest,
                input.reason.trim(),
                event_digest,
                input.idempotency_scope.trim(),
                input.idempotency_key.trim(),
                input.changed_by_user_id.trim(),
                changed_at,
            ],
        )?;
        let stored =
            lifecycle_by_offer_target_on(&tx, input.offer_id.trim(), input.target_status.trim())?
                .ok_or_else(|| anyhow!("Offer 终态回执写入后无法读取"))?;
        audit_lifecycle_on(&tx, &stored)?;
        tx.commit()?;
        Ok(stored.into_receipt(false))
    }

    pub(crate) fn compute_offer_terminal_event(
        &self,
        offer_id: &str,
        target_status: &str,
    ) -> Result<Option<ComputeOfferLifecycleReceipt>> {
        validate_exact("Offer ID", offer_id, 200)?;
        validate_target_status(target_status)?;
        let conn = self.conn()?;
        lifecycle_by_offer_target_on(&conn, offer_id.trim(), target_status.trim())?
            .map(|stored| {
                audit_lifecycle_on(&conn, &stored)?;
                Ok(stored.into_receipt(false))
            })
            .transpose()
    }
}

fn ensure_no_live_reservations(conn: &rusqlite::Connection, offer_id: &str) -> Result<()> {
    let live_count = conn.query_row(
        "SELECT COUNT(*) FROM compute_reservations
          WHERE offer_id=?1 AND status IN ('pending', 'active')",
        params![offer_id],
        |row| row.get::<_, i64>(0),
    )?;
    if live_count > 0 {
        bail!("Offer 仍有 {live_count} 个 pending/active Reservation，拒绝进入终态");
    }
    Ok(())
}

fn validate_replay(
    conn: &rusqlite::Connection,
    input: &TerminateComputeOffer,
    existing: &StoredLifecycleEvent,
) -> Result<()> {
    if existing.offer_id != input.offer_id.trim()
        || existing.previous_offer_version != input.expected_offer_version
        || existing.previous_offer_digest != input.expected_offer_digest.trim()
        || existing.reason != input.reason.trim()
        || existing.target_status != input.target_status.trim()
    {
        bail!("相同终态幂等键或 Offer 已绑定不同请求");
    }
    audit_lifecycle_on(conn, existing)
}

fn validate_input(input: &TerminateComputeOffer) -> Result<()> {
    for (label, value, max_len) in [
        ("Offer ID", input.offer_id.as_str(), 200),
        ("Offer 终态原因", input.reason.as_str(), 1000),
        ("Offer 终态幂等范围", input.idempotency_scope.as_str(), 240),
        ("Offer 终态幂等键", input.idempotency_key.as_str(), 160),
        ("Offer 终态执行人", input.changed_by_user_id.as_str(), 160),
    ] {
        validate_exact(label, value, max_len)?;
    }
    validate_target_status(&input.target_status)?;
    if input.expected_offer_version <= 0 {
        bail!("预期 Offer 版本必须为正整数");
    }
    validate_digest("预期 Offer 摘要", &input.expected_offer_digest)
}

fn validate_target_status(value: &str) -> Result<()> {
    if !matches!(value, OFFER_STATUS_EXPIRED | OFFER_STATUS_REVOKED) {
        bail!("Offer 终态只允许 expired 或 revoked");
    }
    Ok(())
}

fn validate_digest(label: &str, value: &str) -> Result<()> {
    if value.len() != 64
        || value != value.to_ascii_lowercase()
        || !value.bytes().all(|byte| byte.is_ascii_hexdigit())
    {
        bail!("{label}必须是 64 位小写十六进制 SHA-256");
    }
    Ok(())
}

fn validate_exact(label: &str, value: &str, max_len: usize) -> Result<()> {
    if value.trim().is_empty()
        || value != value.trim()
        || value.chars().count() > max_len
        || value.chars().any(char::is_control)
    {
        bail!("{label}为空、过长或包含无效字符");
    }
    Ok(())
}
