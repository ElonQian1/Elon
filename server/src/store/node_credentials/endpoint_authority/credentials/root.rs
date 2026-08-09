use anyhow::{bail, Result};
use rusqlite::{params, Connection, OptionalExtension, Transaction};

use crate::node_compute_sharing::endpoint_authority::{
    derive_node_endpoint_installation_binding_digest, NodeEndpointCredentialBinding,
    PreparedNodeEndpointCredentialVersion,
};

use super::rows::{
    credential_root_on, revocation_for_version_on, version_exact_on, version_revision_on,
};

pub(super) fn credential_root_by_agent_on(
    connection: &Connection,
    agent_id: &str,
) -> Result<Option<NodeEndpointCredentialBinding>> {
    let raw = connection
        .query_row(
            "SELECT credential_id, agent_id, owner_user_id, install_id,
                    installation_binding_digest, current_credential_revision,
                    current_credential_digest, status
               FROM node_endpoint_credentials WHERE agent_id=?1",
            params![agent_id],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, i64>(5)?,
                    row.get::<_, String>(6)?,
                    row.get::<_, String>(7)?,
                ))
            },
        )
        .optional()?;
    raw.map(|value| {
        NodeEndpointCredentialBinding::from_store_readback(
            value.0,
            value.1,
            value.2,
            value.3,
            value.4,
            u64::try_from(value.5)?,
            value.6,
            value.7,
        )
    })
    .transpose()
}

pub(super) fn require_expected_active_on(
    connection: &Connection,
    expected: &NodeEndpointCredentialBinding,
) -> Result<()> {
    let current = credential_root_on(connection, expected.credential_id())?;
    if current.as_ref() != Some(expected) || expected.status() != "active" {
        bail!("NODE_ENDPOINT_CREDENTIAL_CURRENTNESS_MISMATCH");
    }
    ensure_durable_source_on(connection, expected, true)?;
    Ok(())
}

pub(super) fn require_expected_on(
    connection: &Connection,
    expected: &NodeEndpointCredentialBinding,
) -> Result<()> {
    if credential_root_on(connection, expected.credential_id())?.as_ref() != Some(expected) {
        bail!("NODE_ENDPOINT_CREDENTIAL_CURRENTNESS_MISMATCH");
    }
    ensure_durable_source_on(connection, expected, expected.status() == "active")?;
    Ok(())
}

fn ensure_durable_source_on(
    connection: &Connection,
    expected: &NodeEndpointCredentialBinding,
    require_unrevoked: bool,
) -> Result<()> {
    // Identity-only compatibility read. Legacy secret_hash is deliberately never selected.
    let legacy = connection
        .query_row(
            "SELECT owner_user_id, install_id FROM node_credentials WHERE agent_id=?1",
            params![expected.agent_id()],
            |row| Ok((row.get::<_, String>(0)?, row.get::<_, Option<String>>(1)?)),
        )
        .optional()?;
    let valid_legacy = legacy.is_some_and(|(owner_user_id, install_id)| {
        owner_user_id == expected.owner_user_id()
            && install_id.as_deref() == Some(expected.install_id())
    });
    let derived = derive_node_endpoint_installation_binding_digest(
        expected.agent_id(),
        expected.owner_user_id(),
        expected.install_id(),
    )?;
    let version = version_exact_on(connection, expected)?
        .ok_or_else(|| anyhow::anyhow!("NODE_ENDPOINT_CREDENTIAL_VERSION_READBACK_MISSING"))?;
    version.ensure_binding(expected)?;
    let initial = version_revision_on(connection, expected.credential_id(), 1)?
        .ok_or_else(|| anyhow::anyhow!("NODE_ENDPOINT_CREDENTIAL_INITIAL_VERSION_MISSING"))?;
    let times = connection
        .query_row(
            "SELECT created_at, updated_at FROM node_endpoint_credentials
              WHERE credential_id=?1",
            params![expected.credential_id()],
            |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
        )
        .optional()?
        .ok_or_else(|| anyhow::anyhow!("NODE_ENDPOINT_CREDENTIAL_ROOT_READBACK_MISSING"))?;
    if !valid_legacy || derived != expected.installation_binding_digest() {
        bail!("NODE_ENDPOINT_CREDENTIAL_INSTALLATION_CURRENTNESS_MISMATCH");
    }
    if times.0 != initial.recorded_at() {
        bail!("NODE_ENDPOINT_CREDENTIAL_CREATED_AT_READBACK_MISMATCH");
    }
    let revocation = revocation_for_version_on(
        connection,
        expected.credential_id(),
        expected.credential_revision(),
    )?;
    if require_unrevoked {
        if revocation.is_some() || times.1 != version.recorded_at() {
            bail!("NODE_ENDPOINT_CREDENTIAL_ACTIVE_CLOSURE_MISMATCH");
        }
    } else {
        let revocation = revocation
            .ok_or_else(|| anyhow::anyhow!("NODE_ENDPOINT_CREDENTIAL_TERMINAL_MISSING"))?;
        if times.1 != revocation.recorded_at() {
            bail!("NODE_ENDPOINT_CREDENTIAL_TERMINAL_CLOSURE_MISMATCH");
        }
    }
    Ok(())
}

pub(super) fn insert_initial_root_on(
    transaction: &Transaction<'_>,
    prepared: &PreparedNodeEndpointCredentialVersion,
) -> Result<NodeEndpointCredentialBinding> {
    let envelope = prepared.envelope();
    transaction.execute(
        "INSERT INTO node_endpoint_credentials (
            credential_id, agent_id, owner_user_id, install_id,
            installation_binding_digest, current_credential_revision,
            current_credential_digest, status, created_at, updated_at
         ) VALUES (?1,?2,?3,?4,?5,?6,?7,'active',?8,?8)",
        params![
            envelope.credential_id(),
            envelope.agent_id(),
            envelope.owner_user_id(),
            envelope.install_id(),
            envelope.installation_binding_digest(),
            envelope.credential_revision(),
            prepared.credential_digest(),
            envelope.recorded_at(),
        ],
    )?;
    let stored = credential_root_on(transaction, envelope.credential_id())?
        .ok_or_else(|| anyhow::anyhow!("NODE_ENDPOINT_CREDENTIAL_ROOT_READBACK_MISSING"))?;
    let expected = NodeEndpointCredentialBinding::from_store_readback(
        envelope.credential_id().to_string(),
        envelope.agent_id().to_string(),
        envelope.owner_user_id().to_string(),
        envelope.install_id().to_string(),
        envelope.installation_binding_digest().to_string(),
        envelope.credential_revision(),
        prepared.credential_digest().to_string(),
        "active".to_string(),
    )?;
    if stored != expected {
        bail!("NODE_ENDPOINT_CREDENTIAL_ROOT_READBACK_MISMATCH");
    }
    Ok(stored)
}

pub(super) fn advance_root_on(
    transaction: &Transaction<'_>,
    expected: &NodeEndpointCredentialBinding,
    prepared: &PreparedNodeEndpointCredentialVersion,
) -> Result<NodeEndpointCredentialBinding> {
    let envelope = prepared.envelope();
    let updated = transaction.execute(
        "UPDATE node_endpoint_credentials
            SET current_credential_revision=?6, current_credential_digest=?7,
                status='active', updated_at=?8
          WHERE credential_id=?1 AND agent_id=?2 AND owner_user_id=?3 AND install_id=?4
            AND installation_binding_digest=?5
            AND current_credential_revision=?9 AND current_credential_digest=?10
            AND status=?11",
        params![
            expected.credential_id(),
            expected.agent_id(),
            expected.owner_user_id(),
            expected.install_id(),
            expected.installation_binding_digest(),
            envelope.credential_revision(),
            prepared.credential_digest(),
            envelope.recorded_at(),
            expected.credential_revision(),
            expected.credential_digest(),
            expected.status(),
        ],
    )?;
    if updated != 1 {
        bail!("NODE_ENDPOINT_CREDENTIAL_ADVANCE_CAS_MISMATCH");
    }
    credential_root_on(transaction, expected.credential_id())?
        .ok_or_else(|| anyhow::anyhow!("NODE_ENDPOINT_CREDENTIAL_ROOT_READBACK_MISSING"))
}

pub(super) fn revoke_root_on(
    transaction: &Transaction<'_>,
    expected: &NodeEndpointCredentialBinding,
    updated_at: &str,
) -> Result<NodeEndpointCredentialBinding> {
    let updated = transaction.execute(
        "UPDATE node_endpoint_credentials SET status='revoked', updated_at=?6
          WHERE credential_id=?1 AND agent_id=?2
            AND current_credential_revision=?3 AND current_credential_digest=?4
            AND status='active' AND installation_binding_digest=?5",
        params![
            expected.credential_id(),
            expected.agent_id(),
            expected.credential_revision(),
            expected.credential_digest(),
            expected.installation_binding_digest(),
            updated_at,
        ],
    )?;
    if updated != 1 {
        bail!("NODE_ENDPOINT_CREDENTIAL_REVOKE_CAS_MISMATCH");
    }
    credential_root_on(transaction, expected.credential_id())?
        .ok_or_else(|| anyhow::anyhow!("NODE_ENDPOINT_CREDENTIAL_ROOT_READBACK_MISSING"))
}
