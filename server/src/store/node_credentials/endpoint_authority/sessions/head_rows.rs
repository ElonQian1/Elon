use anyhow::{bail, Result};
use rusqlite::{params, Connection, OptionalExtension, Row, Transaction};

use crate::node_compute_sharing::endpoint_authority::{
    NodeEndpointSessionBinding, NodeEndpointSessionHeadSnapshot,
    PreparedNodeEndpointSessionAuthentication,
};

pub(super) fn head_by_agent_on(
    connection: &Connection,
    agent_id: &str,
) -> Result<Option<NodeEndpointSessionHeadSnapshot>> {
    connection
        .query_row(
            "SELECT agent_id, credential_id, credential_revision, credential_digest,
                    authentication_receipt_id, authentication_digest, session_id,
                    session_generation, server_instance_id, state, authenticated_at,
                    expires_at, created_at, updated_at, closed_at, close_reason_code
               FROM node_endpoint_session_heads WHERE agent_id=?1",
            params![agent_id],
            map_head,
        )
        .optional()?
        .map(validate_raw_head)
        .transpose()
}

pub(super) fn all_heads_on(
    connection: &Connection,
) -> Result<Vec<NodeEndpointSessionHeadSnapshot>> {
    let mut statement = connection.prepare(
        "SELECT agent_id, credential_id, credential_revision, credential_digest,
                authentication_receipt_id, authentication_digest, session_id,
                session_generation, server_instance_id, state, authenticated_at,
                expires_at, created_at, updated_at, closed_at, close_reason_code
           FROM node_endpoint_session_heads ORDER BY agent_id",
    )?;
    let rows = statement.query_map([], map_head)?;
    let mut snapshots = Vec::new();
    for row in rows {
        snapshots.push(validate_raw_head(row?)?);
    }
    Ok(snapshots)
}

pub(super) fn insert_or_replace_head_on(
    transaction: &Transaction<'_>,
    prepared: &PreparedNodeEndpointSessionAuthentication,
    predecessor: Option<&NodeEndpointSessionHeadSnapshot>,
) -> Result<NodeEndpointSessionHeadSnapshot> {
    let envelope = prepared.envelope();
    match predecessor {
        None => {
            transaction.execute(
                "INSERT INTO node_endpoint_session_heads (
                    agent_id, credential_id, credential_revision, credential_digest,
                    authentication_receipt_id, authentication_digest, session_id,
                    session_generation, server_instance_id, state, authenticated_at,
                    expires_at, created_at, updated_at, closed_at, close_reason_code
                 ) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,'active',?10,?11,?12,?12,NULL,NULL)",
                params![
                    envelope.agent_id(),
                    envelope.credential_id(),
                    envelope.credential_revision(),
                    envelope.credential_digest(),
                    envelope.authentication_receipt_id(),
                    prepared.authentication_digest(),
                    envelope.session_id(),
                    envelope.session_generation(),
                    envelope.server_instance_id(),
                    envelope.authenticated_at(),
                    envelope.expires_at(),
                    envelope.recorded_at(),
                ],
            )?;
        }
        Some(previous) => {
            let binding = previous.binding();
            let updated = transaction.execute(
                "UPDATE node_endpoint_session_heads
                    SET credential_id=?2, credential_revision=?3, credential_digest=?4,
                        authentication_receipt_id=?5, authentication_digest=?6,
                        session_id=?7, session_generation=?8, server_instance_id=?9,
                        state='active', authenticated_at=?10, expires_at=?11,
                        created_at=?12, updated_at=?12, closed_at=NULL, close_reason_code=NULL
                  WHERE agent_id=?1 AND authentication_receipt_id=?13
                    AND authentication_digest=?14 AND session_id=?15
                    AND session_generation=?16 AND state!='active'",
                params![
                    envelope.agent_id(),
                    envelope.credential_id(),
                    envelope.credential_revision(),
                    envelope.credential_digest(),
                    envelope.authentication_receipt_id(),
                    prepared.authentication_digest(),
                    envelope.session_id(),
                    envelope.session_generation(),
                    envelope.server_instance_id(),
                    envelope.authenticated_at(),
                    envelope.expires_at(),
                    envelope.recorded_at(),
                    binding.authentication_receipt_id(),
                    binding.authentication_digest(),
                    binding.session_id(),
                    binding.session_generation(),
                ],
            )?;
            if updated != 1 {
                bail!("NODE_ENDPOINT_SESSION_HEAD_REPLACE_CAS_MISMATCH");
            }
        }
    }
    let stored = head_by_agent_on(transaction, envelope.agent_id())?
        .ok_or_else(|| anyhow::anyhow!("NODE_ENDPOINT_SESSION_HEAD_READBACK_MISSING"))?;
    if stored.state() != "active"
        || stored.binding().authentication_receipt_id() != envelope.authentication_receipt_id()
        || stored.binding().authentication_digest() != prepared.authentication_digest()
        || stored.binding().session_generation() != envelope.session_generation()
    {
        bail!("NODE_ENDPOINT_SESSION_HEAD_READBACK_MISMATCH");
    }
    Ok(stored)
}

pub(super) fn terminate_exact_head_on(
    transaction: &Transaction<'_>,
    binding: &NodeEndpointSessionBinding,
    state: &str,
    closed_at: &str,
    reason: &str,
) -> Result<NodeEndpointSessionHeadSnapshot> {
    if !matches!(
        state,
        "closed" | "stale" | "credential_rotated" | "credential_revoked"
    ) {
        bail!("NODE_ENDPOINT_SESSION_TERMINAL_STATE_INVALID");
    }
    let updated = transaction.execute(
        "UPDATE node_endpoint_session_heads
            SET state=?10, updated_at=?11, closed_at=?11, close_reason_code=?12
          WHERE agent_id=?1 AND credential_id=?2 AND credential_revision=?3
            AND credential_digest=?4 AND authentication_receipt_id=?5
            AND authentication_digest=?6 AND session_id=?7 AND session_generation=?8
            AND server_instance_id=?9 AND state='active'",
        params![
            binding.agent_id(),
            binding.credential_id(),
            binding.credential_revision(),
            binding.credential_digest(),
            binding.authentication_receipt_id(),
            binding.authentication_digest(),
            binding.session_id(),
            binding.session_generation(),
            binding.server_instance_id(),
            state,
            closed_at,
            reason,
        ],
    )?;
    if updated != 1 {
        bail!("NODE_ENDPOINT_SESSION_HEAD_TERMINATE_CAS_MISMATCH");
    }
    let stored = head_by_agent_on(transaction, binding.agent_id())?
        .ok_or_else(|| anyhow::anyhow!("NODE_ENDPOINT_SESSION_HEAD_READBACK_MISSING"))?;
    if stored.binding() != binding
        || stored.state() != state
        || stored.closed_at() != Some(closed_at)
        || stored.close_reason_code() != Some(reason)
    {
        bail!("NODE_ENDPOINT_SESSION_TERMINATION_READBACK_MISMATCH");
    }
    Ok(stored)
}

struct RawHead {
    agent_id: String,
    credential_id: String,
    credential_revision: i64,
    credential_digest: String,
    authentication_receipt_id: String,
    authentication_digest: String,
    session_id: String,
    session_generation: i64,
    server_instance_id: String,
    state: String,
    authenticated_at: String,
    expires_at: String,
    created_at: String,
    updated_at: String,
    closed_at: Option<String>,
    close_reason_code: Option<String>,
}

fn map_head(row: &Row<'_>) -> rusqlite::Result<RawHead> {
    Ok(RawHead {
        agent_id: row.get(0)?,
        credential_id: row.get(1)?,
        credential_revision: row.get(2)?,
        credential_digest: row.get(3)?,
        authentication_receipt_id: row.get(4)?,
        authentication_digest: row.get(5)?,
        session_id: row.get(6)?,
        session_generation: row.get(7)?,
        server_instance_id: row.get(8)?,
        state: row.get(9)?,
        authenticated_at: row.get(10)?,
        expires_at: row.get(11)?,
        created_at: row.get(12)?,
        updated_at: row.get(13)?,
        closed_at: row.get(14)?,
        close_reason_code: row.get(15)?,
    })
}

fn validate_raw_head(raw: RawHead) -> Result<NodeEndpointSessionHeadSnapshot> {
    let binding = NodeEndpointSessionBinding::from_store_readback(
        raw.agent_id,
        raw.credential_id,
        u64::try_from(raw.credential_revision)?,
        raw.credential_digest,
        raw.authentication_receipt_id,
        raw.authentication_digest,
        raw.session_id,
        u64::try_from(raw.session_generation)?,
        raw.server_instance_id,
    )?;
    NodeEndpointSessionHeadSnapshot::from_store_readback(
        binding,
        raw.state,
        raw.authenticated_at,
        raw.expires_at,
        raw.created_at,
        raw.updated_at,
        raw.closed_at,
        raw.close_reason_code,
    )
}
