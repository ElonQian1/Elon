use anyhow::{anyhow, bail, Result};
use rusqlite::{params, Connection, OptionalExtension};

use crate::compute_federation::delivery_allocation::{
    ComputeDeliveryAllocationGrant, ComputeDeliveryAllocationTerminalReceipt,
    DELIVERY_ALLOCATION_STATUS_DECLINED, DELIVERY_ALLOCATION_STATUS_EXERCISED,
    DELIVERY_ALLOCATION_STATUS_EXPIRED, DELIVERY_ALLOCATION_STATUS_GRANTED,
};

use super::{
    super::{compute_capacity_commitments::audited_capacity_commitment_source_on, Store},
    canonical::{canonical_grant_json_and_digest, canonical_terminal_json_and_digest},
    types::{
        ComputeDeliveryAllocationDetail, ComputeDeliveryAllocationGrantWriteReceipt,
        DeliveryAllocationCommitmentState, DeliveryAllocationCommitmentStatus,
        DeliveryAllocationReservationAuthority,
    },
    validation::validate_exact,
};

mod audit;
mod ledger;

use audit::{
    audit_exercise_consumers_on, audit_grant_dependencies_on, audit_grant_indexes_on,
    audit_terminal_indexes_on, reservation_authority_from_terminal_on,
    validate_non_exercise_terminal,
};

impl Store {
    pub(crate) fn delivery_allocation_grant_for_provider(
        &self,
        owner_account_id: &str,
        provider_id: &str,
        pool_id: &str,
        commitment_id: &str,
    ) -> Result<Option<ComputeDeliveryAllocationDetail>> {
        for (label, value, max) in [
            ("owner account ID", owner_account_id, 200),
            ("Provider ID", provider_id, 160),
            ("Pool ID", pool_id, 200),
            ("Commitment ID", commitment_id, 200),
        ] {
            validate_exact(label, value, max)?;
        }
        let conn = self.conn()?;
        let Some(grant) = grant_by_commitment_on(&conn, commitment_id)? else {
            return Ok(None);
        };
        let (commitment, _) = audited_capacity_commitment_source_on(&conn, commitment_id)?
            .ok_or_else(|| anyhow!("DeliveryAllocation Provider read 缺少 Commitment"))?;
        if grant.provider_owner_account_id != owner_account_id
            || commitment.commitment.provider.provider_id != provider_id
            || commitment.commitment.pool.pool_id != pool_id
        {
            return Ok(None);
        }
        detail_on(&conn, grant).map(Some)
    }

    pub(crate) fn delivery_allocation_grant_for_consumer(
        &self,
        consumer_account_id: &str,
        grant_id: &str,
    ) -> Result<Option<ComputeDeliveryAllocationDetail>> {
        validate_exact("consumer account ID", consumer_account_id, 200)?;
        validate_exact("Grant ID", grant_id, 200)?;
        let conn = self.conn()?;
        let Some(grant) = grant_by_id_on(&conn, grant_id)? else {
            return Ok(None);
        };
        if grant.consumer_account_id != consumer_account_id {
            return Ok(None);
        }
        detail_on(&conn, grant).map(Some)
    }

    pub(crate) fn list_compute_delivery_allocation_grants_for_consumer(
        &self,
        consumer_account_id: &str,
        status: Option<&str>,
        limit: usize,
    ) -> Result<Vec<ComputeDeliveryAllocationDetail>> {
        validate_exact("consumer account ID", consumer_account_id, 200)?;
        if !(1..=100).contains(&limit) {
            bail!("DeliveryAllocation list limit 必须在 1 到 100 之间");
        }
        if status.is_some_and(|value| {
            !matches!(
                value,
                DELIVERY_ALLOCATION_STATUS_GRANTED
                    | DELIVERY_ALLOCATION_STATUS_EXERCISED
                    | DELIVERY_ALLOCATION_STATUS_DECLINED
                    | DELIVERY_ALLOCATION_STATUS_EXPIRED
            )
        }) {
            bail!("DeliveryAllocation list status 不受支持");
        }
        let conn = self.conn()?;
        let mut statement = conn.prepare(
            "SELECT grant.grant_id
               FROM compute_delivery_allocation_grants grant
               LEFT JOIN compute_delivery_allocation_terminal_receipts terminal
                 ON terminal.grant_id=grant.grant_id
              WHERE grant.consumer_account_id=?1
                AND (?2 IS NULL OR COALESCE(terminal.terminal_status,'granted')=?2)
              ORDER BY grant.created_at DESC, grant.grant_id
              LIMIT ?3",
        )?;
        let ids = statement
            .query_map(params![consumer_account_id, status, limit as i64], |row| {
                row.get::<_, String>(0)
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        drop(statement);
        ids.into_iter()
            .map(|id| {
                let grant = grant_by_id_on(&conn, &id)?
                    .ok_or_else(|| anyhow!("DeliveryAllocation list 项在审计时消失"))?;
                detail_on(&conn, grant)
            })
            .collect()
    }
}

pub(super) fn grant_by_id_on(
    conn: &Connection,
    grant_id: &str,
) -> Result<Option<ComputeDeliveryAllocationGrant>> {
    let stored = conn
        .query_row(
            "SELECT grant_json, grant_digest FROM compute_delivery_allocation_grants
              WHERE grant_id=?1",
            params![grant_id],
            |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
        )
        .optional()?;
    let Some((json, indexed_digest)) = stored else {
        return Ok(None);
    };
    let grant: ComputeDeliveryAllocationGrant = serde_json::from_str(&json)
        .map_err(|error| anyhow!("DeliveryAllocation Grant JSON 无效: {error}"))?;
    let (canonical, digest) = canonical_grant_json_and_digest(&grant)?;
    if grant.grant_id != grant_id
        || grant.grant_digest != indexed_digest
        || digest != indexed_digest
        || canonical != json
    {
        bail!("DeliveryAllocation Grant JSON、身份或摘要审计失败");
    }
    audit_grant_indexes_on(conn, &grant)?;
    audit_grant_dependencies_on(conn, &grant)?;
    Ok(Some(grant))
}

pub(super) fn grant_by_idempotency_on(
    conn: &Connection,
    scope: &str,
    key: &str,
) -> Result<Option<ComputeDeliveryAllocationGrant>> {
    grant_id_for_query_on(
        conn,
        "SELECT grant_id FROM compute_delivery_allocation_grants
          WHERE idempotency_scope=?1 AND idempotency_key=?2",
        scope,
        key,
    )
}

pub(super) fn grant_by_commitment_on(
    conn: &Connection,
    commitment_id: &str,
) -> Result<Option<ComputeDeliveryAllocationGrant>> {
    let id = conn
        .query_row(
            "SELECT grant_id FROM compute_delivery_allocation_grants WHERE commitment_id=?1",
            params![commitment_id],
            |row| row.get::<_, String>(0),
        )
        .optional()?;
    id.map(|value| grant_by_id_on(conn, &value))
        .transpose()
        .map(Option::flatten)
}

pub(super) fn grant_by_job_on(
    conn: &Connection,
    job_id: &str,
) -> Result<Option<ComputeDeliveryAllocationGrant>> {
    let id = conn
        .query_row(
            "SELECT grant_id FROM compute_delivery_allocation_grants WHERE job_id=?1",
            params![job_id],
            |row| row.get::<_, String>(0),
        )
        .optional()?;
    id.map(|value| grant_by_id_on(conn, &value))
        .transpose()
        .map(Option::flatten)
}

pub(super) fn raw_terminal_by_grant_on(
    conn: &Connection,
    grant: &ComputeDeliveryAllocationGrant,
) -> Result<Option<ComputeDeliveryAllocationTerminalReceipt>> {
    let stored = conn
        .query_row(
            "SELECT terminal_receipt_json, terminal_receipt_digest
               FROM compute_delivery_allocation_terminal_receipts WHERE grant_id=?1",
            params![grant.grant_id],
            |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
        )
        .optional()?;
    let Some((json, indexed_digest)) = stored else {
        return Ok(None);
    };
    let terminal: ComputeDeliveryAllocationTerminalReceipt = serde_json::from_str(&json)
        .map_err(|error| anyhow!("DeliveryAllocation terminal JSON 无效: {error}"))?;
    let (canonical, digest) = canonical_terminal_json_and_digest(&terminal)?;
    if terminal.terminal_receipt_digest != indexed_digest
        || digest != indexed_digest
        || canonical != json
        || terminal.grant_id != grant.grant_id
        || terminal.grant_digest != grant.grant_digest
        || terminal.commitment != grant.commitment
    {
        bail!("DeliveryAllocation terminal JSON、身份或摘要审计失败");
    }
    audit_terminal_indexes_on(conn, &terminal)?;
    Ok(Some(terminal))
}

pub(super) fn terminal_by_grant_on(
    conn: &Connection,
    grant: &ComputeDeliveryAllocationGrant,
) -> Result<Option<ComputeDeliveryAllocationTerminalReceipt>> {
    let Some(terminal) = raw_terminal_by_grant_on(conn, grant)? else {
        return Ok(None);
    };
    if terminal.terminal_status == DELIVERY_ALLOCATION_STATUS_EXERCISED {
        audit_exercise_consumers_on(conn, grant, &terminal)?;
    } else {
        validate_non_exercise_terminal(grant, &terminal)?;
    }
    Ok(Some(terminal))
}

pub(super) fn terminal_by_idempotency_on(
    conn: &Connection,
    scope: &str,
    key: &str,
) -> Result<
    Option<(
        ComputeDeliveryAllocationGrant,
        ComputeDeliveryAllocationTerminalReceipt,
    )>,
> {
    let grant_id = conn
        .query_row(
            "SELECT grant_id FROM compute_delivery_allocation_terminal_receipts
              WHERE idempotency_scope=?1 AND idempotency_key=?2",
            params![scope, key],
            |row| row.get::<_, String>(0),
        )
        .optional()?;
    let Some(grant_id) = grant_id else {
        return Ok(None);
    };
    let grant = grant_by_id_on(conn, &grant_id)?
        .ok_or_else(|| anyhow!("DeliveryAllocation terminal replay 缺少 Grant"))?;
    let terminal = terminal_by_grant_on(conn, &grant)?
        .ok_or_else(|| anyhow!("DeliveryAllocation terminal replay 缺少 receipt"))?;
    Ok(Some((grant, terminal)))
}

pub(super) fn grant_receipt_on(
    conn: &Connection,
    grant: ComputeDeliveryAllocationGrant,
    replayed: bool,
) -> Result<ComputeDeliveryAllocationGrantWriteReceipt> {
    audit_grant_dependencies_on(conn, &grant)?;
    Ok(ComputeDeliveryAllocationGrantWriteReceipt { grant, replayed })
}

pub(super) fn detail_on(
    conn: &Connection,
    grant: ComputeDeliveryAllocationGrant,
) -> Result<ComputeDeliveryAllocationDetail> {
    let terminal_receipt = terminal_by_grant_on(conn, &grant)?;
    let current_status = terminal_receipt
        .as_ref()
        .map(|value| value.terminal_status.clone())
        .unwrap_or_else(|| DELIVERY_ALLOCATION_STATUS_GRANTED.to_string());
    Ok(ComputeDeliveryAllocationDetail {
        grant,
        terminal_receipt,
        current_status,
    })
}

pub(super) fn due_grant_ids_on(
    conn: &Connection,
    recorded_at: &str,
    limit: usize,
) -> Result<Vec<String>> {
    let mut statement = conn.prepare(
        "SELECT grant.grant_id FROM compute_delivery_allocation_grants grant
           LEFT JOIN compute_delivery_allocation_terminal_receipts terminal
             ON terminal.grant_id=grant.grant_id
          WHERE terminal.grant_id IS NULL
            AND julianday(grant.exercise_expires_at)<=julianday(?1)
          ORDER BY grant.exercise_expires_at, grant.grant_id LIMIT ?2",
    )?;
    Ok(statement
        .query_map(params![recorded_at, limit as i64], |row| row.get(0))?
        .collect::<rusqlite::Result<Vec<_>>>()?)
}

pub(in crate::store) fn delivery_allocation_commitment_status_on(
    conn: &Connection,
    commitment_id: &str,
    commitment_digest: &str,
) -> Result<Option<DeliveryAllocationCommitmentStatus>> {
    let Some(grant) = grant_by_commitment_on(conn, commitment_id)? else {
        return Ok(None);
    };
    if grant.commitment.commitment_digest != commitment_digest {
        bail!("DeliveryAllocation Commitment status lookup digest 不一致");
    }
    let terminal = raw_terminal_by_grant_on(conn, &grant)?;
    let state = match terminal.as_ref() {
        None => DeliveryAllocationCommitmentState::Granted,
        Some(value) if value.terminal_status == DELIVERY_ALLOCATION_STATUS_EXERCISED => {
            let _ = reservation_authority_from_terminal_on(conn, &grant, value)?;
            DeliveryAllocationCommitmentState::Exercised
        }
        Some(value) if value.terminal_status == DELIVERY_ALLOCATION_STATUS_DECLINED => {
            DeliveryAllocationCommitmentState::Declined
        }
        Some(value) if value.terminal_status == DELIVERY_ALLOCATION_STATUS_EXPIRED => {
            DeliveryAllocationCommitmentState::Expired
        }
        Some(_) => bail!("DeliveryAllocation terminal status 不受支持"),
    };
    if let Some(terminal) = terminal.as_ref() {
        if terminal.terminal_status != DELIVERY_ALLOCATION_STATUS_EXERCISED {
            validate_non_exercise_terminal(&grant, terminal)?;
        }
    }
    Ok(Some(DeliveryAllocationCommitmentStatus {
        grant_id: grant.grant_id,
        grant_digest: grant.grant_digest,
        state,
        terminal_receipt_id: terminal
            .as_ref()
            .map(|value| value.terminal_receipt_id.clone()),
        terminal_receipt_digest: terminal
            .as_ref()
            .map(|value| value.terminal_receipt_digest.clone()),
    }))
}

pub(in crate::store) fn persisted_delivery_allocation_reservation_authority_on(
    conn: &Connection,
    reservation_id: &str,
    claim_id: &str,
) -> Result<Option<DeliveryAllocationReservationAuthority>> {
    let grant_id = conn
        .query_row(
            "SELECT grant_id FROM compute_delivery_allocation_terminal_receipts
              WHERE terminal_status='exercised' AND reservation_id=?1 AND reservation_claim_id=?2",
            params![reservation_id, claim_id],
            |row| row.get::<_, String>(0),
        )
        .optional()?;
    let Some(grant_id) = grant_id else {
        return Ok(None);
    };
    let grant = grant_by_id_on(conn, &grant_id)?
        .ok_or_else(|| anyhow!("persisted DeliveryAllocation authority 缺少 Grant"))?;
    let terminal = raw_terminal_by_grant_on(conn, &grant)?
        .ok_or_else(|| anyhow!("persisted DeliveryAllocation authority 缺少 terminal"))?;
    reservation_authority_from_terminal_on(conn, &grant, &terminal).map(Some)
}

fn grant_id_for_query_on(
    conn: &Connection,
    sql: &str,
    left: &str,
    right: &str,
) -> Result<Option<ComputeDeliveryAllocationGrant>> {
    let id = conn
        .query_row(sql, params![left, right], |row| row.get::<_, String>(0))
        .optional()?;
    id.map(|value| grant_by_id_on(conn, &value))
        .transpose()
        .map(Option::flatten)
}
