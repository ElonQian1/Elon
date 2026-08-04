use anyhow::{anyhow, bail, Context, Result};
use rusqlite::{params, Connection, OptionalExtension, Row, TransactionBehavior};
use sha2::{Digest, Sha256};

use crate::{
    compute_federation::offer::{OFFER_STATUS_ACTIVE, OFFER_STATUS_DRAINING},
    compute_federation_offer_lifecycle_model::{
        ComputeOfferLifecycleReceipt, COMPUTE_OFFER_LIFECYCLE_SCHEMA,
    },
};

use super::{
    compute_offer_contract_validation::compute_offer_digest,
    compute_offer_registry::{current_registered_offer_on, registered_offer_version_on},
    new_id, now, Store,
};

#[derive(Debug, Clone)]
pub(crate) struct DrainComputeOffer {
    pub offer_id: String,
    pub expected_offer_version: i64,
    pub expected_offer_digest: String,
    pub reason: String,
    pub idempotency_scope: String,
    pub idempotency_key: String,
    pub changed_by_user_id: String,
}

impl Store {
    pub(crate) fn drain_compute_offer(
        &self,
        input: DrainComputeOffer,
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
            lifecycle_by_offer_target_on(&tx, input.offer_id.trim(), OFFER_STATUS_DRAINING)?
        {
            validate_replay(&tx, &input, &existing)?;
            tx.commit()?;
            return Ok(existing.into_receipt(true));
        }

        let previous = current_registered_offer_on(&tx, input.offer_id.trim())?
            .ok_or_else(|| anyhow!("算力 Offer 不存在"))?;
        if previous.offer.status != OFFER_STATUS_ACTIVE
            || previous.offer.offer_version != input.expected_offer_version
            || previous.offer.offer_digest != input.expected_offer_digest
        {
            bail!("只有当前版本和摘要精确匹配的 active Offer 可以转为 draining");
        }

        let mut draining = previous.offer.clone();
        draining.offer_version = draining
            .offer_version
            .checked_add(1)
            .context("算力 Offer 版本溢出")?;
        draining.status = OFFER_STATUS_DRAINING.to_string();
        draining.offer_digest.clear();
        draining.offer_digest = compute_offer_digest(&draining)?;
        let draining_receipt = self.register_compute_offer_on(&tx, &draining)?;
        if draining_receipt.replayed || draining_receipt.offer != draining {
            bail!("draining Offer 未按退场合同创建");
        }

        let changed_at = now();
        let event_id = new_id("compute_offer_lifecycle");
        let event_digest = lifecycle_digest(
            &event_id,
            &draining.offer_id,
            &draining.provider_id,
            &draining.capacity_pool.pool_id,
            OFFER_STATUS_ACTIVE,
            OFFER_STATUS_DRAINING,
            previous.offer.offer_version,
            &previous.offer.offer_digest,
            draining.offer_version,
            &draining.offer_digest,
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
                draining.offer_id,
                draining.provider_id,
                draining.capacity_pool.pool_id,
                OFFER_STATUS_ACTIVE,
                OFFER_STATUS_DRAINING,
                previous.offer.offer_version,
                previous.offer.offer_digest,
                draining.offer_version,
                draining.offer_digest,
                input.reason.trim(),
                event_digest,
                input.idempotency_scope.trim(),
                input.idempotency_key.trim(),
                input.changed_by_user_id.trim(),
                changed_at,
            ],
        )?;
        let stored =
            lifecycle_by_offer_target_on(&tx, input.offer_id.trim(), OFFER_STATUS_DRAINING)?
                .ok_or_else(|| anyhow!("Offer draining 回执写入后无法读取"))?;
        audit_lifecycle_on(&tx, &stored)?;
        tx.commit()?;
        Ok(stored.into_receipt(false))
    }

    pub(crate) fn compute_offer_drain_event(
        &self,
        offer_id: &str,
    ) -> Result<Option<ComputeOfferLifecycleReceipt>> {
        validate_exact("Offer ID", offer_id, 200)?;
        let conn = self.conn()?;
        lifecycle_by_offer_target_on(&conn, offer_id.trim(), OFFER_STATUS_DRAINING)?
            .map(|stored| {
                audit_lifecycle_on(&conn, &stored)?;
                Ok(stored.into_receipt(false))
            })
            .transpose()
    }
}

#[derive(Debug, Clone)]
struct StoredLifecycleEvent {
    event_id: String,
    offer_id: String,
    provider_id: String,
    pool_id: String,
    previous_status: String,
    target_status: String,
    previous_offer_version: i64,
    previous_offer_digest: String,
    target_offer_version: i64,
    target_offer_digest: String,
    reason: String,
    event_digest: String,
    changed_by_user_id: String,
    changed_at: String,
}

impl StoredLifecycleEvent {
    fn into_receipt(self, replayed: bool) -> ComputeOfferLifecycleReceipt {
        ComputeOfferLifecycleReceipt {
            schema: COMPUTE_OFFER_LIFECYCLE_SCHEMA,
            event_id: self.event_id,
            offer_id: self.offer_id,
            provider_id: self.provider_id,
            pool_id: self.pool_id,
            previous_status: self.previous_status,
            target_status: self.target_status,
            previous_offer_version: self.previous_offer_version,
            previous_offer_digest: self.previous_offer_digest,
            target_offer_version: self.target_offer_version,
            target_offer_digest: self.target_offer_digest,
            reason: self.reason,
            event_digest: self.event_digest,
            changed_by_user_id: self.changed_by_user_id,
            changed_at: self.changed_at,
            replayed,
            quote_candidate_effect: "excluded_from_new_quotes",
            reservation_effect: "preserved",
            attempt_effect: "none_direct",
            funds_effect: "none",
        }
    }
}

fn validate_replay(
    conn: &Connection,
    input: &DrainComputeOffer,
    existing: &StoredLifecycleEvent,
) -> Result<()> {
    if existing.offer_id != input.offer_id.trim()
        || existing.previous_offer_version != input.expected_offer_version
        || existing.previous_offer_digest != input.expected_offer_digest.trim()
        || existing.reason != input.reason.trim()
        || existing.target_status != OFFER_STATUS_DRAINING
    {
        bail!("相同退场幂等键或 Offer 已绑定不同请求");
    }
    audit_lifecycle_on(conn, existing)
}

fn audit_lifecycle_on(conn: &Connection, stored: &StoredLifecycleEvent) -> Result<()> {
    let previous =
        registered_offer_version_on(conn, &stored.offer_id, stored.previous_offer_version)?
            .ok_or_else(|| anyhow!("Offer 退场前历史版本不存在"))?;
    let target = registered_offer_version_on(conn, &stored.offer_id, stored.target_offer_version)?
        .ok_or_else(|| anyhow!("Offer 退场后历史版本不存在"))?;
    let expected_digest = lifecycle_digest(
        &stored.event_id,
        &stored.offer_id,
        &stored.provider_id,
        &stored.pool_id,
        &stored.previous_status,
        &stored.target_status,
        stored.previous_offer_version,
        &stored.previous_offer_digest,
        stored.target_offer_version,
        &stored.target_offer_digest,
        &stored.reason,
        &stored.changed_by_user_id,
        &stored.changed_at,
    )?;
    if previous.offer.status != stored.previous_status
        || target.offer.status != stored.target_status
        || previous.offer.offer_digest != stored.previous_offer_digest
        || target.offer.offer_digest != stored.target_offer_digest
        || previous.offer.provider_id != stored.provider_id
        || target.offer.provider_id != stored.provider_id
        || previous.offer.capacity_pool.pool_id != stored.pool_id
        || target.offer.capacity_pool.pool_id != stored.pool_id
        || stored.target_offer_version != stored.previous_offer_version + 1
        || expected_digest != stored.event_digest
    {
        bail!("Offer 生命周期回执与不可变历史版本不一致");
    }
    Ok(())
}

fn lifecycle_by_idempotency_on(
    conn: &Connection,
    scope: &str,
    key: &str,
) -> Result<Option<StoredLifecycleEvent>> {
    lifecycle_on(
        conn,
        "WHERE idempotency_scope=?1 AND idempotency_key=?2",
        params![scope, key],
    )
}

fn lifecycle_by_offer_target_on(
    conn: &Connection,
    offer_id: &str,
    target_status: &str,
) -> Result<Option<StoredLifecycleEvent>> {
    lifecycle_on(
        conn,
        "WHERE offer_id=?1 AND target_status=?2",
        params![offer_id, target_status],
    )
}

fn lifecycle_on<P: rusqlite::Params>(
    conn: &Connection,
    filter: &str,
    parameters: P,
) -> Result<Option<StoredLifecycleEvent>> {
    conn.query_row(
        &format!(
            "SELECT event_id, offer_id, provider_id, pool_id,
                    previous_status, target_status,
                    previous_offer_version, previous_offer_digest,
                    target_offer_version, target_offer_digest,
                    reason, event_digest, changed_by_user_id, changed_at
               FROM compute_offer_lifecycle_events {filter}"
        ),
        parameters,
        stored_lifecycle_from_row,
    )
    .optional()
    .map_err(Into::into)
}

fn stored_lifecycle_from_row(row: &Row<'_>) -> rusqlite::Result<StoredLifecycleEvent> {
    Ok(StoredLifecycleEvent {
        event_id: row.get(0)?,
        offer_id: row.get(1)?,
        provider_id: row.get(2)?,
        pool_id: row.get(3)?,
        previous_status: row.get(4)?,
        target_status: row.get(5)?,
        previous_offer_version: row.get(6)?,
        previous_offer_digest: row.get(7)?,
        target_offer_version: row.get(8)?,
        target_offer_digest: row.get(9)?,
        reason: row.get(10)?,
        event_digest: row.get(11)?,
        changed_by_user_id: row.get(12)?,
        changed_at: row.get(13)?,
    })
}

#[allow(clippy::too_many_arguments)]
fn lifecycle_digest(
    event_id: &str,
    offer_id: &str,
    provider_id: &str,
    pool_id: &str,
    previous_status: &str,
    target_status: &str,
    previous_offer_version: i64,
    previous_offer_digest: &str,
    target_offer_version: i64,
    target_offer_digest: &str,
    reason: &str,
    changed_by_user_id: &str,
    changed_at: &str,
) -> Result<String> {
    let value = serde_json::json!({
        "schema":COMPUTE_OFFER_LIFECYCLE_SCHEMA,
        "event_id":event_id,
        "offer_id":offer_id,
        "provider_id":provider_id,
        "pool_id":pool_id,
        "previous_status":previous_status,
        "target_status":target_status,
        "previous_offer_version":previous_offer_version,
        "previous_offer_digest":previous_offer_digest,
        "target_offer_version":target_offer_version,
        "target_offer_digest":target_offer_digest,
        "reason":reason,
        "changed_by_user_id":changed_by_user_id,
        "changed_at":changed_at,
    });
    Ok(hex::encode(Sha256::digest(serde_json::to_vec(&value)?)))
}

fn validate_input(input: &DrainComputeOffer) -> Result<()> {
    for (label, value, max_len) in [
        ("Offer ID", input.offer_id.as_str(), 200),
        ("Offer 退场原因", input.reason.as_str(), 1000),
        ("Offer 退场幂等范围", input.idempotency_scope.as_str(), 240),
        ("Offer 退场幂等键", input.idempotency_key.as_str(), 160),
        ("Offer 退场执行人", input.changed_by_user_id.as_str(), 160),
    ] {
        validate_exact(label, value, max_len)?;
    }
    if input.expected_offer_version <= 0 {
        bail!("预期 Offer 版本必须为正整数");
    }
    validate_digest("预期 Offer 摘要", &input.expected_offer_digest)
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
