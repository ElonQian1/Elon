use anyhow::{bail, Result};
use chrono::{DateTime, Utc};

use crate::node_compute_sharing::endpoint_authority::{
    NodeEndpointCredentialBinding, PreparedNodeEndpointCredentialRevocation,
    PreparedNodeEndpointCredentialVersion,
};

use super::super::{root, rows};

pub(super) fn require_exact_revocation(
    connection: &rusqlite::Connection,
    prepared: &PreparedNodeEndpointCredentialRevocation,
) -> Result<rows::StoredCredentialRevocation> {
    let envelope = prepared.envelope();
    let stored = rows::revocation_for_version_on(
        connection,
        envelope.credential_id(),
        envelope.credential_revision(),
    )?
    .ok_or_else(|| anyhow::anyhow!("NODE_ENDPOINT_CREDENTIAL_REVOCATION_READBACK_MISSING"))?;
    stored.ensure_exact(prepared)?;
    Ok(stored)
}

pub(super) fn require_terminal_recovery_revocation(
    connection: &rusqlite::Connection,
    expected: &NodeEndpointCredentialBinding,
) -> Result<rows::StoredCredentialRevocation> {
    let stored = rows::revocation_for_version_on(
        connection,
        expected.credential_id(),
        expected.credential_revision(),
    )?
    .ok_or_else(|| anyhow::anyhow!("NODE_ENDPOINT_CREDENTIAL_RECOVERY_REVOCATION_MISSING"))?;
    let envelope = stored.envelope();
    if envelope.credential_digest() != expected.credential_digest()
        || envelope.agent_id() != expected.agent_id()
        || !matches!(
            envelope.revocation_kind(),
            "owner_revoked" | "security_revoked"
        )
    {
        bail!("NODE_ENDPOINT_CREDENTIAL_RECOVERY_REVOCATION_MISMATCH");
    }
    Ok(stored)
}

pub(super) fn require_root_for_version(
    connection: &rusqlite::Connection,
    prepared: &PreparedNodeEndpointCredentialVersion,
) -> Result<NodeEndpointCredentialBinding> {
    let current = rows::credential_root_on(connection, prepared.envelope().credential_id())?
        .ok_or_else(|| anyhow::anyhow!("NODE_ENDPOINT_CREDENTIAL_ROOT_READBACK_MISSING"))?;
    root::require_expected_on(connection, &current)?;
    Ok(current)
}

pub(super) fn ensure_current_version(
    current: &NodeEndpointCredentialBinding,
    prepared: &PreparedNodeEndpointCredentialVersion,
) -> Result<()> {
    let envelope = prepared.envelope();
    if current.credential_id() != envelope.credential_id()
        || current.agent_id() != envelope.agent_id()
        || current.owner_user_id() != envelope.owner_user_id()
        || current.install_id() != envelope.install_id()
        || current.installation_binding_digest() != envelope.installation_binding_digest()
        || current.credential_revision() != envelope.credential_revision()
        || current.credential_digest() != prepared.credential_digest()
        || current.status() != "active"
    {
        bail!("NODE_ENDPOINT_CREDENTIAL_CURRENT_READBACK_MISMATCH");
    }
    Ok(())
}

pub(super) fn parse_recorded_at(value: &str) -> Result<DateTime<Utc>> {
    Ok(DateTime::parse_from_rfc3339(value)?.with_timezone(&Utc))
}
