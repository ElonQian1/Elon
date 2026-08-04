use anyhow::{anyhow, bail, Context, Result};
use rusqlite::{params, Connection, OptionalExtension, Row, TransactionBehavior};
use sha2::{Digest, Sha256};

use crate::{
    compute_federation::offer::{OFFER_STATUS_ACTIVE, OFFER_STATUS_DRAFT},
    compute_federation_offer_publication_model::{
        ComputeOfferPublicationReceipt, COMPUTE_OFFER_PUBLICATION_SCHEMA,
    },
};

use super::{
    compute_offer_contract_validation::compute_offer_digest,
    compute_offer_registry::{current_registered_offer_on, registered_offer_version_on},
    new_id, now, Store,
};

#[derive(Debug, Clone)]
pub(crate) struct PublishComputeOfferDraft {
    pub offer_id: String,
    pub expected_offer_version: i64,
    pub expected_offer_digest: String,
    pub idempotency_scope: String,
    pub idempotency_key: String,
    pub approved_by_user_id: String,
}

impl Store {
    pub(crate) fn publish_compute_offer_draft(
        &self,
        input: PublishComputeOfferDraft,
    ) -> Result<ComputeOfferPublicationReceipt> {
        validate_input(&input)?;
        let mut conn = self.conn()?;
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;

        if let Some(existing) = publication_by_idempotency_on(
            &tx,
            input.idempotency_scope.trim(),
            input.idempotency_key.trim(),
        )? {
            validate_replay(&tx, &input, &existing)?;
            tx.commit()?;
            return Ok(existing.into_receipt(true));
        }
        if let Some(existing) = publication_by_offer_on(&tx, input.offer_id.trim())? {
            validate_replay(&tx, &input, &existing)?;
            tx.commit()?;
            return Ok(existing.into_receipt(true));
        }

        let source = current_registered_offer_on(&tx, input.offer_id.trim())?
            .ok_or_else(|| anyhow!("算力 Offer 不存在"))?;
        if source.offer.status != OFFER_STATUS_DRAFT
            || source.offer.offer_version != input.expected_offer_version
            || source.offer.offer_digest != input.expected_offer_digest
        {
            bail!("只有当前版本和摘要精确匹配的 draft Offer 可以发布");
        }

        let mut active = source.offer.clone();
        active.offer_version = active
            .offer_version
            .checked_add(1)
            .context("算力 Offer 版本溢出")?;
        active.status = OFFER_STATUS_ACTIVE.to_string();
        active.offer_digest.clear();
        active.offer_digest = compute_offer_digest(&active)?;
        let active_receipt = self.register_compute_offer_on(&tx, &active)?;
        if active_receipt.replayed || active_receipt.offer != active {
            bail!("active Offer 未按审核合同创建");
        }

        let published_at = now();
        let publication_id = new_id("compute_offer_publication");
        let publication_digest = publication_digest(
            &publication_id,
            &active.offer_id,
            &active.provider_id,
            &active.capacity_pool.pool_id,
            source.offer.offer_version,
            &source.offer.offer_digest,
            active.offer_version,
            &active.offer_digest,
            active_receipt.provider_policy_revision,
            &active_receipt.provider_digest,
            input.approved_by_user_id.trim(),
            &published_at,
        )?;
        tx.execute(
            "INSERT INTO compute_offer_publications (
                publication_id, offer_id, provider_id, pool_id,
                source_offer_version, source_offer_digest,
                active_offer_version, active_offer_digest,
                provider_policy_revision, provider_digest,
                publication_digest, idempotency_scope, idempotency_key,
                approved_by_user_id, published_at, created_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?15)",
            params![
                publication_id,
                active.offer_id,
                active.provider_id,
                active.capacity_pool.pool_id,
                source.offer.offer_version,
                source.offer.offer_digest,
                active.offer_version,
                active.offer_digest,
                active_receipt.provider_policy_revision,
                active_receipt.provider_digest,
                publication_digest,
                input.idempotency_scope.trim(),
                input.idempotency_key.trim(),
                input.approved_by_user_id.trim(),
                published_at,
            ],
        )?;
        let stored = publication_by_offer_on(&tx, &active.offer_id)?
            .ok_or_else(|| anyhow!("Offer 发布回执写入后无法读取"))?;
        audit_publication_on(&tx, &stored)?;
        tx.commit()?;
        Ok(stored.into_receipt(false))
    }

    pub(crate) fn compute_offer_publication(
        &self,
        offer_id: &str,
    ) -> Result<Option<ComputeOfferPublicationReceipt>> {
        validate_exact("Offer ID", offer_id, 200)?;
        let conn = self.conn()?;
        publication_by_offer_on(&conn, offer_id.trim())?
            .map(|stored| {
                audit_publication_on(&conn, &stored)?;
                Ok(stored.into_receipt(false))
            })
            .transpose()
    }
}

#[derive(Debug, Clone)]
struct StoredPublication {
    publication_id: String,
    offer_id: String,
    provider_id: String,
    pool_id: String,
    source_offer_version: i64,
    source_offer_digest: String,
    active_offer_version: i64,
    active_offer_digest: String,
    provider_policy_revision: i64,
    provider_digest: String,
    publication_digest: String,
    approved_by_user_id: String,
    published_at: String,
}

impl StoredPublication {
    fn into_receipt(self, replayed: bool) -> ComputeOfferPublicationReceipt {
        ComputeOfferPublicationReceipt {
            schema: COMPUTE_OFFER_PUBLICATION_SCHEMA,
            publication_id: self.publication_id,
            offer_id: self.offer_id,
            provider_id: self.provider_id,
            pool_id: self.pool_id,
            source_offer_version: self.source_offer_version,
            source_offer_digest: self.source_offer_digest,
            active_offer_version: self.active_offer_version,
            active_offer_digest: self.active_offer_digest,
            provider_policy_revision: self.provider_policy_revision,
            provider_digest: self.provider_digest,
            publication_digest: self.publication_digest,
            approved_by_user_id: self.approved_by_user_id,
            published_at: self.published_at,
            replayed,
            offer_effect: "active",
            price_snapshot_effect: "none",
            capacity_effect: "none",
            funds_effect: "none",
        }
    }
}

fn validate_replay(
    conn: &Connection,
    input: &PublishComputeOfferDraft,
    existing: &StoredPublication,
) -> Result<()> {
    if existing.offer_id != input.offer_id.trim()
        || existing.source_offer_version != input.expected_offer_version
        || existing.source_offer_digest != input.expected_offer_digest.trim()
    {
        bail!("相同 Offer 或发布幂等键已绑定不同草稿");
    }
    audit_publication_on(conn, existing)
}

fn audit_publication_on(conn: &Connection, stored: &StoredPublication) -> Result<()> {
    let source = registered_offer_version_on(conn, &stored.offer_id, stored.source_offer_version)?
        .ok_or_else(|| anyhow!("Offer 发布回执引用的 draft 历史版本不存在"))?;
    let active = registered_offer_version_on(conn, &stored.offer_id, stored.active_offer_version)?
        .ok_or_else(|| anyhow!("Offer 发布回执引用的 active 历史版本不存在"))?;
    let expected_digest = publication_digest(
        &stored.publication_id,
        &stored.offer_id,
        &stored.provider_id,
        &stored.pool_id,
        stored.source_offer_version,
        &stored.source_offer_digest,
        stored.active_offer_version,
        &stored.active_offer_digest,
        stored.provider_policy_revision,
        &stored.provider_digest,
        &stored.approved_by_user_id,
        &stored.published_at,
    )?;
    if source.offer.status != OFFER_STATUS_DRAFT
        || source.offer.offer_digest != stored.source_offer_digest
        || active.offer.status != OFFER_STATUS_ACTIVE
        || active.offer.offer_digest != stored.active_offer_digest
        || active.offer.provider_id != stored.provider_id
        || active.offer.capacity_pool.pool_id != stored.pool_id
        || active.provider_policy_revision != stored.provider_policy_revision
        || active.provider_digest != stored.provider_digest
        || expected_digest != stored.publication_digest
    {
        bail!("Offer 发布回执与不可变合同历史不一致");
    }
    Ok(())
}

fn publication_by_idempotency_on(
    conn: &Connection,
    scope: &str,
    key: &str,
) -> Result<Option<StoredPublication>> {
    publication_on(
        conn,
        "WHERE idempotency_scope=?1 AND idempotency_key=?2",
        params![scope, key],
    )
}

fn publication_by_offer_on(conn: &Connection, offer_id: &str) -> Result<Option<StoredPublication>> {
    publication_on(conn, "WHERE offer_id=?1", params![offer_id])
}

fn publication_on<P: rusqlite::Params>(
    conn: &Connection,
    filter: &str,
    parameters: P,
) -> Result<Option<StoredPublication>> {
    conn.query_row(
        &format!(
            "SELECT publication_id, offer_id, provider_id, pool_id,
                    source_offer_version, source_offer_digest,
                    active_offer_version, active_offer_digest,
                    provider_policy_revision, provider_digest,
                    publication_digest, approved_by_user_id, published_at
               FROM compute_offer_publications {filter}"
        ),
        parameters,
        stored_publication_from_row,
    )
    .optional()
    .map_err(Into::into)
}

fn stored_publication_from_row(row: &Row<'_>) -> rusqlite::Result<StoredPublication> {
    Ok(StoredPublication {
        publication_id: row.get(0)?,
        offer_id: row.get(1)?,
        provider_id: row.get(2)?,
        pool_id: row.get(3)?,
        source_offer_version: row.get(4)?,
        source_offer_digest: row.get(5)?,
        active_offer_version: row.get(6)?,
        active_offer_digest: row.get(7)?,
        provider_policy_revision: row.get(8)?,
        provider_digest: row.get(9)?,
        publication_digest: row.get(10)?,
        approved_by_user_id: row.get(11)?,
        published_at: row.get(12)?,
    })
}

#[allow(clippy::too_many_arguments)]
fn publication_digest(
    publication_id: &str,
    offer_id: &str,
    provider_id: &str,
    pool_id: &str,
    source_offer_version: i64,
    source_offer_digest: &str,
    active_offer_version: i64,
    active_offer_digest: &str,
    provider_policy_revision: i64,
    provider_digest: &str,
    approved_by_user_id: &str,
    published_at: &str,
) -> Result<String> {
    let value = serde_json::json!({
        "schema":COMPUTE_OFFER_PUBLICATION_SCHEMA,
        "publication_id":publication_id,
        "offer_id":offer_id,
        "provider_id":provider_id,
        "pool_id":pool_id,
        "source_offer_version":source_offer_version,
        "source_offer_digest":source_offer_digest,
        "active_offer_version":active_offer_version,
        "active_offer_digest":active_offer_digest,
        "provider_policy_revision":provider_policy_revision,
        "provider_digest":provider_digest,
        "approved_by_user_id":approved_by_user_id,
        "published_at":published_at,
    });
    Ok(hex::encode(Sha256::digest(serde_json::to_vec(&value)?)))
}

fn validate_input(input: &PublishComputeOfferDraft) -> Result<()> {
    for (label, value, max_len) in [
        ("Offer ID", input.offer_id.as_str(), 200),
        ("Offer 发布幂等范围", input.idempotency_scope.as_str(), 240),
        ("Offer 发布幂等键", input.idempotency_key.as_str(), 160),
        ("Offer 发布审批人", input.approved_by_user_id.as_str(), 160),
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
