use anyhow::{anyhow, bail, Context, Result};
use rusqlite::{params, Connection, OptionalExtension};
use sha2::{Digest, Sha256};

use crate::compute_federation::capacity::{
    validate_capacity_claim, ComputeCapacityClaim, ComputeCapacityClaimKind,
    ComputeCapacityClaimLine, ComputeCapacityClaimState, COMPUTE_CAPACITY_CLAIM_SCHEMA,
};

use super::{compute_capacity_rows::stored_bucket_on, now};

struct ClaimHeader {
    claim_id: String,
    claim_digest: String,
    pool_id: String,
    capacity_epoch: i64,
    delivery_window_id: String,
    claim_kind: String,
    subject_kind: String,
    subject_id: String,
    state: String,
    revision: i64,
    parent_claim_id: Option<String>,
    idempotency_scope: String,
    idempotency_key: String,
    request_digest: String,
    created_at: String,
    updated_at: String,
    expires_at: Option<String>,
    terminal_at: Option<String>,
}

pub(super) fn stored_claim_on(
    conn: &Connection,
    claim_id: &str,
) -> Result<Option<ComputeCapacityClaim>> {
    let header = conn
        .query_row(
            "SELECT claim_id, claim_digest, pool_id, capacity_epoch,
                    delivery_window_id, claim_kind, subject_kind, subject_id,
                    status, revision, parent_claim_id, idempotency_scope,
                    idempotency_key, request_digest, created_at, updated_at,
                    expires_at, terminal_at
               FROM compute_capacity_claims
              WHERE claim_id=?1",
            params![claim_id.trim()],
            |row| {
                Ok(ClaimHeader {
                    claim_id: row.get(0)?,
                    claim_digest: row.get(1)?,
                    pool_id: row.get(2)?,
                    capacity_epoch: row.get(3)?,
                    delivery_window_id: row.get(4)?,
                    claim_kind: row.get(5)?,
                    subject_kind: row.get(6)?,
                    subject_id: row.get(7)?,
                    state: row.get(8)?,
                    revision: row.get(9)?,
                    parent_claim_id: row.get(10)?,
                    idempotency_scope: row.get(11)?,
                    idempotency_key: row.get(12)?,
                    request_digest: row.get(13)?,
                    created_at: row.get(14)?,
                    updated_at: row.get(15)?,
                    expires_at: row.get(16)?,
                    terminal_at: row.get(17)?,
                })
            },
        )
        .optional()?;
    let Some(header) = header else {
        return Ok(None);
    };

    let mut statement = conn.prepare(
        "SELECT line_no, bucket_id, meter, quantity_units
           FROM compute_capacity_claim_lines
          WHERE claim_id=?1 ORDER BY line_no",
    )?;
    let raw_lines = statement
        .query_map(params![header.claim_id], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, i64>(3)?,
            ))
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    if raw_lines.is_empty() {
        bail!("容量 Claim 缺少不可变资源行");
    }

    let mut lines = Vec::with_capacity(raw_lines.len());
    for (line_no, bucket_id, meter, quantity_units) in raw_lines {
        let stored = stored_bucket_on(conn, &bucket_id)?
            .ok_or_else(|| anyhow!("容量 Claim 引用的 bucket {bucket_id} 不存在"))?;
        if stored.balance.binding.meter != meter
            || stored.balance.binding.pool.pool_id != header.pool_id
            || stored.balance.binding.pool.capacity_epoch != header.capacity_epoch
            || stored.balance.binding.delivery_window.window_id != header.delivery_window_id
        {
            bail!("容量 Claim 的资源行与 bucket 绑定不一致");
        }
        lines.push(ComputeCapacityClaimLine {
            line_no,
            bucket: stored.balance.binding,
            quantity_units,
        });
    }
    let first = lines
        .first()
        .ok_or_else(|| anyhow!("容量 Claim 缺少资源绑定"))?;
    let mut claim = ComputeCapacityClaim {
        schema: COMPUTE_CAPACITY_CLAIM_SCHEMA.to_string(),
        claim_id: header.claim_id,
        claim_digest: header.claim_digest,
        pool: first.bucket.pool.clone(),
        delivery_window: first.bucket.delivery_window.clone(),
        claim_kind: parse_claim_kind(&header.claim_kind)?,
        state: parse_claim_state(&header.state)?,
        revision: header.revision,
        parent_claim_id: header.parent_claim_id,
        subject_kind: header.subject_kind,
        subject_id: header.subject_id,
        idempotency_scope: header.idempotency_scope,
        idempotency_key: header.idempotency_key,
        request_digest: header.request_digest,
        lines,
        created_at: header.created_at,
        updated_at: header.updated_at,
        expires_at: header.expires_at,
        terminal_at: header.terminal_at,
    };
    validate_capacity_claim(&claim).map_err(|error| anyhow!("容量 Claim 合同无效: {error:?}"))?;
    let stored_digest = claim.claim_digest.clone();
    finalize_claim_digest(&mut claim)?;
    if claim.claim_digest != stored_digest {
        bail!("容量 Claim 摘要与当前状态不一致");
    }
    Ok(Some(claim))
}

pub(super) fn insert_claim_on(conn: &Connection, claim: &ComputeCapacityClaim) -> Result<()> {
    conn.execute(
        "INSERT INTO compute_capacity_claims (
            claim_id, claim_digest, pool_id, capacity_epoch,
            delivery_window_id, claim_kind, subject_kind, subject_id,
            status, revision, parent_claim_id, idempotency_scope,
            idempotency_key, request_digest, created_at, updated_at,
            expires_at, terminal_at
         ) VALUES (
            ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11,
            ?12, ?13, ?14, ?15, ?16, ?17, ?18
         )",
        params![
            claim.claim_id,
            claim.claim_digest,
            claim.pool.pool_id,
            claim.pool.capacity_epoch,
            claim.delivery_window.window_id,
            claim_kind_value(claim.claim_kind),
            claim.subject_kind,
            claim.subject_id,
            claim_state_value(claim.state),
            claim.revision,
            claim.parent_claim_id,
            claim.idempotency_scope,
            claim.idempotency_key,
            claim.request_digest,
            claim.created_at,
            claim.updated_at,
            claim.expires_at,
            claim.terminal_at,
        ],
    )?;
    for line in &claim.lines {
        conn.execute(
            "INSERT INTO compute_capacity_claim_lines (
                claim_id, line_no, bucket_id, meter, quantity_units, created_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                claim.claim_id,
                line.line_no,
                line.bucket.bucket_id,
                line.bucket.meter,
                line.quantity_units,
                claim.created_at,
            ],
        )?;
    }
    insert_claim_version_on(conn, claim)?;
    Ok(())
}

pub(super) fn update_claim_projection_on(
    conn: &Connection,
    previous_revision: i64,
    previous_state: ComputeCapacityClaimState,
    claim: &ComputeCapacityClaim,
) -> Result<()> {
    let changed = conn.execute(
        "UPDATE compute_capacity_claims SET
            claim_digest=?1, status=?2, revision=?3, updated_at=?4, terminal_at=?5
          WHERE claim_id=?6 AND revision=?7 AND status=?8",
        params![
            claim.claim_digest,
            claim_state_value(claim.state),
            claim.revision,
            claim.updated_at,
            claim.terminal_at,
            claim.claim_id,
            previous_revision,
            claim_state_value(previous_state),
        ],
    )?;
    if changed != 1 {
        bail!("容量 Claim revision 或状态已变化，事务未提交");
    }
    insert_claim_version_on(conn, claim)?;
    Ok(())
}

pub(super) fn stored_claim_version_on(
    conn: &Connection,
    claim_id: &str,
    revision: i64,
) -> Result<Option<ComputeCapacityClaim>> {
    let normalized_claim_id = claim_id.trim();
    let stored = conn
        .query_row(
            "SELECT claim_digest, status, request_digest, claim_json
               FROM compute_capacity_claim_versions
              WHERE claim_id=?1 AND revision=?2",
            params![normalized_claim_id, revision],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                ))
            },
        )
        .optional()?;
    let Some((claim_digest, status, request_digest, claim_json)) = stored else {
        return Ok(None);
    };
    let claim: ComputeCapacityClaim =
        serde_json::from_str(&claim_json).context("容量 Claim 历史版本 JSON 无效")?;
    validate_capacity_claim(&claim)
        .map_err(|error| anyhow!("容量 Claim 历史版本合同无效: {error:?}"))?;
    let mut recomputed = claim.clone();
    finalize_claim_digest(&mut recomputed)?;
    if claim.claim_id != normalized_claim_id
        || claim.revision != revision
        || claim.claim_digest != claim_digest
        || recomputed.claim_digest != claim_digest
        || claim_state_value(claim.state) != status
        || claim.request_digest != request_digest
    {
        bail!("容量 Claim 历史版本身份、摘要或索引字段审计失败");
    }
    Ok(Some(claim))
}

fn insert_claim_version_on(conn: &Connection, claim: &ComputeCapacityClaim) -> Result<()> {
    let claim_json = serde_json::to_string(claim)?;
    conn.execute(
        "INSERT INTO compute_capacity_claim_versions (
            claim_id, revision, claim_digest, status, request_digest,
            claim_json, recorded_at
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![
            claim.claim_id,
            claim.revision,
            claim.claim_digest,
            claim_state_value(claim.state),
            claim.request_digest,
            claim_json,
            now(),
        ],
    )?;
    Ok(())
}

pub(super) fn finalize_claim_digest(claim: &mut ComputeCapacityClaim) -> Result<()> {
    let payload = serde_json::json!({
        "schema": claim.schema,
        "claim_id": claim.claim_id,
        "pool": claim.pool,
        "delivery_window": claim.delivery_window,
        "claim_kind": claim.claim_kind,
        "state": claim.state,
        "revision": claim.revision,
        "parent_claim_id": claim.parent_claim_id,
        "subject_kind": claim.subject_kind,
        "subject_id": claim.subject_id,
        "idempotency_scope": claim.idempotency_scope,
        "idempotency_key": claim.idempotency_key,
        "request_digest": claim.request_digest,
        "lines": claim.lines,
        "created_at": claim.created_at,
        "updated_at": claim.updated_at,
        "expires_at": claim.expires_at,
        "terminal_at": claim.terminal_at,
    });
    claim.claim_digest = hex::encode(Sha256::digest(serde_json::to_vec(&payload)?));
    Ok(())
}

pub(super) fn claim_kind_value(kind: ComputeCapacityClaimKind) -> &'static str {
    match kind {
        ComputeCapacityClaimKind::QuoteHold => "quote_hold",
        ComputeCapacityClaimKind::Reservation => "reservation",
        ComputeCapacityClaimKind::CapacityCommitment => "capacity_commitment",
        ComputeCapacityClaimKind::DeliveryAllocation => "delivery_allocation",
        ComputeCapacityClaimKind::Attempt => "attempt",
    }
}

pub(super) fn claim_state_value(state: ComputeCapacityClaimState) -> &'static str {
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

fn parse_claim_kind(value: &str) -> Result<ComputeCapacityClaimKind> {
    match value {
        "quote_hold" => Ok(ComputeCapacityClaimKind::QuoteHold),
        "reservation" => Ok(ComputeCapacityClaimKind::Reservation),
        "capacity_commitment" => Ok(ComputeCapacityClaimKind::CapacityCommitment),
        "delivery_allocation" => Ok(ComputeCapacityClaimKind::DeliveryAllocation),
        "attempt" => Ok(ComputeCapacityClaimKind::Attempt),
        _ => bail!("容量 Claim kind 无效"),
    }
}

fn parse_claim_state(value: &str) -> Result<ComputeCapacityClaimState> {
    match value {
        "pending" => Ok(ComputeCapacityClaimState::Pending),
        "held" => Ok(ComputeCapacityClaimState::Held),
        "active" => Ok(ComputeCapacityClaimState::Active),
        "consumed" => Ok(ComputeCapacityClaimState::Consumed),
        "released" => Ok(ComputeCapacityClaimState::Released),
        "expired" => Ok(ComputeCapacityClaimState::Expired),
        "canceled" => Ok(ComputeCapacityClaimState::Canceled),
        _ => bail!("容量 Claim status 无效"),
    }
}
